/*
FUNCOES MATEMATICAS PRAS REGRAS DO ENVIRONMENTAL

1. DESVIO
2. TENDENCIA
3. PERSISTENCIA
4. CORRELAÇÃO DE CONTEXTO
*/

// src/application/engine/statistics.rs

// ============================================================
// 1. DEVIATION
// ============================================================

pub async fn absolute_deviation(current: f64, reference: f64) -> f64 {
    current - reference
}

pub async fn percentage_deviation(current: f64, reference: f64) -> Option<f64> {
    if !reference.is_finite() || reference.abs() < f64::EPSILON {
        return None;
    }

    Some(((current - reference) / reference.abs()) * 100.0)
}

pub async fn z_score(current: f64, mean: f64, standard_deviation: f64) -> Option<f64> {
    if standard_deviation <= 0.0 {
        return None;
    }

    Some((current - mean) / standard_deviation)
}


// ============================================================
// 2. TREND
// ============================================================

pub async fn moving_average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    Some(values.iter().sum::<f64>() / values.len() as f64)
}

pub async fn linear_regression(values: &[f64]) -> Option<(f64, f64)> {
    let n = values.len();

    if n < 2 {
        return None;
    }

    let n_f = n as f64;

    let mean_x = (n_f - 1.0) / 2.0;
    let mean_y = values.iter().sum::<f64>() / n_f;

    let numerator: f64 = values
        .iter()
        .enumerate()
        .map(|(i, y)| (i as f64 - mean_x) * (y - mean_y))
        .sum();

    let denominator: f64 = (0..n)
        .map(|i| (i as f64 - mean_x).powi(2))
        .sum();

    if denominator == 0.0 {
        return None;
    }

    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;

    Some((slope, intercept))
}


// ============================================================
// 3. PERSISTENCE
// ============================================================

pub async fn persistence_percentage(total: usize, occurrences: usize) -> Option<f64> {
    if total == 0 || occurrences > total {
        return None;
    }

    Some((occurrences as f64 / total as f64) * 100.0)
}

pub async fn consecutive_occurrences(values: &[bool]) -> usize {
    let mut max_count = 0;
    let mut current_count = 0;

    for &value in values {
        if value {
            current_count += 1;
            max_count = max_count.max(current_count);
        } else {
            current_count = 0;
        }
    }

    max_count
}


// ============================================================
// 4. CORRELATION
// ============================================================

pub async fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }

    if x.iter().chain(y.iter()).any(|v| !v.is_finite()) {
        return None;
    }

    let n = x.len() as f64;

    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let numerator: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(a, b)| (a - mean_x) * (b - mean_y))
        .sum();

    let sum_x: f64 = x.iter().map(|a| (a - mean_x).powi(2)).sum();
    let sum_y: f64 = y.iter().map(|b| (b - mean_y).powi(2)).sum();

    let denominator = sum_x.sqrt() * sum_y.sqrt();

    if denominator == 0.0 || !denominator.is_finite() {
        return None;
    }

    Some((numerator / denominator).clamp(-1.0, 1.0))
}

// ============================================================
// 5. ADDITIONAL STATISTICS
// ============================================================
pub async fn median(values: &[f64]) -> Option<f64> {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();

    if sorted.is_empty() {
        return None;
    }

    sorted.sort_by(|a, b| a.total_cmp(b));

    let middle = sorted.len() / 2;

    if sorted.len() % 2 == 0 {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    } else {
        Some(sorted[middle])
    }
}

pub async fn standard_deviation(values: &[f64]) -> Option<f64> {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();

    if finite.len() < 2 {
        return None;
    }

    let mean = finite.iter().sum::<f64>() / finite.len() as f64;

    let variance = finite
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (finite.len() - 1) as f64;

    Some(variance.sqrt())
}

pub async fn median_absolute_deviation(values: &[f64]) -> Option<f64> {
    let median_value = median(values).await?;

    let deviations: Vec<f64> = values
        .iter()
        .map(|value| (value - median_value).abs())
        .collect();

    median(&deviations).await
}

/// MAD escalado (1.4826 × MAD), comparável ao desvio padrão sob distribuição normal.
pub async fn scaled_median_absolute_deviation(values: &[f64]) -> Option<f64> {
    median_absolute_deviation(values)
        .await
        .map(|mad| mad * 1.4826)
}

// ============================================================
// 6. VARIAÇÕES COM REPOSITÓRIO (BUSCAM OS PRÓPRIOS DADOS)
// ============================================================

use std::collections::HashMap;

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::{
    buscar_serie_do_indicador, buscar_valores_do_indicador,
};

/// Média móvel de um indicador fisiológico buscado do repositório.
pub async fn moving_average_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<f64>, sqlx::Error> {
    let values = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    Ok(moving_average(&values).await)
}

/// Regressão linear sobre os valores (em ordem temporal) de um indicador.
pub async fn linear_regression_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<(f64, f64)>, sqlx::Error> {
    let values = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    Ok(linear_regression(&values).await)
}

/// Mediana de um indicador fisiológico buscado do repositório.
pub async fn median_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<f64>, sqlx::Error> {
    let values = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    Ok(median(&values).await)
}

/// Desvio padrão de um indicador fisiológico buscado do repositório.
pub async fn standard_deviation_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<f64>, sqlx::Error> {
    let values = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    Ok(standard_deviation(&values).await)
}

/// MAD de um indicador fisiológico buscado do repositório.
pub async fn median_absolute_deviation_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<f64>, sqlx::Error> {
    let values = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    Ok(median_absolute_deviation(&values).await)
}

/// MAD escalado de um indicador fisiológico buscado do repositório.
pub async fn scaled_median_absolute_deviation_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<f64>, sqlx::Error> {
    let values = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    Ok(scaled_median_absolute_deviation(&values).await)
}

/// Correlação de Pearson entre dois indicadores fisiológicos, alinhando as
/// duas séries pelos timestamps em comum (junção interna).
pub async fn pearson_correlation_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador_x: &str,
    indicador_y: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Option<f64>, sqlx::Error> {
    let serie_x = buscar_serie_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador_x,
        inicio_utc,
        fim_utc,
    )
    .await?;

    let serie_y = buscar_serie_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador_y,
        inicio_utc,
        fim_utc,
    )
    .await?;

    let valores_y: HashMap<&str, f64> = serie_y
        .iter()
        .map(|ponto| (ponto.momento_utc.as_str(), ponto.valor))
        .collect();

    let mut x: Vec<f64> = Vec::new();
    let mut y: Vec<f64> = Vec::new();

    for ponto in &serie_x {
        if let Some(valor_y) = valores_y.get(ponto.momento_utc.as_str()) {
            x.push(ponto.valor);
            y.push(*valor_y);
        }
    }

    Ok(pearson_correlation(&x, &y).await)
}