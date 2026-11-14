// statistics.rs — só matemática pura, síncrona, sem repositório.
fn finite(v: &[f64]) -> Vec<f64> { v.iter().copied().filter(|x| x.is_finite()).collect() }

pub fn mean(v: &[f64]) -> Option<f64> {
    let f = finite(v);
    (!f.is_empty()).then(|| f.iter().sum::<f64>() / f.len() as f64)
}
pub fn variance(v: &[f64]) -> Option<f64> {
    let f = finite(v);
    if f.len() < 2 { return None; }
    let m = mean(&f)?;
    Some(f.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (f.len() - 1) as f64)
}
pub fn standard_deviation(v: &[f64]) -> Option<f64> { variance(v).map(f64::sqrt) }
pub fn min(v: &[f64]) -> Option<f64> { finite(v).into_iter().reduce(f64::min) }
pub fn max(v: &[f64]) -> Option<f64> { finite(v).into_iter().reduce(f64::max) }
pub fn median(v: &[f64]) -> Option<f64> { /* versão atual, sem async */ }
pub fn scaled_mad(v: &[f64]) -> Option<f64> { /* 1.4826 × MAD */ }

/// Regressão com X real (ex.: horas), usada pelo trend.rs.
pub fn linear_regression(points: &[(f64, f64)]) -> Option<(f64, f64)> { /* slope, intercept */ }
pub fn ratio_percentage(part: usize, total: usize) -> Option<f64> { /* ... */ }
pub fn max_consecutive(flags: &[bool]) -> usize { /* ... */ }
pub fn trapezoid(a: f64, b: f64, dt: f64) -> f64 { (a + b) / 2.0 * dt }
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> { /* atual, sem async */ }
/// (x - média) / desvio. `None` se o desvio for ≤ 0 ou algum valor não for finito.
pub fn z_score(x: f64, media: f64, desvio: f64) -> Option<f64> {
    (x.is_finite() && media.is_finite() && desvio.is_finite() && desvio > 0.0)
        .then(|| (x - media) / desvio)
}

/// Variação percentual de `de` para `para`. `None` se `de` for 0.
pub fn percent_change(de: f64, para: f64) -> Option<f64> {
    (de != 0.0 && de.is_finite() && para.is_finite())
        .then(|| (para - de) / de.abs() * 100.0)
}
