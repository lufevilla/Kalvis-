// statistics.rs — só matemática pura, síncrona, sem repositório.
//
// Contrato geral de todas as funções:
// - Valores não finitos (`NaN`, `±inf`) são descartados antes do cálculo,
//   EXCETO onde a assinatura recebe escalares individuais (aí o resultado é
//   `None`/valor neutro quando algum escalar não for finito).
// - Listas vazias (ou sem nenhum valor finito) devolvem `None`.
// - Nenhuma função devolve `NaN` ou infinito.
// - A filtragem de não finitos aqui NÃO substitui a validação feita em
//   `data_quality.rs`; é apenas uma salvaguarda matemática.

/// Filtra valores não finitos. Uso interno das funções puras; não é um
/// substituto da validação de qualidade de `data_quality.rs`.
fn finite(v: &[f64]) -> Vec<f64> {
    v.iter().copied().filter(|x| x.is_finite()).collect()
}

/// Média aritmética. `None` se não houver nenhum valor finito.
pub fn mean(v: &[f64]) -> Option<f64> {
    let f = finite(v);
    (!f.is_empty()).then(|| f.iter().sum::<f64>() / f.len() as f64)
}

/// Variância amostral (denominador `n - 1`). `None` com menos de 2 valores
/// finitos (com 1 elemento a variância não é definida).
pub fn variance(v: &[f64]) -> Option<f64> {
    let f = finite(v);
    if f.len() < 2 {
        return None;
    }
    let m = mean(&f)?;
    Some(f.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (f.len() - 1) as f64)
}

/// Desvio-padrão amostral (raiz da variância). `None` quando a variância
/// não é definida (menos de 2 valores finitos).
pub fn standard_deviation(v: &[f64]) -> Option<f64> {
    variance(v).map(f64::sqrt)
}

/// Mínimo dos valores finitos. `None` se a lista estiver vazia ou não
/// contiver nenhum valor finito.
pub fn min(v: &[f64]) -> Option<f64> {
    finite(v).into_iter().reduce(f64::min)
}

/// Máximo dos valores finitos. `None` se a lista estiver vazia ou não
/// contiver nenhum valor finito.
pub fn max(v: &[f64]) -> Option<f64> {
    finite(v).into_iter().reduce(f64::max)
}

/// Mediana dos valores finitos. Com quantidade par, é a média dos dois
/// valores centrais. `None` se não houver valores finitos.
pub fn median(v: &[f64]) -> Option<f64> {
    let mut f = finite(v);
    if f.is_empty() {
        return None;
    }
    f.sort_by(|a, b| a.partial_cmp(b).expect("valores já filtrados são finitos"));
    let meio = f.len() / 2;
    if f.len() % 2 == 1 {
        Some(f[meio])
    } else {
        Some((f[meio - 1] + f[meio]) / 2.0)
    }
}

/// MAD escalado: `1.4826 × mediana(|x − mediana(x)|)`.
///
/// O fator 1.4826 torna o MAD comparável ao desvio-padrão de uma
/// distribuição normal. Com um único valor (ou todos iguais) devolve
/// `Some(0.0)` — dispersão zero é um resultado válido, cabe ao chamador
/// decidir o que fazer com ela. `None` sem valores finitos.
pub fn scaled_mad(v: &[f64]) -> Option<f64> {
    let centro = median(v)?;
    let desvios: Vec<f64> = finite(v).into_iter().map(|x| (x - centro).abs()).collect();
    median(&desvios).map(|mad| mad * 1.4826)
}

/// Regressão linear por mínimos quadrados sobre pontos `(x, y)`.
/// Devolve `(inclinação, intercepto)`.
///
/// - Pontos com coordenadas não finitas são descartados.
/// - `None` com menos de 2 pontos ou quando todos os `x` forem iguais
///   (denominador zero — a inclinação é indefinida).
pub fn linear_regression(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    let pontos: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();

    if pontos.len() < 2 {
        return None;
    }

    let n = pontos.len() as f64;
    let mean_x = pontos.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = pontos.iter().map(|p| p.1).sum::<f64>() / n;

    let numerador: f64 = pontos
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();
    let denominador: f64 = pontos.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();

    if !denominador.is_finite() || denominador <= f64::EPSILON {
        return None;
    }

    let slope = numerador / denominador;
    if !slope.is_finite() {
        return None;
    }
    Some((slope, mean_y - slope * mean_x))
}

/// `parte / total × 100`. `None` se `total == 0`.
pub fn ratio_percentage(part: usize, total: usize) -> Option<f64> {
    (total > 0).then(|| part as f64 / total as f64 * 100.0)
}

/// Maior quantidade consecutiva de `true` na lista. Lista vazia → `0`.
pub fn max_consecutive(flags: &[bool]) -> usize {
    let mut atual = 0;
    let mut maior = 0;
    for &flag in flags {
        if flag {
            atual += 1;
            maior = maior.max(atual);
        } else {
            atual = 0;
        }
    }
    maior
}

