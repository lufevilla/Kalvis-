// trend.rs

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct TrendResult {
    pub slope_per_hour: f64,
    pub intercept: f64,
    pub first_value: f64,
    pub last_value: f64,
    pub direction: TrendDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

pub fn analyze_trend(
    samples: &[(DateTime<Utc>, f64)],
    minimum_slope: f64,
) -> Option<TrendResult> {
    if samples.len() < 2 {
        return None;
    }

    let mut samples = samples.to_vec();

    samples.sort_by_key(|sample| sample.0);

    let first_timestamp = samples.first()?.0;

    let points: Vec<(f64, f64)> = samples
        .iter()
        .map(|(timestamp, value)| {
            let elapsed_hours =
                (*timestamp - first_timestamp).num_milliseconds() as f64
                / 3_600_000.0;

            (elapsed_hours, *value)
        })
        .collect();

    let n = points.len() as f64;

    let mean_x = points.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = points.iter().map(|p| p.1).sum::<f64>() / n;

    let numerator: f64 = points
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();

    let denominator: f64 = points
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum();

    if denominator == 0.0 {
        return None;
    }

    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;

    let direction = if slope > minimum_slope {
        TrendDirection::Increasing
    } else if slope < -minimum_slope {
        TrendDirection::Decreasing
    } else {
        TrendDirection::Stable
    };

    Some(TrendResult {
        slope_per_hour: slope,
        intercept,
        first_value: samples.first()?.1,
        last_value: samples.last()?.1,
        direction,
    })
}

// ============================================================
// VARIAÇÃO COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::{buscar_serie_do_indicador, parse_momento};

/// Analisa a tendência de um indicador fisiológico buscando a série
/// temporal direto do repositório (janela `[inicio_utc, fim_utc]` opcional).
/// Amostras insuficientes ou série vazia devolvem `None`.
pub async fn analyze_trend_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    minimum_slope: f64,
) -> Result<Option<TrendResult>, sqlx::Error> {
    let serie = buscar_serie_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    let samples: Vec<(DateTime<Utc>, f64)> = serie
        .iter()
        .filter_map(|ponto| {
            let momento = parse_momento(&ponto.momento_utc)?;

            Some((momento, ponto.valor))
        })
        .collect();

    Ok(analyze_trend(&samples, minimum_slope))
}