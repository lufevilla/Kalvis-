/*
GARANTE QUE O DADO É CONFIÁVEL ANTES DE QUALQUER CÁLCULO OU ALERTA.

CHECA: JANELA, TIMESTAMPS INVÁLIDOS/DUPLICADOS, FORA DA JANELA, FALTANTES,
VALORES ABSURDOS, RUÍDO DE SENSOR (TAXA DE VARIAÇÃO), LACUNAS E QUANTIDADE.
DEVOLVE AS AMOSTRAS VÁLIDAS, ORDENADAS, PRONTAS PARA USO.

CLASSIFICAÇÃO (cada amostra entra em NO MÁXIMO UMA categoria, na ordem):

  1. Timestamp inválido     → `invalid_timestamps`
  2. Fora da janela         → `out_of_window`
  3. Timestamp duplicado    → `duplicated_timestamps`
  4. Valor ausente          → `missing_samples`
  5. Valor fora dos limites → `out_of_range_samples`
  6. Ruidosa (taxa alta)    → `noisy_samples`
  7. Caso contrário         → válida (`valid_samples`)

Todos os percentuais usam como denominador o TOTAL de amostras recebidas
(`total_samples`), então a soma das categorias + válidas = 100%.
*/

use chrono::{DateTime, Utc};

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::{AmostraBruta, Indicador, buscar_serie_bruta, parse_momento};
use super::statistics::ratio_percentage;

