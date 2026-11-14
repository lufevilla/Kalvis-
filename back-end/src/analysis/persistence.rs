/*
DIFERENCIA UM PICO PONTUAL (EX: EXERCICIO) DE UMA AÇÃO QUE PODE SER MAIS GERAL.

A persistência é ESTATÍSTICA: combina o percentual de janelas alteradas com
a maior sequência consecutiva de janelas alteradas. NÃO é diagnóstico
clínico — a interpretação cabe à camada de Evaluation.

Critérios de alteração são configuráveis por indicador (limite inferior,
superior ou faixa aceitável) via `CriterioAlteracao`; janelas têm duração
definida via `PersistenceConfig`.
*/

use chrono::{DateTime, Utc};

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::data_quality::{DataQualityConfig, DataQualityResult, evaluate_data_quality};
use super::repository::{buscar_serie_do_indicador, parse_momento};
use super::statistics::max_consecutive;

#[derive(Debug, Clone, PartialEq)]
pub struct PersistenceResult {
    pub total_windows: usize,
    pub altered_windows: usize,
    pub persistence_percentage: f64,
    pub max_consecutive_windows: usize,
    pub is_persistent: bool,
}

/// Critério de alteração por janela, configurável por indicador.
/// Nenhum indicador é obrigado a usar `valor >= limiar`: alguns exigem
/// limite inferior (ex.: SpO₂ baixa), outros superior (ex.: CO₂ alta) e
/// outros faixa aceitável (ex.: temperatura).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CriterioAlteracao {
    /// Alterada quando o valor ATINGE ou ultrapassa o limiar (`valor >= limiar`).
    AcimaDe(f64),
    /// Alterada quando o valor está no limiar ou abaixo dele (`valor <= limiar`).
    AbaixoDe(f64),
    /// Alterada quando o valor sai da faixa aceitável (`< minimo` ou `> maximo`).
    ForaDaFaixa { minimo: f64, maximo: f64 },
}

impl CriterioAlteracao {
    pub fn atende(&self, valor: f64) -> bool {
        match *self {
            CriterioAlteracao::AcimaDe(limiar) => valor >= limiar,
            CriterioAlteracao::AbaixoDe(limiar) => valor <= limiar,
            CriterioAlteracao::ForaDaFaixa { minimo, maximo } => valor < minimo || valor > maximo,
        }
    }

