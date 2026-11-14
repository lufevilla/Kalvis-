/*
GARANTE QUE O DADO É CONFIÁVEL ANTES DE QUALQUER CÁLCULO OU ALERTA.

CHECA: JANELA, TIMESTAMPS INVÁLIDOS/DUPLICADOS, FORA DA JANELA, FALTANTES,
VALORES ABSURDOS, RUÍDO DE SENSOR (TAXA DE VARIAÇÃO), LACUNAS E QUANTIDADE.
DEVOLVE AS AMOSTRAS VÁLIDAS, ORDENADAS, PRONTAS PARA USO.
*/

use chrono::{DateTime, Utc};

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::{buscar_serie_bruta, parse_momento, AmostraBruta, Indicador};
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
    pub invalid_samples: usize,
    pub noisy_samples: usize,
    pub temporal_gaps: usize,
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
    let mut invalid_values = 0;
    let mut noisy_samples = 0;
    let mut temporal_gaps = 0;
    let mut validas: Vec<(DateTime<Utc>, f64)> = Vec::new();
    let mut ultimo_ts: Option<DateTime<Utc>> = None;

    for (ts, valor) in com_tempo {
        if ultimo_ts == Some(ts) {
            duplicated_timestamps += 1;
            continue;
        }
        if let Some(anterior) = ultimo_ts {
            if (ts - anterior).num_seconds() > config.max_gap_seconds {
                temporal_gaps += 1;
            }
        }
        ultimo_ts = Some(ts);

        let Some(v) = valor else {
            missing_samples += 1;
            continue;
        };
        if fora_dos_limites(v, config) {
            invalid_values += 1;
            continue;
        }

        // Ruído: compara com a última amostra VÁLIDA, considerando o tempo.
        // Uma amostra ruidosa não vira referência para a próxima.
        if let (Some(&(ts_ant, v_ant)), Some(max_rate)) = (validas.last(), config.max_rate_per_second) {
            let dt = (ts - ts_ant).num_milliseconds() as f64 / 1000.0;
            if dt > 0.0 && (v - v_ant).abs() / dt > max_rate {
                noisy_samples += 1;
                continue;
            }
        }

        validas.push((ts, v));
    }

    // 3. Critérios de aceitação.
    let invalid_samples = invalid_timestamps + duplicated_timestamps + invalid_values;
    let pct = |n: usize| ratio_percentage(n, total).unwrap_or(0.0);

    let valid_percentage = pct(validas.len());
    let invalid_pct = pct(invalid_samples);
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
        invalid_samples,
        noisy_samples,
        temporal_gaps,
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
        repo, astronauta_id, missao_id, indicador, inicio.as_deref(), fim.as_deref(),
    )
    .await?;

    Ok(evaluate_data_quality(&serie, janela, config))
}
