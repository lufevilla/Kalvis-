/*
CALCULA O PADRÃO INDIVIDUAL DO ASTRONAUTA (valor_referencia + desvio_padrao).

RECEBE AMOSTRAS JÁ APROVADAS PELO data_quality.rs, REMOVE ATÍPICOS
(mediana ± k·MAD escalado) E CALCULA MÉDIA/DESVIO VIA statistics.rs.
*/

use chrono::{DateTime, SecondsFormat, Utc};
use uuid::Uuid;

use crate::domain::entities::measurement_entity::BaselineFisiologico;
use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::data_quality::{
    evaluate_data_quality_de_repositorio, DataQualityConfig, DataQualityResult, JanelaTempo,
};
use super::repository::{FaseMissao, Indicador};
use super::statistics::{mean, median, scaled_mad, standard_deviation};

#[derive(Debug)]
pub enum BaselineErro {
    Repositorio(sqlx::Error),
    QualidadeInaceitavel(DataQualityResult),
    SemAmostras,
}

impl From<sqlx::Error> for BaselineErro {
    fn from(e: sqlx::Error) -> Self {
        Self::Repositorio(e)
    }
}

fn iso(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Mantém as amostras em mediana ± k·MAD_escalado. Com MAD = 0 não filtra.
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
pub fn calcular_baseline(
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    fase: FaseMissao,
    amostras_validas: &[(DateTime<Utc>, f64)],
    k_mad: f64,
    atualizado_em: DateTime<Utc>,
) -> Option<BaselineFisiologico> {
    let tipicas = remover_atipicos(amostras_validas, k_mad);
    let valores: Vec<f64> = tipicas.iter().map(|(_, v)| *v).collect();

    let media = mean(&valores)?;
    let inicio = tipicas.iter().map(|(ts, _)| *ts).min()?;
    let fim = tipicas.iter().map(|(ts, _)| *ts).max()?;
    let agora = iso(atualizado_em);

    Some(BaselineFisiologico {
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
    k_mad: f64,
    atualizado_em: DateTime<Utc>,
) -> Result<BaselineFisiologico, BaselineErro> {
    let resultado = evaluate_data_quality_de_repositorio(
        repo, astronauta_id, missao_id, indicador, Some(janela), qualidade,
    )
    .await?;

    if !resultado.acceptable_quality {
        return Err(BaselineErro::QualidadeInaceitavel(resultado));
    }

    calcular_baseline(
        astronauta_id, missao_id, indicador, fase,
        &resultado.amostras_validas, k_mad, atualizado_em,
    )
    .ok_or(BaselineErro::SemAmostras)
}