    /// Rejeita critérios com valores não finitos ou faixa invertida.
    pub fn validar(&self) -> Result<(), String> {
        match *self {
            CriterioAlteracao::AcimaDe(limiar) | CriterioAlteracao::AbaixoDe(limiar) => {
                if !limiar.is_finite() {
                    return Err("limiar deve ser finito".to_string());
                }
            }
            CriterioAlteracao::ForaDaFaixa { minimo, maximo } => {
                if !minimo.is_finite() || !maximo.is_finite() {
                    return Err("limites da faixa devem ser finitos".to_string());
                }
                if minimo >= maximo {
                    return Err(format!(
                        "faixa invertida: minimo ({minimo}) deve ser menor que maximo ({maximo})"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Configuração da avaliação de persistência.
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Duração de cada janela em segundos. `<= 0` trata cada amostra como
    /// uma janela (comportamento original). Com duração definida, as
    /// amostras são agrupadas por período e uma janela é "alterada" quando
    /// TODOS os valores dela atendem ao critério — um pico isolado dentro
    /// da janela não conta como persistência.
    pub janela_segundos: i64,
    /// Percentual mínimo de janelas alteradas (0–100).
    pub min_persistence_percentage: f64,
    /// Maior sequência consecutiva de janelas alteradas exigida.
    pub min_consecutive_windows: usize,
}

impl PersistenceConfig {
    pub fn validar(&self) -> Result<(), String> {
        if !self.min_persistence_percentage.is_finite()
            || !(0.0..=100.0).contains(&self.min_persistence_percentage)
        {
            return Err(format!(
                "min_persistence_percentage deve estar em [0, 100] (recebido {})",
                self.min_persistence_percentage
            ));
        }
        if self.min_consecutive_windows == 0 {
            return Err("min_consecutive_windows deve ser >= 1".to_string());
        }
        Ok(())
    }
}

pub fn evaluate_persistence(
    windows: &[bool],
    min_persistence_percentage: f64,
    min_consecutive_windows: usize,
) -> Option<PersistenceResult> {
    if windows.is_empty() {
        return None;
    }

    let total = windows.len();

    let altered = windows.iter().filter(|&&v| v).count();

    let percentage = (altered as f64 / total as f64) * 100.0;

    let max_streak = max_consecutive(windows);

    let is_persistent =
        percentage >= min_persistence_percentage && max_streak >= min_consecutive_windows;

    Some(PersistenceResult {
        total_windows: total,
        altered_windows: altered,
        persistence_percentage: percentage,
        max_consecutive_windows: max_streak,
        is_persistent,
    })
}

/// Converte a série temporal em janelas booleanas conforme o critério.
///
/// - `janela_segundos <= 0`: cada amostra é uma janela (valor ≥/atende).
/// - `janela_segundos > 0`: amostras agrupadas por período a partir do
///   primeiro timestamp; janela alterada somente quando TODOS os valores
///   dela atendem ao critério. Períodos sem amostras (lacunas) não geram
///   janelas nem quebram sequências.
pub fn janelas_da_serie(
    serie: &[(DateTime<Utc>, f64)],
    criterio: &CriterioAlteracao,
    janela_segundos: i64,
) -> Vec<bool> {
    if janela_segundos <= 0 {
        return serie.iter().map(|&(_, v)| criterio.atende(v)).collect();
    }

    let Some(&primeiro) = serie.first().map(|(ts, _)| ts) else {
        return Vec::new();
    };

    let mut janelas: Vec<Vec<f64>> = Vec::new();
    for &(ts, v) in serie {
        let indice = ((ts - primeiro).num_seconds() / janela_segundos).max(0) as usize;
        if indice >= janelas.len() {
            janelas.resize(indice + 1, Vec::new());
        }
        janelas[indice].push(v);
    }

    janelas
        .into_iter()
        .map(|valores| valores.iter().all(|&v| criterio.atende(v)))
        .collect()
}

// ============================================================
// PERSISTÊNCIA COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

/// Avalia a persistência de um indicador fisiológico buscando a série
/// temporal direto do repositório (ordenada cronologicamente pela query,
/// com nova ordenação de segurança).
///
/// `qualidade: Option<&DataQualityConfig>`: quando informado, a qualidade
/// das amostras é verificada antes da análise; série com qualidade
/// inaceitável devolve `Ok(None)` (não analisável).
/// Série vazia, nome de indicador desconhecido ou critérios inválidos
/// também devolvem `Ok(None)` / `Err`, conforme o caso.
pub async fn evaluate_persistence_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    criterio: &CriterioAlteracao,
    config: &PersistenceConfig,
    qualidade: Option<&DataQualityConfig>,
) -> Result<Option<PersistenceResult>, sqlx::Error> {
    if criterio.validar().is_err() || config.validar().is_err() {
        return Err(sqlx::Error::Protocol(
            "critérios de persistência inválidos (limiar não finito, faixa invertida ou \
             percentuais fora de [0, 100])"
                .to_string(),
        ));
    }

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

    // Qualidade verificada antes de analisar, quando configurada.
    let amostras_validas: Vec<(DateTime<Utc>, f64)> = match qualidade {
        Some(cfg) => {
            let resultado: DataQualityResult = evaluate_data_quality(&serie, None, cfg);
            if !resultado.acceptable_quality {
                return Ok(None);
            }
            resultado.amostras_validas
        }
        None => serie
            .iter()
            .filter_map(|a| {
                let ts = parse_momento(&a.momento_utc)?;
                let v = a.valor?;
                v.is_finite().then_some((ts, v))
            })
            .collect(),
    };

    let mut ordenada = amostras_validas;
    ordenada.sort_by_key(|&(ts, _)| ts);

    let windows = janelas_da_serie(&ordenada, criterio, config.janela_segundos);

    Ok(evaluate_persistence(
        &windows,
        config.min_persistence_percentage,
        config.min_consecutive_windows,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0).unwrap()
    }

    fn serie_de(valores: &[f64], passo: i64) -> Vec<(DateTime<Utc>, f64)> {
        valores
            .iter()
            .enumerate()
            .map(|(i, &v)| (ts(i as i64 * passo), v))
            .collect()
    }

    #[test]
    fn alteracao_pontual_nao_e_persistente() {
        let r = evaluate_persistence(&[false, true, false], 50.0, 2).unwrap();
        assert_eq!(r.total_windows, 3);
        assert_eq!(r.altered_windows, 1);
        assert_eq!(r.max_consecutive_windows, 1);
        assert!(!r.is_persistent);
    }

    #[test]
    fn alteracao_persistente() {
        let r = evaluate_persistence(&[true, true, true, true], 75.0, 3).unwrap();
        assert_eq!(r.altered_windows, 4);
        assert_eq!(r.max_consecutive_windows, 4);
        assert!(r.is_persistent);
    }

    #[test]
    fn sequencias_consecutivas_sao_medidas() {
        // Duas sequências de 2 e uma de 3.
        let r = evaluate_persistence(
            &[true, true, false, true, true, false, true, true, true],
            60.0,
            3,
        )
        .unwrap();
        assert_eq!(r.altered_windows, 7);
        assert_eq!(r.max_consecutive_windows, 3);
        assert!(r.is_persistent);

        // Exigindo 4 consecutivas não seria persistente.
        let r = evaluate_persistence(
            &[true, true, false, true, true, false, true, true, true],
            60.0,
            4,
        )
        .unwrap();
        assert!(!r.is_persistent);
    }

    #[test]
    fn percentual_exigido_e_respeitado() {
        // 2/4 = 50% alteradas.
        let r = evaluate_persistence(&[true, false, true, false], 51.0, 1).unwrap();
        assert!(!r.is_persistent);
        let r = evaluate_persistence(&[true, false, true, false], 50.0, 1).unwrap();
        assert!(r.is_persistent);
    }

    #[test]
    fn serie_vazia_e_none() {
        assert_eq!(evaluate_persistence(&[], 50.0, 1), None);
        assert_eq!(
            janelas_da_serie(&[], &CriterioAlteracao::AcimaDe(1.0), 60),
            Vec::<bool>::new()
        );
    }

    #[test]
    fn criterios_invalidos_sao_rejeitados() {
        assert!(CriterioAlteracao::AcimaDe(f64::NAN).validar().is_err());
        assert!(
            CriterioAlteracao::AbaixoDe(f64::INFINITY)
                .validar()
                .is_err()
        );
        assert!(
            CriterioAlteracao::ForaDaFaixa {
                minimo: 10.0,
                maximo: 5.0
            }
            .validar()
            .is_err()
        );
        assert!(
            CriterioAlteracao::ForaDaFaixa {
                minimo: 5.0,
                maximo: 10.0
            }
            .validar()
            .is_ok()
        );

        let cfg = PersistenceConfig {
            janela_segundos: 0,
            min_persistence_percentage: 150.0,
            min_consecutive_windows: 1,
        };
        assert!(cfg.validar().is_err());
        let cfg = PersistenceConfig {
            janela_segundos: 0,
            min_persistence_percentage: 50.0,
            min_consecutive_windows: 0,
        };
        assert!(cfg.validar().is_err());
    }

    #[test]
    fn criterio_abaixo_e_fora_da_faixa() {
        let criterio = CriterioAlteracao::AbaixoDe(90.0);
        let serie = serie_de(&[98.0, 88.0, 92.0], 5);
        assert_eq!(
            janelas_da_serie(&serie, &criterio, 0),
            vec![false, true, false]
        );

        let criterio = CriterioAlteracao::ForaDaFaixa {
            minimo: 36.0,
            maximo: 37.5,
        };
        let serie = serie_de(&[36.5, 38.0, 35.0], 5);
        assert_eq!(
            janelas_da_serie(&serie, &criterio, 0),
            vec![false, true, true]
        );

        // AcimaDe preserva a semântica original (>=).
        let criterio = CriterioAlteracao::AcimaDe(74.0);
        let serie = serie_de(&[70.0, 74.0, 75.0], 5);
        assert_eq!(
            janelas_da_serie(&serie, &criterio, 0),
            vec![false, true, true]
        );
    }

    #[test]
    fn janelas_com_duracao_definida_agrupam_amostras() {
        // Amostras a cada 5 s, janela de 10 s: [60,61][55,58][70][63].
        let serie = serie_de(&[60.0, 61.0, 55.0, 58.0, 70.0, 63.0], 5);
        let criterio = CriterioAlteracao::AcimaDe(50.0);

        // Cada amostra como janela (janela_segundos <= 0).
        assert_eq!(janelas_da_serie(&serie, &criterio, 0).len(), 6);

        // Janelas de 10 s: TODOS os valores da janela precisam atender.
        let janelas = janelas_da_serie(&serie, &criterio, 10);
        assert_eq!(janelas, vec![true, true, true]);

        // Um único pico dentro da janela não a torna alterada (semântica
        // "todos atendem"): janelas [70,72] e [95,71] com limiar 80.
        let serie = serie_de(&[70.0, 72.0, 95.0, 71.0], 5);
        let janelas = janelas_da_serie(&serie, &CriterioAlteracao::AcimaDe(80.0), 10);
        assert_eq!(janelas, vec![false, false]);
        // Sem agrupamento, o pico apareceria como janela alterada.
        assert_eq!(
            janelas_da_serie(&serie, &CriterioAlteracao::AcimaDe(80.0), 0),
            vec![false, false, true, false]
        );
    }
}
