 /*
 CHECA SE O DADO É CONFIÁVEL ANTES DE FAZER ALGO
 */

 // data_quality.rs

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DataQualityConfig {
    pub minimum_samples: usize,
    pub minimum_valid_percentage: f64,
    pub max_gap_seconds: i64,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub max_step: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DataQualityResult {
    pub total_samples: usize,
    pub valid_samples: usize,
    pub missing_samples: usize,
    pub invalid_samples: usize,
    pub noisy_samples: usize,
    pub temporal_gaps: usize,
    pub valid_percentage: f64,
    pub sufficient_samples: bool,
    pub acceptable_quality: bool,
}

pub fn evaluate_data_quality(
    samples: &[(DateTime<Utc>, Option<f64>)],
    config: &DataQualityConfig,
) -> DataQualityResult {
    let total = samples.len();

    let mut sorted = samples.to_vec();
    sorted.sort_by_key(|sample| sample.0);

    let mut valid_samples = 0;
    let mut missing_samples = 0;
    let mut invalid_samples = 0;
    let mut noisy_samples = 0;
    let mut temporal_gaps = 0;

    let mut previous_valid: Option<f64> = None;

    for (index, (timestamp, value)) in sorted.iter().enumerate() {
        if index > 0 {
            let previous_timestamp = sorted[index - 1].0;

            let gap = (*timestamp - previous_timestamp).num_seconds();

            if gap > config.max_gap_seconds {
                temporal_gaps += 1;
            }
        }

        let Some(value) = value else {
            missing_samples += 1;
            continue;
        };

        if !value.is_finite() {
            invalid_samples += 1;
            continue;
        }

        let outside_min = config
            .min_value
            .is_some_and(|min| *value < min);

        let outside_max = config
            .max_value
            .is_some_and(|max| *value > max);

        if outside_min || outside_max {
            invalid_samples += 1;
            continue;
        }

        if let (Some(previous), Some(max_step)) =
            (previous_valid, config.max_step)
        {
            if (*value - previous).abs() > max_step {
                noisy_samples += 1;
            }
        }

        previous_valid = Some(*value);
        valid_samples += 1;
    }

    let valid_percentage = if total > 0 {
        (valid_samples as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let sufficient_samples =
        valid_samples >= config.minimum_samples;

    let acceptable_quality =
        sufficient_samples
        && valid_percentage >= config.minimum_valid_percentage
        && invalid_samples == 0
        && noisy_samples == 0
        && temporal_gaps == 0;

    DataQualityResult {
        total_samples: total,
        valid_samples,
        missing_samples,
        invalid_samples,
        noisy_samples,
        temporal_gaps,
        valid_percentage,
        sufficient_samples,
        acceptable_quality,
    }
}

// ============================================================
// VARIAÇÃO COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::{buscar_serie_do_indicador, parse_momento};

/// Avalia a qualidade do dado de um indicador fisiológico buscando a
/// série temporal direto do repositório (janela `[inicio_utc, fim_utc]`
/// opcional). Série vazia devolve `None`.
pub async fn evaluate_data_quality_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    config: &DataQualityConfig,
) -> Result<Option<DataQualityResult>, sqlx::Error> {
    let serie = buscar_serie_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    if serie.is_empty() {
        return Ok(None);
    }

    let samples: Vec<(DateTime<Utc>, Option<f64>)> = serie
        .iter()
        .filter_map(|ponto| {
            let momento = parse_momento(&ponto.momento_utc)?;

            Some((momento, Some(ponto.valor)))
        })
        .collect();

    Ok(Some(evaluate_data_quality(&samples, config)))
}