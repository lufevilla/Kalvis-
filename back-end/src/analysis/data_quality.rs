/*
GARANTE QUE O DADO É CONFIÁVEL ANTES DE QUALQUER CÁLCULO OU ALERTA.

CHECA: JANELA, TIMESTAMPS INVÁLIDOS/DUPLICADOS, FORA DA JANELA, FALTANTES,
VALORES ABSURDOS, RUÍDO DE SENSOR (PICO ISOLADO), LACUNAS, DADO PARADO E
QUANTIDADE. DEVOLVE AS AMOSTRAS VÁLIDAS, ORDENADAS, PRONTAS PARA USO.

CLASSIFICAÇÃO (cada amostra entra em NO MÁXIMO UMA categoria, na ordem):

  1. Timestamp inválido     → `invalid_timestamps`
  2. Fora da janela         → `out_of_window`
  3. Timestamp duplicado    → `duplicated_timestamps`
  4. Valor ausente          → `missing_samples`
  5. Valor fora dos limites → `out_of_range_samples`
  6. Pico isolado (ruído)   → `noisy_samples`
  7. Caso contrário         → válida (`valid_samples`)

RUÍDO: somente PICO ISOLADO — o valor sai do padrão E volta na amostra
seguinte. Mudança sustentada (degrau) é evento fisiológico e é mantida;
descartá-la poderia esconder uma emergência real (ex.: taquicardia, queda
de SpO2).

Todos os percentuais usam como denominador o TOTAL de amostras recebidas
(`total_samples`), então a soma das categorias + válidas = 100%.
*/

use chrono::{DateTime, SecondsFormat, Utc};

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
    /// Variação máxima aceitável por segundo (usada para detectar pico isolado).
    pub max_rate_per_second: Option<f64>,
    /// Atraso máximo entre a última amostra válida e o fim da janela.
    pub max_staleness_seconds: Option<i64>,
}