/// Regra do trapézio para um único intervalo: `(a + b) / 2 × dt`.
/// Devolve `0.0` quando `a`, `b` ou `dt` não forem finitos, ou quando
/// `dt <= 0` (intervalo de duração nula ou negativa não integra nada).
pub fn trapezoid(a: f64, b: f64, dt: f64) -> f64 {
    if !a.is_finite() || !b.is_finite() || !dt.is_finite() || dt <= 0.0 {
        return 0.0;
    }
    (a + b) / 2.0 * dt
}

/// Correlação de Pearson entre duas séries pareadas `x` e `y`.
///
/// - As séries precisam ter o mesmo comprimento; pares com qualquer valor
///   não finito são descartados antes do cálculo.
/// - `None` com menos de 2 pares válidos ou quando `x` (ou `y`) tem
///   variância zero (correlação é indefinida — evita divisão por zero).
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() {
        return None;
    }

    let pares: Vec<(f64, f64)> = x
        .iter()
        .zip(y.iter())
        .map(|(&a, &b)| (a, b))
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .collect();

    if pares.len() < 2 {
        return None;
    }

    let n = pares.len() as f64;
    let mean_x = pares.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = pares.iter().map(|p| p.1).sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for (a, b) in &pares {
        let dx = a - mean_x;
        let dy = b - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denominador = (var_x * var_y).sqrt();
    if !denominador.is_finite() || denominador <= f64::EPSILON {
        return None;
    }
    Some((cov / denominador).clamp(-1.0, 1.0))
}

/// Z-score: `(x − média) / desvio`. `None` se o desvio for ≤ 0 ou algum
/// valor não for finito. Interpretar o z-score como desvio em unidades de
/// desvio-padrão da referência fornecida — não é um limite clínico universal.
pub fn z_score(x: f64, media: f64, desvio: f64) -> Option<f64> {
    (x.is_finite() && media.is_finite() && desvio.is_finite() && desvio > 0.0)
        .then(|| (x - media) / desvio)
}

