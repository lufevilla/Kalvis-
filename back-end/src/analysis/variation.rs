/*
 AQ ELE VAI PEGAR OS DADOS E FAZER A COMPARAÇÃO DE VARIAÇÃO E VER SE TÃO SEGUROS
*/
/*
COMPARA O ESTADO ATUAL DO ASTRONAUTA COM O SEU BASELINE INDIVIDUAL.

RECEBE AMOSTRAS JÁ APROVADAS PELO data_quality.rs E UM BASELINE PRONTO.
CALCULA DIFERENÇA ABSOLUTA, PERCENTUAL E Z-SCORE VIA statistics.rs,
E CLASSIFICA A VARIAÇÃO.
*/

use chrono::{DateTime, Utc};

use crate::domain::entities::measurement_entity::BaselineFisiologico;
use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::data_quality::{
    evaluate_data_quality_de_repositorio, DataQualityConfig, DataQualityResult, JanelaTempo,
};
use super::repository::Indicador;
use super::statistics::{mean, percent_change, z_score};

#[derive(Debug, Clone)]
pub struct VariationConfig {
    /// |z| acima disso = Atenção.
    pub z_atencao: f64,
    /// |z| acima disso = Crítico.
    pub z_critico: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelVariacao {
    Normal,
    Atencao,
    Critico,
    /// Baseline sem desvio padrão: não dá para calcular o z-score.
    Indeterminado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direcao {
    Acima,
    Abaixo,
    Estavel,
}

#[derive(Debug, Clone)]
pub struct VariacaoResultado {
    pub indicador: Indicador,
    pub valor_atual: f64,
    pub valor_referencia: f64,
    pub diferenca_absoluta: f64,
    pub diferenca_percentual: Option<f64>,
    pub z_score: Option<f64>,
    pub direcao: Direcao,
    pub nivel: NivelVariacao,
    pub quantidade_amostras: usize,
    pub periodo_inicio: DateTime<Utc>,
    pub periodo_fim: DateTime<Utc>,
}

#[derive(Debug)]
pub enum VariacaoErro {
    Repositorio(sqlx::Error),
    QualidadeInaceitavel(DataQualityResult),
    IndicadorDiferente { baseline: String, solicitado: String },
    SemAmostras,
}

impl From<sqlx::Error> for VariacaoErro {
    fn from(e: sqlx::Error) -> Self {
        Self::Repositorio(e)
    }
}

fn classificar(z: Option<f64>, c: &VariationConfig) -> NivelVariacao {
    match z.map(f64::abs) {
        None => NivelVariacao::Indeterminado,
        Some(a) if a > c.z_critico => NivelVariacao::Critico,
        Some(a) if a > c.z_atencao => NivelVariacao::Atencao,
        Some(_) => NivelVariacao::Normal,
    }
}

/// Cálculo puro: assume amostras já validadas pelo `data_quality`.
pub fn calcular_variacao(
    indicador: Indicador,
    baseline: &BaselineFisiologico,
    amostras_validas: &[(DateTime<Utc>, f64)],
    config: &VariationConfig,
) -> Result<VariacaoResultado, VariacaoErro> {
    if baseline.indicador != indicador.nome() {
        return Err(VariacaoErro::IndicadorDiferente {
            baseline: baseline.indicador.clone(),
            solicitado: indicador.nome().to_string(),
        });
    }

    let valores: Vec<f64> = amostras_validas.iter().map(|(_, v)| *v).collect();
    let atual = mean(&valores).ok_or(VariacaoErro::SemAmostras)?;
    let inicio = amostras_validas.iter().map(|(t, _)| *t).min().ok_or(VariacaoErro::SemAmostras)?;
    let fim = amostras_validas.iter().map(|(t, _)| *t).max().ok_or(VariacaoErro::SemAmostras)?;

    let referencia = baseline.valor_referencia;
    let diferenca = atual - referencia;
    let z = baseline.desvio_padrao.and_then(|dp| z_score(atual, referencia, dp));

    let direcao = match diferenca {
        d if d > 0.0 => Direcao::Acima,
        d if d < 0.0 => Direcao::Abaixo,
        _ => Direcao::Estavel,
    };

    Ok(VariacaoResultado {
        indicador,
        valor_atual: atual,
        valor_referencia: referencia,
        diferenca_absoluta: diferenca,
        diferenca_percentual: percent_change(referencia, atual),
        z_score: z,
        direcao,
        nivel: classificar(z, config),
        quantidade_amostras: valores.len(),
        periodo_inicio: inicio,
        periodo_fim: fim,
    })
}

/// Coleta (repository) → validação (data_quality) → comparação.
pub async fn calcular_variacao_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    baseline: &BaselineFisiologico,
    janela_atual: JanelaTempo,
    qualidade: &DataQualityConfig,
    config: &VariationConfig,
) -> Result<VariacaoResultado, VariacaoErro> {
    let resultado = evaluate_data_quality_de_repositorio(
        repo, astronauta_id, missao_id, indicador, Some(janela_atual), qualidade,
    )
    .await?;

    if !resultado.acceptable_quality {
        return Err(VariacaoErro::QualidadeInaceitavel(resultado));
    }

    calcular_variacao(indicador, baseline, &resultado.amostras_validas, config)
}