#[derive(Debug, Clone, Copy)]
pub struct JanelaTempo {
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DataQualityConfig {
    pub minimum_samples: usize,
    pub minimum_valid_percentage: f64,
    pub max_invalid_percentage: f64,
    pub max_noisy_percentage: f64,
    pub max_gaps: usize,
    pub max_gap_seconds: i64,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    /// Variação máxima aceitável por segundo entre amostras válidas.
    pub max_rate_per_second: Option<f64>,
}

impl DataQualityConfig {
    /// Configuração padrão POR INDICADOR.
    ///
    /// Os limites abaixo são de PLAUSIBILIDADE DE COLETA (faixa física em que
    /// um sensor/registro pode operar) e taxas máximas de variação para
    /// detectar ruído — NÃO são limites clínicos nem diagnósticos. Ajuste por
    /// indicador porque as unidades e os comportamentos diferem; usar um
    /// único limite de variação para todos é incorreto (ex.: 5 bpm/s é
    /// plausível para frequência cardíaca, mas absurdo para temperatura).
    pub fn padrao_para(indicador: Indicador) -> Self {
        let (min_value, max_value, max_rate_per_second) = match indicador {
            Indicador::FrequenciaCardiaca => (Some(20.0), Some(250.0), Some(5.0)), // bpm
            Indicador::Spo2 => (Some(50.0), Some(100.0), Some(2.0)),               // %
            Indicador::TemperaturaCorporal => (Some(30.0), Some(43.0), Some(0.5)), // °C
            Indicador::TemperaturaPele => (Some(25.0), Some(42.0), Some(1.0)),     // °C
            Indicador::TaxaRespiratoria => (Some(4.0), Some(60.0), Some(3.0)),     // rpm
            Indicador::Hrv => (Some(0.0), Some(500.0), Some(20.0)),                // ms
            Indicador::PressaoSistolica => (Some(40.0), Some(300.0), Some(10.0)),  // mmHg
            Indicador::PressaoDiastolica => (Some(20.0), Some(200.0), Some(10.0)), // mmHg
            Indicador::Glicose => (Some(20.0), Some(1000.0), Some(5.0)),           // mg/dL
            // Registro diário: faixa de horas de sono por dia; sem taxa de
            // variação (amostras diárias, não contínuas).
            Indicador::HorasSono => (Some(0.0), Some(24.0), None),
        };

        Self {
            minimum_samples: 2,
            minimum_valid_percentage: 80.0,
            max_invalid_percentage: 5.0,
            max_noisy_percentage: 5.0,
            // Por padrão não rejeita por contagem de lacunas; a lacuna fica
            // registrada no resultado (`temporal_gaps` / `largest_gap_seconds`).
            max_gaps: usize::MAX,
            // Janela de coleta contínua é de 5 s; uma lacuna de mais de 5 min
            // indica interrupção de coleta (registro diário tolera dias).
            max_gap_seconds: if indicador == Indicador::HorasSono {
                3 * 86_400
            } else {
                300
            },
            min_value,
            max_value,
            max_rate_per_second,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotivoRejeicao {
    JanelaInvalida,
    AmostrasForaDaJanela(usize),
    AmostrasInsuficientes { validas: usize, minimo: usize },
    PercentualValidoBaixo(f64),
    PercentualInvalidoAlto(f64),
    PercentualRuidoAlto(f64),
    LacunasExcessivas(usize),
}

#[derive(Debug, Clone)]
pub struct DataQualityResult {
    pub total_samples: usize,
    pub valid_samples: usize,
    pub missing_samples: usize,
    pub invalid_timestamps: usize,
    pub duplicated_timestamps: usize,
    pub out_of_window: usize,
    /// Amostras rejeitadas por VALOR fora dos limites (ou não finito).
    /// NÃO soma timestamps inválidos/duplicados — cada categoria é contada
    /// separadamente (ver o mapa de classificação no topo do arquivo).
    pub out_of_range_samples: usize,
    pub noisy_samples: usize,
    pub temporal_gaps: usize,
    /// Duração (s) da maior lacuna temporal encontrada. As amostras
    /// posteriores a uma lacuna podem continuar válidas; a interrupção
    /// fica registrada aqui e em `temporal_gaps`.
    pub largest_gap_seconds: i64,
    pub valid_percentage: f64,
    pub motivos: Vec<MotivoRejeicao>,
    pub acceptable_quality: bool,
    /// Amostras aprovadas, em ordem temporal.
    pub amostras_validas: Vec<(DateTime<Utc>, f64)>,
}

fn fora_dos_limites(v: f64, c: &DataQualityConfig) -> bool {
    !v.is_finite()
        || c.min_value.is_some_and(|min| v < min)
        || c.max_value.is_some_and(|max| v > max)
}

pub fn evaluate_data_quality(
    samples: &[AmostraBruta],
    janela: Option<JanelaTempo>,
    config: &DataQualityConfig,
) -> DataQualityResult {
    let total = samples.len();
    let mut motivos = Vec::new();

    if janela.is_some_and(|j| j.fim <= j.inicio) {
        motivos.push(MotivoRejeicao::JanelaInvalida);
    }

    // 1. Timestamps.
    let mut invalid_timestamps = 0;
    let mut out_of_window = 0;
    let mut com_tempo: Vec<(DateTime<Utc>, Option<f64>)> = Vec::with_capacity(total);

    for s in samples {
        let Some(ts) = parse_momento(&s.momento_utc) else {
            invalid_timestamps += 1;
            continue;
        };
        if janela.is_some_and(|j| ts < j.inicio || ts > j.fim) {
            out_of_window += 1;
            continue;
        }
        com_tempo.push((ts, s.valor));
    }

    com_tempo.sort_by_key(|s| s.0);

    // 2. Duplicados, faltantes, absurdos e ruído.
    let mut duplicated_timestamps = 0;
    let mut missing_samples = 0;
    let mut out_of_range_samples = 0;
    let mut noisy_samples = 0;
    let mut temporal_gaps = 0;
    let mut largest_gap_seconds = 0;
    let mut validas: Vec<(DateTime<Utc>, f64)> = Vec::new();
    // Referência do ÚLTIMO TIMESTAMP ENCONTRADO (duplicidades e lacunas).
    let mut ultimo_ts_encontrado: Option<DateTime<Utc>> = None;

    for (ts, valor) in com_tempo {
        if ultimo_ts_encontrado == Some(ts) {
            duplicated_timestamps += 1;
            continue;
        }
        if let Some(anterior) = ultimo_ts_encontrado {
            let delta = (ts - anterior).num_seconds();
            if delta > config.max_gap_seconds {
                temporal_gaps += 1;
                largest_gap_seconds = largest_gap_seconds.max(delta);
            }
        }
        ultimo_ts_encontrado = Some(ts);

        let Some(v) = valor else {
            missing_samples += 1;
            continue;
        };
        if fora_dos_limites(v, config) {
            out_of_range_samples += 1;
            continue;
        }

        // Ruído: compara com a última amostra VÁLIDA, considerando o tempo.
        // A referência de ruído é separada da referência de duplicidade:
        // uma amostra ruidosa não vira referência para a próxima, mas o
        // controle temporal de duplicidades continua pelo timestamp.
        if let (Some(&(ts_ant, v_ant)), Some(max_rate)) =
            (validas.last(), config.max_rate_per_second)
        {
            let dt = (ts - ts_ant).num_milliseconds() as f64 / 1000.0;
            if dt > 0.0 && (v - v_ant).abs() / dt > max_rate {
                noisy_samples += 1;
                continue;
            }
        }

        validas.push((ts, v));
    }

    // 3. Critérios de aceitação. Denominador de TODOS os percentuais é o
    // total de amostras recebidas; as categorias são disjuntas (ver topo).
    let pct = |n: usize| ratio_percentage(n, total).unwrap_or(0.0);

    let valid_percentage = pct(validas.len());
    let invalid_pct = pct(invalid_timestamps + duplicated_timestamps + out_of_range_samples);
    let noisy_pct = pct(noisy_samples);

    if out_of_window > 0 {
        motivos.push(MotivoRejeicao::AmostrasForaDaJanela(out_of_window));
    }
    if validas.len() < config.minimum_samples.max(1) {
        motivos.push(MotivoRejeicao::AmostrasInsuficientes {
            validas: validas.len(),
            minimo: config.minimum_samples,
        });
    }
    if valid_percentage < config.minimum_valid_percentage {
        motivos.push(MotivoRejeicao::PercentualValidoBaixo(valid_percentage));
    }
    if invalid_pct > config.max_invalid_percentage {
        motivos.push(MotivoRejeicao::PercentualInvalidoAlto(invalid_pct));
    }
    if noisy_pct > config.max_noisy_percentage {
        motivos.push(MotivoRejeicao::PercentualRuidoAlto(noisy_pct));
    }
    if temporal_gaps > config.max_gaps {
        motivos.push(MotivoRejeicao::LacunasExcessivas(temporal_gaps));
    }

    DataQualityResult {
        total_samples: total,
        valid_samples: validas.len(),
        missing_samples,
        invalid_timestamps,
        duplicated_timestamps,
        out_of_window,
        out_of_range_samples,
        noisy_samples,
        temporal_gaps,
        largest_gap_seconds,
        valid_percentage,
        acceptable_quality: motivos.is_empty(),
        motivos,
        amostras_validas: validas,
    }
}

// ============================================================
// VARIAÇÃO COM REPOSITÓRIO
// ============================================================

pub async fn evaluate_data_quality_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    janela: Option<JanelaTempo>,
    config: &DataQualityConfig,
) -> Result<DataQualityResult, sqlx::Error> {
    let inicio = janela.map(|j| j.inicio.to_rfc3339());
    let fim = janela.map(|j| j.fim.to_rfc3339());

    let serie = buscar_serie_bruta(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio.as_deref(),
        fim.as_deref(),
    )
    .await?;

    Ok(evaluate_data_quality(&serie, janela, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn amostra(ts: &str, valor: Option<f64>) -> AmostraBruta {
        AmostraBruta {
            momento_utc: ts.to_string(),
            valor,
        }
    }

    fn config(min_rate: Option<f64>) -> DataQualityConfig {
        DataQualityConfig {
            minimum_samples: 1,
            minimum_valid_percentage: 0.0,
            max_invalid_percentage: 100.0,
            max_noisy_percentage: 100.0,
            max_gaps: usize::MAX,
            max_gap_seconds: 10,
            min_value: Some(0.0),
            max_value: Some(100.0),
            max_rate_per_second: min_rate,
        }
    }

    fn ts(segundos: i64) -> String {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0)
            .unwrap()
            .to_rfc3339()
    }

    #[test]
    fn amostras_validas_sao_aceitas_e_ordenadas() {
        let serie = vec![amostra(&ts(20), Some(5.0)), amostra(&ts(0), Some(1.0))];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.valid_samples, 2);
        assert_eq!(r.amostras_validas[0].1, 1.0);
        assert!(r.acceptable_quality);
    }

    #[test]
    fn timestamp_invalido_e_contado_separadamente() {
        let serie = vec![
            amostra("nao-e-uma-data", Some(1.0)),
            amostra(&ts(0), Some(1.0)),
        ];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.invalid_timestamps, 1);
        assert_eq!(r.valid_samples, 1);
        assert_eq!(r.total_samples, 2);
    }

    #[test]
    fn timestamps_duplicados_sao_rejeitados_uma_vez_cada() {
        let serie = vec![
            amostra(&ts(0), Some(1.0)),
            amostra(&ts(0), Some(2.0)),
            amostra(&ts(0), Some(3.0)),
        ];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.duplicated_timestamps, 2);
        assert_eq!(r.valid_samples, 1);
        assert_eq!(r.amostras_validas[0].1, 1.0, "mantém a primeira ocorrência");
    }