impl DataQualityConfig {
    /// Configuração padrão POR INDICADOR.
    ///
    /// Limites de PLAUSIBILIDADE DE COLETA e taxas de variação para detectar
    /// ruído — NÃO são limites clínicos nem diagnósticos.
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
            // Registro diário: sem taxa de variação (amostras não contínuas).
            Indicador::HorasSono => (Some(0.0), Some(24.0), None),
        };

        let diario = indicador == Indicador::HorasSono;

        Self {
            minimum_samples: 2,
            minimum_valid_percentage: 80.0,
            max_invalid_percentage: 5.0,
            max_noisy_percentage: 5.0,
            // Não rejeita por contagem de lacunas; fica registrado no resultado.
            max_gaps: usize::MAX,
            // Coleta contínua a cada 5 s; > 5 min indica interrupção.
            max_gap_seconds: if diario { 3 * 86_400 } else { 300 },
            min_value,
            max_value,
            max_rate_per_second,
            max_staleness_seconds: if diario { None } else { Some(30) },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotivoRejeicao {
    JanelaInvalida,
    /// Mantido por compatibilidade; amostras fora da janela agora são
    /// apenas contadas (`out_of_window`) e não rejeitam a análise.
    AmostrasForaDaJanela(usize),
    AmostrasInsuficientes { validas: usize, minimo: usize },
    PercentualValidoBaixo(f64),
    PercentualInvalidoAlto(f64),
    PercentualRuidoAlto(f64),
    LacunasExcessivas(usize),
    /// Segundos entre a última amostra válida e o fim da janela.
    /// Indica sensor desconectado/parado — tratar como "verificar sensor".
    DadoDesatualizado(i64),
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
    pub out_of_range_samples: usize,
    pub noisy_samples: usize,
    pub temporal_gaps: usize,
    /// Duração (s) da maior lacuna temporal entre amostras.
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

/// Taxa de variação (unidade/s) entre dois pontos.
fn taxa(t0: DateTime<Utc>, v0: f64, t1: DateTime<Utc>, v1: f64) -> f64 {
    let dt = (t1 - t0).num_milliseconds() as f64 / 1000.0;
    if dt <= 0.0 { 0.0 } else { (v1 - v0) / dt }
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

    // 1. Timestamps inválidos e fora da janela.
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

    // Em timestamps duplicados, a amostra COM valor vem primeiro.
    com_tempo.sort_by_key(|s| (s.0, s.1.is_none()));

    // 2a. Duplicados, ausentes, limites e lacunas.
    let mut duplicated_timestamps = 0;
    let mut missing_samples = 0;
    let mut out_of_range_samples = 0;
    let mut noisy_samples = 0;
    let mut temporal_gaps = 0;
    let mut largest_gap_seconds = 0;
    let mut candidatos: Vec<(DateTime<Utc>, f64)> = Vec::with_capacity(com_tempo.len());
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
        candidatos.push((ts, v));
    }

    // 2b. Ruído = pico isolado (sai do padrão e volta). Degrau sustentado é válido.
    let mut validas: Vec<(DateTime<Utc>, f64)> = Vec::with_capacity(candidatos.len());
    for i in 0..candidatos.len() {
        let (ts, v) = candidatos[i];
        let pico = match (validas.last(), candidatos.get(i + 1), config.max_rate_per_second) {
            (Some(&(ta, va)), Some(&(tn, vn)), Some(max)) => {
                let ida = taxa(ta, va, ts, v);
                let volta = taxa(ts, v, tn, vn);
                ida.abs() > max && volta.abs() > max && ida.signum() != volta.signum()
            }
            _ => false,
        };
        if pico {
            noisy_samples += 1;
        } else {
            validas.push((ts, v));
        }
    }

    // 3. Critérios de aceitação (denominador = total recebido).
    let pct = |n: usize| ratio_percentage(n, total).unwrap_or(0.0);

    let valid_percentage = pct(validas.len());
    let invalid_pct = pct(invalid_timestamps + duplicated_timestamps + out_of_range_samples);
    let noisy_pct = pct(noisy_samples);

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
    // Dado parado: última amostra válida muito antes do fim da janela.
    if let (Some(j), Some(max)) = (janela, config.max_staleness_seconds) {
        let ultimo = validas.last().map(|s| s.0).unwrap_or(j.inicio);
        let atraso = (j.fim - ultimo).num_seconds();
        if atraso > max {
            motivos.push(MotivoRejeicao::DadoDesatualizado(atraso));
        }
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
// QUALIDADE COM REPOSITÓRIO
// ============================================================

pub async fn evaluate_data_quality_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    janela: Option<JanelaTempo>,
    config: &DataQualityConfig,
) -> Result<DataQualityResult, sqlx::Error> {
    // Formato idêntico ao gravado no banco (YYYY-MM-DDTHH:MM:SS.sssZ).
    let inicio = janela.map(|j| j.inicio.to_rfc3339_opts(SecondsFormat::Millis, true));
    let fim = janela.map(|j| j.fim.to_rfc3339_opts(SecondsFormat::Millis, true));

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

    fn config(max_rate: Option<f64>) -> DataQualityConfig {
        DataQualityConfig {
            minimum_samples: 1,
            minimum_valid_percentage: 0.0,
            max_invalid_percentage: 100.0,
            max_noisy_percentage: 100.0,
            max_gaps: usize::MAX,
            max_gap_seconds: 10,
            min_value: Some(0.0),
            max_value: Some(100.0),
            max_rate_per_second: max_rate,
            max_staleness_seconds: None,
        }
    }

    fn base() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn ts(segundos: i64) -> String {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0)
            .unwrap()
            .to_rfc3339_opts(SecondsFormat::Millis, true)
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
    fn duplicado_prioriza_valor_presente() {
        let serie = vec![amostra(&ts(0), None), amostra(&ts(0), Some(5.0))];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.valid_samples, 1);
        assert_eq!(r.missing_samples, 0);
        assert_eq!(r.duplicated_timestamps, 1);
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
    fn pico_isolado_e_ruido() {
        // max_rate = 1/s: 10 → 100 → 11 = sai e volta.
        let serie = vec![
            amostra(&ts(0), Some(10.0)),
            amostra(&ts(5), Some(100.0)), // pico
            amostra(&ts(10), Some(11.0)),
        ];
        let r = evaluate_data_quality(&serie, None, &config(Some(1.0)));
        assert_eq!(r.noisy_samples, 1);
        assert_eq!(r.valid_samples, 2);
        assert_eq!(r.amostras_validas[1].1, 11.0);
    }

    #[test]
    fn degrau_sustentado_nao_e_ruido() {
        // Taquicardia real: 70 → 140 sustentado.
        let serie = vec![
            amostra(&ts(0), Some(70.0)),
            amostra(&ts(5), Some(140.0)),
            amostra(&ts(10), Some(140.0)),
            amostra(&ts(15), Some(141.0)),
        ];
        let mut cfg = config(Some(5.0));
        cfg.max_value = Some(250.0);
        let r = evaluate_data_quality(&serie, None, &cfg);
        assert_eq!(r.noisy_samples, 0);
        assert_eq!(r.valid_samples, 4);
    }

    #[test]
    fn lacunas_sao_contadas_com_a_maior_duracao() {
        let serie = vec![
            amostra(&ts(0), Some(1.0)),
            amostra(&ts(5), Some(2.0)),
            amostra(&ts(105), Some(3.0)), // lacuna de 100 s
            amostra(&ts(110), Some(4.0)),
            amostra(&ts(140), Some(5.0)), // lacuna de 30 s
        ];
        let r = evaluate_data_quality(&serie, None, &config(None));
        assert_eq!(r.temporal_gaps, 2);
        assert_eq!(r.largest_gap_seconds, 100);
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
    fn janela_invalida_gera_motivo() {
        let r = evaluate_data_quality(
            &[amostra(&ts(0), Some(1.0))],
            Some(JanelaTempo { inicio: base(), fim: base() }),
            &config(None),
        );
        assert!(r.motivos.contains(&MotivoRejeicao::JanelaInvalida));
    }

    #[test]
    fn fora_da_janela_e_contado_mas_nao_rejeita() {
        let janela = JanelaTempo {
            inicio: base(),
            fim: base() + chrono::Duration::seconds(20),
        };
        let serie = vec![
            amostra(&ts(-5), Some(1.0)), // fora
            amostra(&ts(0), Some(1.0)),
            amostra(&ts(10), Some(1.0)),
        ];
        let r = evaluate_data_quality(&serie, Some(janela), &config(None));
        assert_eq!(r.out_of_window, 1);
        assert_eq!(r.valid_samples, 2);
        assert!(r.acceptable_quality);
    }

    #[test]
    fn dado_desatualizado_e_detectado() {
        let janela = JanelaTempo {
            inicio: base(),
            fim: base() + chrono::Duration::seconds(120),
        };
        let serie = vec![amostra(&ts(0), Some(1.0)), amostra(&ts(5), Some(1.0))];
        let mut cfg = config(None);
        cfg.max_staleness_seconds = Some(30);
        let r = evaluate_data_quality(&serie, Some(janela), &cfg);
        assert!(r.motivos.contains(&MotivoRejeicao::DadoDesatualizado(115)));
        assert!(!r.acceptable_quality);
    }

    #[test]
    fn cada_amostra_e_contada_uma_unica_vez() {
        let serie = vec![
            amostra("data-ruim", Some(1.0)), // timestamp inválido
            amostra(&ts(0), Some(1.0)),      // válida
            amostra(&ts(0), None),           // duplicada
            amostra(&ts(5), Some(500.0)),    // fora dos limites
            amostra(&ts(10), Some(95.0)),    // pico isolado
            amostra(&ts(11), Some(1.0)),     // válida
            amostra(&ts(20), None),          // ausente
        ];
        let r = evaluate_data_quality(&serie, None, &config(Some(5.0)));
        let classificadas = r.invalid_timestamps
            + r.out_of_window
            + r.duplicated_timestamps
            + r.missing_samples
            + r.out_of_range_samples
            + r.noisy_samples
            + r.valid_samples;
        assert_eq!(classificadas, r.total_samples);
        assert_eq!(r.noisy_samples, 1);
        assert_eq!(r.valid_samples, 2);
    }

    #[test]
    fn configuracao_padrao_varia_por_indicador() {
        let fc = DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca);
        let temp = DataQualityConfig::padrao_para(Indicador::TemperaturaCorporal);
        let sono = DataQualityConfig::padrao_para(Indicador::HorasSono);

        assert_eq!(fc.max_rate_per_second, Some(5.0));
        assert_eq!(fc.max_staleness_seconds, Some(30));
        assert_eq!(temp.max_rate_per_second, Some(0.5));
        assert_eq!(temp.min_value, Some(30.0));
        assert_eq!(sono.max_rate_per_second, None);
        assert_eq!(sono.max_staleness_seconds, None);
        assert!(sono.max_gap_seconds > fc.max_gap_seconds);
    }
}
