// trend.rs
//
// IDENTIFICA TENDÊNCIAS TEMPORAIS.
// Fluxo: recebe amostras → ordena por timestamp → valida valores (rejeita
// não finitos e timestamps duplicados) → converte timestamps em HORAS
// decorridas → monta os pontos (x, y) → chama `linear_regression` de
// statistics.rs → classifica a tendência.
//
// UNIDADES: a inclinação (`slope_per_hour` e `minimum_slope`) é expressa na
// UNIDADE DO INDICADOR POR HORA (ex.: bpm/h para frequência cardíaca).

use chrono::{DateTime, Utc};

use super::statistics::linear_regression;

#[derive(Debug, Clone, PartialEq)]
pub struct TrendResult {
    /// Inclinação em UNIDADES DO INDICADOR POR HORA.
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

#[derive(Debug, Clone)]
pub struct TrendConfig {
    /// Quantidade mínima de amostras (distintas e válidas) para analisar.
    pub minimum_samples: usize,
    /// Limiar de classificação, em UNIDADES DO INDICADOR POR HORA:
    /// |inclinação| > minimum_slope define crescente/decrescente.
    pub minimum_slope: f64,
    /// Período mínimo que a série precisa cobrir (s). `0` não exige período.
    /// Evita declarar tendência com base numa janela temporal muito curta.
    pub minimum_period_seconds: i64,
}

impl Default for TrendConfig {
    fn default() -> Self {
        Self {
            minimum_samples: 2,
            minimum_slope: 0.0,
            minimum_period_seconds: 0,
        }
    }
}

/// Versão com configuração completa. Devolve `None` quando a análise não é
/// possível (amostras insuficientes, período insuficiente ou série
/// degenerada).
pub fn analyze_trend_com_configuracao(
    samples: &[(DateTime<Utc>, f64)],
    config: &TrendConfig,
) -> Option<TrendResult> {
    // Valores não finitos são rejeitados (não entram na regressão).
    let mut samples: Vec<(DateTime<Utc>, f64)> = samples
        .iter()
        .copied()
        .filter(|&(_, v)| v.is_finite())
        .collect();

    // Ordena pelos timestamps.
    samples.sort_by_key(|&(ts, _)| ts);

    // Timestamps duplicados: mantém a primeira ocorrência (o sort é
    // estável), descartando as repetições.
    samples.dedup_by(|a, b| a.0 == b.0);

    if samples.len() < config.minimum_samples.max(2) {
        return None;
    }

    let first_timestamp = samples.first()?.0;
    let last_timestamp = samples.last()?.0;
    let periodo_segundos = (last_timestamp - first_timestamp).num_seconds();
    if periodo_segundos < config.minimum_period_seconds {
        return None;
    }

    let points: Vec<(f64, f64)> = samples
        .iter()
        .map(|(timestamp, value)| {
            let elapsed_hours =
                (*timestamp - first_timestamp).num_milliseconds() as f64 / 3_600_000.0;
            (elapsed_hours, *value)
        })
        .collect();

    let (slope, intercept) = linear_regression(&points)?;

    let direction = if slope > config.minimum_slope {
        TrendDirection::Increasing
    } else if slope < -config.minimum_slope {
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

/// Versão simplificada preservada por compatibilidade: só define o limiar
/// de inclinação (na unidade do indicador por hora) e usa o resto padrão.
pub fn analyze_trend(samples: &[(DateTime<Utc>, f64)], minimum_slope: f64) -> Option<TrendResult> {
    analyze_trend_com_configuracao(
        samples,
        &TrendConfig {
            minimum_slope,
            ..TrendConfig::default()
        },
    )
}

// ============================================================
// TENDÊNCIA COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::{buscar_serie_do_indicador, parse_momento};

/// Analisa a tendência de um indicador fisiológico buscando a série
/// temporal direto do repositório (janela `[inicio_utc, fim_utc]` opcional).
/// Amostras insuficientes, período insuficiente ou série vazia devolvem `None`.
pub async fn analyze_trend_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    config: &TrendConfig,
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
        .filter_map(|amostra| {
            let momento = parse_momento(&amostra.momento_utc)?;
            let valor = amostra.valor?;
            valor.is_finite().then_some((momento, valor))
        })
        .collect();

    Ok(analyze_trend_com_configuracao(&samples, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0).unwrap()
    }

    fn serie_de(valores: &[f64], passo_segundos: i64) -> Vec<(DateTime<Utc>, f64)> {
        valores
            .iter()
            .enumerate()
            .map(|(i, &v)| (ts(i as i64 * passo_segundos), v))
            .collect()
    }

    #[test]
    fn tendencia_crescente() {
        // +10 por hora exatamente.
        let serie = serie_de(&[70.0, 80.0, 90.0], 3600);
        let r = analyze_trend(&serie, 1.0).unwrap();
        assert_eq!(r.direction, TrendDirection::Increasing);
        assert!((r.slope_per_hour - 10.0).abs() < 1e-9);
        assert_eq!(r.first_value, 70.0);
        assert_eq!(r.last_value, 90.0);
    }

    #[test]
    fn tendencia_decrescente() {
        let serie = serie_de(&[90.0, 80.0, 70.0], 3600);
        let r = analyze_trend(&serie, 1.0).unwrap();
        assert_eq!(r.direction, TrendDirection::Decreasing);
        assert!((r.slope_per_hour + 10.0).abs() < 1e-9);
    }

    #[test]
    fn tendencia_estavel_abaixo_do_limite() {
        // +0.5 por hora com limite de 1.0 → estável.
        let serie = serie_de(&[70.0, 70.5, 71.0], 3600);
        let r = analyze_trend(&serie, 1.0).unwrap();
        assert_eq!(r.direction, TrendDirection::Stable);
    }

    #[test]
    fn timestamps_fora_de_ordem_sao_ordenados() {
        let mut serie = serie_de(&[70.0, 80.0, 90.0], 3600);
        serie.swap(0, 2);
        let r = analyze_trend(&serie, 1.0).unwrap();
        assert_eq!(r.direction, TrendDirection::Increasing);
        assert_eq!(r.first_value, 70.0, "ordena antes de calcular");
        assert_eq!(r.last_value, 90.0);
    }

    #[test]
    fn timestamps_duplicados_sao_descartados() {
        let mut serie = serie_de(&[70.0, 80.0, 90.0], 3600);
        // Repetição do primeiro timestamp com outro valor.
        serie.insert(1, (ts(0), 200.0));
        let r = analyze_trend(&serie, 1.0).unwrap();
        assert_eq!(r.direction, TrendDirection::Increasing);
        assert!((r.slope_per_hour - 10.0).abs() < 1e-9);
    }

    #[test]
    fn valores_nao_finitos_sao_rejeitados() {
        let mut serie = serie_de(&[70.0, f64::NAN, 90.0], 3600);
        serie[1].1 = f64::INFINITY;
        let r = analyze_trend(&serie, 1.0).unwrap();
        assert_eq!(r.direction, TrendDirection::Increasing);
    }

    #[test]
    fn poucas_amostras_ou_vazias_e_none() {
        assert_eq!(analyze_trend(&[], 0.0), None);
        assert_eq!(analyze_trend(&[(ts(0), 70.0)], 0.0), None);

        let cfg = TrendConfig {
            minimum_samples: 5,
            ..Default::default()
        };
        assert_eq!(
            analyze_trend_com_configuracao(&serie_de(&[1.0, 2.0, 3.0], 5), &cfg),
            None
        );
    }

    #[test]
    fn periodo_insuficiente_e_none() {
        let serie = serie_de(&[70.0, 80.0, 90.0], 60); // 2 minutos no total
        let cfg = TrendConfig {
            minimum_period_seconds: 3600,
            ..TrendConfig::default()
        };
        assert_eq!(analyze_trend_com_configuracao(&serie, &cfg), None);
    }

    #[test]
    fn minimo_de_amostras_configuravel() {
        let serie = serie_de(&[1.0, 2.0, 3.0], 5);
        let cfg = TrendConfig {
            minimum_samples: 3,
            ..TrendConfig::default()
        };
        assert!(analyze_trend_com_configuracao(&serie, &cfg).is_some());
        let cfg = TrendConfig {
            minimum_samples: 4,
            ..TrendConfig::default()
        };
        assert!(analyze_trend_com_configuracao(&serie, &cfg).is_none());
    }

    #[test]
    fn todos_os_timestamps_iguais_e_none() {
        let serie = vec![(ts(0), 70.0), (ts(0), 71.0), (ts(0), 72.0)];
        // Duplicados reduzem a série a 1 ponto → None.
        assert_eq!(analyze_trend(&serie, 0.0), None);
    }
}