/// Variação percentual de `de` para `para`, usando `|de|` como base.
/// `None` se `de == 0` (divisão por zero) ou algum valor não for finito.
pub fn percent_change(de: f64, para: f64) -> Option<f64> {
    (de != 0.0 && de.is_finite() && para.is_finite()).then(|| (para - de) / de.abs() * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aprox(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // ===== Média =====

    #[test]
    fn media_de_valores_normais() {
        assert!(aprox(mean(&[1.0, 2.0, 3.0]).unwrap(), 2.0));
    }

    #[test]
    fn media_descarta_nao_finitos() {
        assert!(aprox(
            mean(&[1.0, f64::NAN, 3.0, f64::INFINITY]).unwrap(),
            2.0
        ));
    }

    #[test]
    fn media_de_lista_vazia_ou_sem_finitos_e_none() {
        assert_eq!(mean(&[]), None);
        assert_eq!(mean(&[f64::NAN]), None);
    }

    // ===== Variância e desvio-padrão =====

    #[test]
    fn variancia_amostral() {
        // [1, 2, 3] -> variância amostral = 1.0
        assert!(aprox(variance(&[1.0, 2.0, 3.0]).unwrap(), 1.0));
        assert!(aprox(standard_deviation(&[1.0, 2.0, 3.0]).unwrap(), 1.0));
    }

    #[test]
    fn variancia_com_um_elemento_e_none() {
        assert_eq!(variance(&[5.0]), None);
        assert_eq!(standard_deviation(&[5.0]), None);
        assert_eq!(variance(&[]), None);
    }

    // ===== Mínimo, máximo, mediana =====

    #[test]
    fn min_e_max_ignoram_nao_finitos() {
        assert_eq!(min(&[4.0, f64::NAN, 2.0]), Some(2.0));
        assert_eq!(max(&[4.0, f64::NAN, 2.0]), Some(4.0));
        assert_eq!(min(&[]), None);
        assert_eq!(max(&[f64::INFINITY, f64::NEG_INFINITY]), None);
    }

    #[test]
    fn mediana_impar_e_par() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert!(aprox(median(&[4.0, 1.0, 2.0, 3.0]).unwrap(), 2.5));
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[f64::NAN, f64::INFINITY]), None);
    }

    #[test]
    fn mediana_com_um_elemento() {
        assert_eq!(median(&[7.0]), Some(7.0));
    }

    // ===== MAD escalado =====

    #[test]
    fn mad_escalado_de_distribuicao_simetrica() {
        // [1, 2, 3]: mediana 2, desvios [1, 0, 1], MAD 1 → escalado 1.4826.
        assert!(aprox(scaled_mad(&[1.0, 2.0, 3.0]).unwrap(), 1.4826));
    }

    #[test]
    fn mad_escalado_zero_quando_todos_iguais() {
        // Dispersão zero é resultado válido: Some(0.0), não None.
        assert_eq!(scaled_mad(&[5.0, 5.0, 5.0]), Some(0.0));
        assert_eq!(scaled_mad(&[5.0]), Some(0.0));
    }

    #[test]
    fn mad_escalado_sem_valores_finitos_e_none() {
        assert_eq!(scaled_mad(&[]), None);
        assert_eq!(scaled_mad(&[f64::NAN]), None);
    }

    // ===== Regressão linear =====

    #[test]
    fn regressao_linear_perfeita() {
        // y = 2x + 1
        let pontos = [(0.0, 1.0), (1.0, 3.0), (2.0, 5.0)];
        let (slope, intercept) = linear_regression(&pontos).unwrap();
        assert!(aprox(slope, 2.0));
        assert!(aprox(intercept, 1.0));
    }

    #[test]
    fn regressao_linear_todos_x_iguais_e_none() {
        let pontos = [(1.0, 1.0), (1.0, 2.0), (1.0, 3.0)];
        assert_eq!(linear_regression(&pontos), None);
    }

    #[test]
    fn regressao_linear_poucos_pontos_e_none() {
        assert_eq!(linear_regression(&[]), None);
        assert_eq!(linear_regression(&[(1.0, 2.0)]), None);
    }

    #[test]
    fn regressao_linear_descarta_pontos_nao_finitos() {
        let pontos = [
            (0.0, 1.0),
            (f64::NAN, 9.0),
            (1.0, 3.0),
            (f64::INFINITY, 0.0),
        ];
        let (slope, _) = linear_regression(&pontos).unwrap();
        assert!(aprox(slope, 2.0));
    }

    // ===== ratio_percentage =====

    #[test]
    fn ratio_percentage_normal_e_total_zero() {
        assert!(aprox(ratio_percentage(1, 4).unwrap(), 25.0));
        assert_eq!(ratio_percentage(1, 0), None);
    }

    // ===== max_consecutive =====

    #[test]
    fn max_consecutive_conta_sequencias() {
        assert_eq!(
            max_consecutive(&[false, true, true, false, true, true, true]),
            3
        );
        assert_eq!(max_consecutive(&[false, false]), 0);
        assert_eq!(max_consecutive(&[]), 0);
        assert_eq!(max_consecutive(&[true, true, true]), 3);
    }

    // ===== trapezoid =====

    #[test]
    fn trapezoid_normal() {
        assert!(aprox(trapezoid(2.0, 4.0, 3.0), 9.0));
    }

    #[test]
    fn trapezoid_com_entrada_invalida_e_zero() {
        assert_eq!(trapezoid(f64::NAN, 1.0, 1.0), 0.0);
        assert_eq!(trapezoid(1.0, 1.0, f64::INFINITY), 0.0);
        assert_eq!(trapezoid(1.0, 2.0, 0.0), 0.0);
        assert_eq!(trapezoid(1.0, 2.0, -1.0), 0.0);
    }

    // ===== pearson_correlation =====

    #[test]
    fn correlacao_perfeita_positiva_e_negativa() {
        let x = [1.0, 2.0, 3.0];
        let y = [2.0, 4.0, 6.0];
        assert!(aprox(pearson_correlation(&x, &y).unwrap(), 1.0));

        let y2 = [6.0, 4.0, 2.0];
        assert!(aprox(pearson_correlation(&x, &y2).unwrap(), -1.0));
    }

    #[test]
    fn correlacao_com_variancia_zero_e_none() {
        // y constante → correlação indefinida.
        assert_eq!(
            pearson_correlation(&[1.0, 2.0, 3.0], &[5.0, 5.0, 5.0]),
            None
        );
        // x constante.
        assert_eq!(pearson_correlation(&[2.0, 2.0], &[1.0, 2.0]), None);
    }

    #[test]
    fn correlacao_com_comprimentos_diferentes_e_none() {
        assert_eq!(pearson_correlation(&[1.0, 2.0], &[1.0]), None);
    }

    #[test]
    fn correlacao_com_um_par_e_none() {
        assert_eq!(pearson_correlation(&[1.0], &[1.0]), None);
    }

    #[test]
    fn correlacao_descarta_pares_nao_finitos() {
        let x = [1.0, f64::NAN, 3.0];
        let y = [2.0, 99.0, 6.0];
        assert!(aprox(pearson_correlation(&x, &y).unwrap(), 1.0));
    }

    // ===== z_score =====

    #[test]
    fn z_score_normal() {
        assert!(aprox(z_score(75.0, 70.0, 5.0).unwrap(), 1.0));
    }

    #[test]
    fn z_score_com_desvio_zero_ou_nao_finite_e_none() {
        assert_eq!(z_score(1.0, 0.0, 0.0), None);
        assert_eq!(z_score(1.0, 0.0, -1.0), None);
        assert_eq!(z_score(f64::NAN, 0.0, 1.0), None);
        assert_eq!(z_score(1.0, f64::INFINITY, 1.0), None);
    }

    // ===== percent_change =====

    #[test]
    fn percent_change_normal() {
        assert!(aprox(percent_change(100.0, 120.0).unwrap(), 20.0));
        // Base usa |de|: de -100 para -50 → +50%.
        assert!(aprox(percent_change(-100.0, -50.0).unwrap(), 50.0));
    }

    #[test]
    fn percent_change_divisao_por_zero_e_none() {
        assert_eq!(percent_change(0.0, 10.0), None);
        assert_eq!(percent_change(f64::NAN, 10.0), None);
        assert_eq!(percent_change(10.0, f64::INFINITY), None);
    }
}
