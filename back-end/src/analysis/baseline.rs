/*
CALCULA O PADRÃO INDIVIDUAL DO ASTRONAUTA (valor_referencia + desvio_padrao).

RECEBE AMOSTRAS JÁ APROVADAS PELO data_quality.rs, REMOVE ATÍPICOS
(mediana ± k·MAD escalado) E CALCULA MÉDIA/DESVIO VIA statistics.rs.

CONTEXTO: o baseline é sempre associado a astronauta + missão + indicador +
fase da missão (`BaselineFisiologico`); dados de contextos diferentes nunca
são misturados — a seleção da série já é feita por contexto no repositório.

ATUALIZAÇÃO: o campo `atualizado_em` permite identificar baselines antigos.
O baseline NÃO deve ser recalculado a cada coleta (janela de 5 s); use
`baseline_desatualizado` para aplicar uma política de atualização periódica.
*/

use chrono::{DateTime, SecondsFormat, Utc};
use uuid::Uuid;

use crate::domain::entities::measurement_entity::BaselineFisiologico;
use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::data_quality::{
    DataQualityConfig, DataQualityResult, JanelaTempo, evaluate_data_quality_de_repositorio,
};
use super::repository::{FaseMissao, Indicador, parse_momento};
use super::statistics::{mean, median, scaled_mad, standard_deviation};

#[derive(Debug, Clone)]
pub struct BaselineConfig {
    /// Fator k do filtro de atípicos (mediana ± k·MAD escalado).
    /// Valores usuais estão entre 2.5 e 3.5; `0.0` desativa o filtro.
    pub k_mad: f64,
    /// Mínimo de amostras que precisam RESTAR após a filtragem para que o
    /// baseline seja calculado. Média/desvio de poucas amostras filtradas
    /// não representam o padrão individual.
    pub minimum_samples_after_filter: usize,
}