    #[test]
    fn valores_ausentes_sao_contados() {
        let serie = vec![amostra(&ts(0), None), amostra(&ts(5), Some(1.0))];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.missing_samples, 1);
        assert_eq!(r.valid_samples, 1);
    }

    #[test]
    fn valores_fora_dos_limites_e_nao_finitos_sao_rejeitados() {
        let serie = vec![
            amostra(&ts(0), Some(-1.0)),
            amostra(&ts(5), Some(150.0)),
            amostra(&ts(10), Some(f64::NAN)),
            amostra(&ts(15), Some(50.0)),
        ];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.out_of_range_samples, 3);
        assert_eq!(r.valid_samples, 1);
    }

    #[test]
    fn ruido_e_medido_contra_a_ultima_amostra_valida() {
        // max_rate = 1/s: de 0 a 100 em 5 s = 20/s → ruidosa.
        let serie = vec![
            amostra(&ts(0), Some(10.0)),
            amostra(&ts(5), Some(100.0)), // ruidosa
            amostra(&ts(10), Some(11.0)), // válida de novo (vs. 10.0)
        ];
        let r = evaluate_data_quality(&serie, None, &config(Some(1.0)));
        assert_eq!(r.noisy_samples, 1);
        assert_eq!(r.valid_samples, 2);
        assert_eq!(r.amostras_validas[1].1, 11.0);
    }

    #[test]
    fn lacunas_sao_contadas_com_a_maior_duracao() {
        let serie = vec![
            amostra(&ts(0), Some(1.0)),
            amostra(&ts(5), Some(2.0)),
            // Lacuna de 100 s (max_gap = 10).
            amostra(&ts(105), Some(3.0)),
            amostra(&ts(110), Some(4.0)),
            // Lacuna menor de 30 s.
            amostra(&ts(140), Some(5.0)),
        ];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.temporal_gaps, 2);
        assert_eq!(r.largest_gap_seconds, 100);
        // Amostras posteriores à lacuna continuam válidas.
        assert_eq!(r.valid_samples, 5);
    }

    #[test]
    fn lacunas_excessivas_geram_motivo_sem_descartar_amostras() {
        let serie = vec![amostra(&ts(0), Some(1.0)), amostra(&ts(100), Some(2.0))];
        let mut cfg = config(None);
        cfg.max_gaps = 0;
        let r = evaluate_data_quality(&serie, None, &cfg);
        assert!(!r.acceptable_quality);
        assert!(r.motivos.contains(&MotivoRejeicao::LacunasExcessivas(1)));
        assert_eq!(r.valid_samples, 2);
    }

    #[test]
    fn janela_invalida_e_amostras_fora_da_janela() {
        let inicio = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let fim = inicio;
        let r = evaluate_data_quality(
            &[amostra(&ts(0), Some(1.0))],
            Some(JanelaTempo { inicio, fim }),
            &config(None),
        );
        assert!(r.motivos.contains(&MotivoRejeicao::JanelaInvalida));
    }

    #[test]
    fn cada_amostra_e_contada_uma_unica_vez() {
        let serie = vec![
            amostra("data-ruim", Some(1.0)), // timestamp inválido
            amostra(&ts(0), None),           // sem valor
            amostra(&ts(0), Some(7.0)),      // duplicada
            amostra(&ts(5), Some(500.0)),    // fora dos limites
            amostra(&ts(10), Some(1.0)),     // válida
            amostra(&ts(11), Some(95.0)),    // ruidosa (89/1s > 5)
        ];
        let r = evaluate_data_quality(&serie, None, &config(Some(5.0)));
        let classificadas = r.invalid_timestamps
            + r.duplicated_timestamps
            + r.missing_samples
            + r.out_of_range_samples
            + r.noisy_samples
            + r.valid_samples;
        assert_eq!(classificadas, r.total_samples);
        assert_eq!(r.valid_samples, 1);
    }

    #[test]
    fn configuracao_padrao_varia_por_indicador() {
        let fc = DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca);
        let temp = DataQualityConfig::padrao_para(Indicador::TemperaturaCorporal);
        let sono = DataQualityConfig::padrao_para(Indicador::HorasSono);

        // Cada indicador tem limites coerentes com a sua unidade.
        assert_eq!(fc.max_rate_per_second, Some(5.0));
        assert_eq!(temp.max_rate_per_second, Some(0.5));
        assert_eq!(temp.min_value, Some(30.0));
        // Registro diário não tem taxa de variação e tolera lacunas longas.
        assert_eq!(sono.max_rate_per_second, None);
        assert!(sono.max_gap_seconds > fc.max_gap_seconds);
    }
}