impl BaselineConfig {
    pub fn validar(&self) -> Result<(), String> {
        if !self.k_mad.is_finite() || self.k_mad < 0.0 {
            return Err(format!(
                "k_mad deve ser finito e >= 0.0 (recebido {})",
                self.k_mad
            ));
        }
        if self.minimum_samples_after_filter == 0 {
            return Err("minimum_samples_after_filter deve ser >= 1".to_string());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum BaselineErro {
    Repositorio(sqlx::Error),
    QualidadeInaceitavel(DataQualityResult),
    /// Série vazia ou sem nenhum valor utilizável.
    SemAmostras,
    /// Restaram menos amostras que `minimum_samples_after_filter`.
    AmostrasInsuficientes {
        apos_filtro: usize,
        minimo: usize,
    },
    ConfigInvalida(String),
}

impl From<sqlx::Error> for BaselineErro {
    fn from(e: sqlx::Error) -> Self {
        Self::Repositorio(e)
    }
}

fn iso(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Mantém as amostras em mediana ± k·MAD_escalado.
///
/// COM MAD = 0 A FILTRAGEM NÃO É APLICADA (falta de dispersão): eliminar
/// tudo deixaria o baseline impossível, então todas as amostras são
/// mantidas e o fato fica registrado nesta documentação e nos testes.
/// Com `k_mad = 0` o filtro também é desativado explicitamente.
fn remover_atipicos(amostras: &[(DateTime<Utc>, f64)], k_mad: f64) -> Vec<(DateTime<Utc>, f64)> {
    let valores: Vec<f64> = amostras.iter().map(|(_, v)| *v).collect();

    match (median(&valores), scaled_mad(&valores)) {
        (Some(centro), Some(mad)) if mad > 0.0 && k_mad > 0.0 => amostras
            .iter()
            .copied()
            .filter(|(_, v)| (v - centro).abs() <= k_mad * mad)
            .collect(),
        _ => amostras.to_vec(),
    }
}

/// Cálculo puro: assume amostras já validadas pelo `data_quality`.
///
/// Devolve `Err` quando:
/// - a configuração for incoerente (`ConfigInvalida`);
/// - não houver amostras (`SemAmostras`);
/// - restarem menos amostras que `minimum_samples_after_filter` após a
///   remoção de atípicos (`AmostrasInsuficientes`) — um baseline não pode
///   ser calculado com quantidade insuficiente de amostras.
pub fn calcular_baseline(
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    fase: FaseMissao,
    amostras_validas: &[(DateTime<Utc>, f64)],
    config: &BaselineConfig,
    atualizado_em: DateTime<Utc>,
) -> Result<BaselineFisiologico, BaselineErro> {
    config.validar().map_err(BaselineErro::ConfigInvalida)?;

    let tipicas = remover_atipicos(amostras_validas, config.k_mad);
    if tipicas.len() < config.minimum_samples_after_filter {
        return Err(BaselineErro::AmostrasInsuficientes {
            apos_filtro: tipicas.len(),
            minimo: config.minimum_samples_after_filter,
        });
    }

    let valores: Vec<f64> = tipicas.iter().map(|(_, v)| *v).collect();

    let media = mean(&valores).ok_or(BaselineErro::SemAmostras)?;
    let inicio = tipicas
        .iter()
        .map(|(ts, _)| *ts)
        .min()
        .ok_or(BaselineErro::SemAmostras)?;
    let fim = tipicas
        .iter()
        .map(|(ts, _)| *ts)
        .max()
        .ok_or(BaselineErro::SemAmostras)?;
    let agora = iso(atualizado_em);

    Ok(BaselineFisiologico {
        id: Uuid::new_v4().to_string(),
        astronauta_id: astronauta_id.to_string(),
        indicador: indicador.nome().to_string(),
        fase_missao: fase.nome().to_string(),
        missao_id: missao_id.map(str::to_string),
        valor_referencia: media,
        desvio_padrao: standard_deviation(&valores),
        periodo_inicio: Some(iso(inicio)),
        periodo_fim: Some(iso(fim)),
        quantidade_amostras: Some(tipicas.len() as i64),
        atualizado_em: agora.clone(),
        updated_at: Some(agora),
        deleted_at: None,
    })
}

/// Coleta (repository) → validação (data_quality) → cálculo.
pub async fn calcular_baseline_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    fase: FaseMissao,
    janela: JanelaTempo,
    qualidade: &DataQualityConfig,
    config: &BaselineConfig,
    atualizado_em: DateTime<Utc>,
) -> Result<BaselineFisiologico, BaselineErro> {
    let resultado = evaluate_data_quality_de_repositorio(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        Some(janela),
        qualidade,
    )
    .await?;

    if !resultado.acceptable_quality {
        return Err(BaselineErro::QualidadeInaceitavel(resultado));
    }

    calcular_baseline(
        astronauta_id,
        missao_id,
        indicador,
        fase,
        &resultado.amostras_validas,
        config,
        atualizado_em,
    )
}

// ============================================================
// POLÍTICA DE ATUALIZAÇÃO
// ============================================================

/// Idade do baseline a partir do campo `atualizado_em`.
/// `None` quando o campo não puder ser interpretado.
pub fn idade_do_baseline(
    baseline: &BaselineFisiologico,
    agora: DateTime<Utc>,
) -> Option<chrono::Duration> {
    parse_momento(&baseline.atualizado_em).map(|ts| agora - ts)
}

/// Política configurável de atualização: `true` quando o baseline é mais
/// antigo que `max_idade`. Baseline com `atualizado_em` ilegível é tratado
/// como desatualizado (não confiável) — melhor recalcular do que comparar
/// com uma referência de idade desconhecida.
pub fn baseline_desatualizado(
    baseline: &BaselineFisiologico,
    agora: DateTime<Utc>,
    max_idade: chrono::Duration,
) -> bool {
    match idade_do_baseline(baseline, agora) {
        Some(idade) => idade > max_idade,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0).unwrap()
    }

    fn amostras(valores: &[f64]) -> Vec<(DateTime<Utc>, f64)> {
        valores
            .iter()
            .enumerate()
            .map(|(i, &v)| (ts(i as i64 * 5), v))
            .collect()
    }

    fn config() -> BaselineConfig {
        BaselineConfig {
            k_mad: 3.0,
            minimum_samples_after_filter: 2,
        }
    }

    #[test]
    fn baseline_normal() {
        let valores = amostras(&[70.0, 71.0, 72.0, 73.0, 74.0]);
        let b = calcular_baseline(
            "A1",
            Some("M1"),
            Indicador::FrequenciaCardiaca,
            FaseMissao::PreVoo,
            &valores,
            &config(),
            ts(1000),
        )
        .unwrap();

        assert_eq!(b.astronauta_id, "A1");
        assert_eq!(b.missao_id.as_deref(), Some("M1"));
        assert_eq!(b.indicador, "FREQUENCIA_CARDIACA");
        assert_eq!(b.fase_missao, "PRE_VOO");
        assert!(b.desvio_padrao.unwrap() > 0.0);
        assert_eq!(b.quantidade_amostras, Some(5));
        assert_eq!(b.atualizado_em, "2023-11-14T22:30:00Z");
    }

    #[test]
    fn valores_atipicos_sao_removidos_antes_da_media() {
        // 70±1 com um outlier de 200 (k=3, MAD=1 → limite 70±3).
        let valores = amostras(&[69.0, 70.0, 70.0, 71.0, 200.0]);
        let b = calcular_baseline(
            "A1",
            None,
            Indicador::FrequenciaCardiaca,
            FaseMissao::PreVoo,
            &valores,
            &config(),
            ts(1000),
        )
        .unwrap();

        assert_eq!(b.quantidade_amostras, Some(4), "outlier removido");
        assert!((b.valor_referencia - 70.0).abs() < 1e-9);
    }

    #[test]
    fn mad_zero_nao_elimina_amostras() {
        // Todos os valores iguais: MAD = 0 → filtragem não aplicada.
        let valores = amostras(&[72.0; 4]);
        let b = calcular_baseline(
            "A1",
            None,
            Indicador::Spo2,
            FaseMissao::PreVoo,
            &valores,
            &config(),
            ts(1000),
        )
        .unwrap();

        assert_eq!(b.quantidade_amostras, Some(4), "MAD 0 → sem filtragem");
        assert_eq!(b.valor_referencia, 72.0);
        // Desvio-padrão de valores idênticos é zero.
        assert_eq!(b.desvio_padrao, Some(0.0));
    }

    #[test]
    fn amostras_insuficientes_apos_filtro_sao_rejeitadas() {
        // 5 amostras: o outlier 200 é removido (MAD escalado 1.48, k=3 →
        // limite 70±4.45), restando 4; exigindo 5 restantes o cálculo deve
        // falhar (não calcula baseline com amostras de mais removidas).
        let valores = amostras(&[69.0, 70.0, 70.0, 71.0, 200.0]);
        let cfg = BaselineConfig {
            k_mad: 3.0,
            minimum_samples_after_filter: 5,
        };

        match calcular_baseline(
            "A1",
            None,
            Indicador::FrequenciaCardiaca,
            FaseMissao::PreVoo,
            &valores,
            &cfg,
            ts(1000),
        ) {
            Err(BaselineErro::AmostrasInsuficientes {
                apos_filtro,
                minimo,
            }) => {
                assert_eq!(apos_filtro, 4);
                assert_eq!(minimo, 5);
            }
            _ => panic!("deveria falhar por amostras insuficientes"),
        }
    }

    #[test]
    fn serie_vazia_ou_sem_finitos_e_rejeitada() {
        assert!(
            calcular_baseline(
                "A1",
                None,
                Indicador::Spo2,
                FaseMissao::PreVoo,
                &[],
                &config(),
                ts(1000),
            )
            .is_err()
        );
    }

    #[test]
    fn configuracao_invalida_e_rejeitada() {
        let cfg = BaselineConfig {
            k_mad: -1.0,
            minimum_samples_after_filter: 2,
        };
        match calcular_baseline(
            "A1",
            None,
            Indicador::Spo2,
            FaseMissao::PreVoo,
            &amostras(&[1.0, 2.0]),
            &cfg,
            ts(1000),
        ) {
            Err(BaselineErro::ConfigInvalida(_)) => {}
            _ => panic!("deveria rejeitar k_mad negativo"),
        }

        let cfg = BaselineConfig {
            k_mad: 3.0,
            minimum_samples_after_filter: 0,
        };
        assert!(
            calcular_baseline(
                "A1",
                None,
                Indicador::Spo2,
                FaseMissao::PreVoo,
                &amostras(&[1.0, 2.0]),
                &cfg,
                ts(1000),
            )
            .is_err()
        );
    }

    #[test]
    fn politica_de_atualizacao_usa_atualizado_em() {
        let mut b = calcular_baseline(
            "A1",
            None,
            Indicador::Spo2,
            FaseMissao::PreVoo,
            &amostras(&[70.0, 71.0]),
            &config(),
            ts(1000),
        )
        .unwrap();

        // Recém-calculado: dentro da idade máxima de 1h.
        assert!(!baseline_desatualizado(
            &b,
            ts(1000 + 3600),
            chrono::Duration::hours(1)
        ));

        // 2 horas depois: desatualizado.
        assert!(baseline_desatualizado(
            &b,
            ts(1000 + 7200),
            chrono::Duration::hours(1)
        ));

        // `atualizado_em` ilegível → tratado como desatualizado.
        b.atualizado_em = "data-invalida".to_string();
        assert!(baseline_desatualizado(
            &b,
            ts(1000),
            chrono::Duration::hours(1)
        ));
        assert_eq!(idade_do_baseline(&b, ts(1000)), None);
    }
}
