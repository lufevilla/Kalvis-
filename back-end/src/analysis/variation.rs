/*
COMPARA O ESTADO ATUAL DO ASTRONAUTA COM O SEU BASELINE INDIVIDUAL.

RECEBE AMOSTRAS JÁ APROVADAS PELO data_quality.rs E UM BASELINE PRONTO.
CALCULA DIFERENÇA ABSOLUTA, PERCENTUAL E Z-SCORE VIA statistics.rs,
E CLASSIFICA A VARIAÇÃO.

INTERPRETAÇÃO DO Z-SCORE (importante):
O z-score aqui é apenas um ESCORE DE DESVIO da média da janela atual em
relação à referência individual do próprio astronauta (baseline):
    z = (média atual − valor_referencia) / desvio_padrao
Ele descreve quantos desvios-padrão do PRÓPRIO histórico a janela atual
está. NÃO é um diagnóstico e NÃO corresponde a nenhum limite clínico
universal — a interpretação dos níveis (Atenção/Crítico) é política
estatística configurável em `VariationConfig`, e a decisão clínica é
responsabilidade da camada de Evaluation.
*/

use chrono::{DateTime, Utc};

use crate::domain::entities::measurement_entity::BaselineFisiologico;
use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::baseline::baseline_desatualizado;
use super::data_quality::{
    DataQualityConfig, DataQualityResult, JanelaTempo, evaluate_data_quality_de_repositorio,
};
use super::repository::Indicador;
use super::statistics::{mean, percent_change, z_score};

#[derive(Debug, Clone, Default)]
pub struct VariationConfig {
    /// |z| acima disso = Atenção.
    pub z_atencao: f64,
    /// |z| acima disso = Crítico.
    pub z_critico: f64,
    /// Política de atualização: idade máxima aceita para o baseline
    /// (medida a partir de `atualizado_em`). `None` desativa a verificação.
    pub max_idade_baseline: Option<chrono::Duration>,
}

impl VariationConfig {
    /// Rejeita configurações incoerentes: valores não finitos, limites
    /// negativos e `z_critico` que não seja estritamente maior que
    /// `z_atencao` (senão "Crítico" nunca seria alcançado).
    pub fn validar(&self) -> Result<(), String> {
        if !self.z_atencao.is_finite() {
            return Err("z_atencao deve ser finito".to_string());
        }
        if !self.z_critico.is_finite() {
            return Err("z_critico deve ser finito".to_string());
        }
        if self.z_atencao < 0.0 {
            return Err(format!(
                "z_atencao deve ser >= 0.0 (recebido {})",
                self.z_atencao
            ));
        }
        if self.z_critico <= self.z_atencao {
            return Err(format!(
                "z_critico ({}) deve ser maior que z_atencao ({})",
                self.z_critico, self.z_atencao
            ));
        }
        Ok(())
    }
}

/// Contexto atual da avaliação: usado para conferir a identidade do
/// baseline (astronauta, missão e fase) antes de comparar dados.
#[derive(Debug, Clone, Copy)]
pub struct ContextoAtual<'a> {
    pub astronauta_id: &'a str,
    pub missao_id: Option<&'a str>,
    /// Fase da missão do contexto atual, quando conhecida.
    pub fase_missao: Option<&'a str>,
    /// Momento da avaliação (para a política de idade do baseline).
    pub agora: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelVariacao {
    Normal,
    Atencao,
    Critico,
    /// Baseline sem desvio padrão utilizável: não dá para calcular o z-score.
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
    /// Desvio da média da janela atual em relação ao baseline individual
    /// (ver a interpretação no topo do arquivo).
    pub z_score: Option<f64>,
    pub direcao: Direcao,
    pub nivel: NivelVariacao,
    /// Presente quando `max_idade_baseline` está configurado:
    /// `Some(true)` = baseline mais antigo que a política permite.
    pub baseline_desatualizado: Option<bool>,
    pub quantidade_amostras: usize,
    pub periodo_inicio: DateTime<Utc>,
    pub periodo_fim: DateTime<Utc>,
}

#[derive(Debug)]
pub enum VariacaoErro {
    Repositorio(sqlx::Error),
    QualidadeInaceitavel(DataQualityResult),
    IndicadorDiferente {
        baseline: String,
        solicitado: String,
    },
    /// Baseline de outro astronauta, missão ou fase — dados de contextos
    /// diferentes nunca são comparados.
    ContextoDivergente {
        campo: &'static str,
        baseline: String,
        solicitado: Option<String>,
    },
    /// Valor de referência ou configuração não utilizáveis.
    ConfigInvalida(String),
    SemAmostras,
}

impl From<sqlx::Error> for VariacaoErro {
    fn from(e: sqlx::Error) -> Self {
        Self::Repositorio(e)
    }
}

fn divergente(campo: &'static str, baseline: &str, solicitado: Option<&str>) -> VariacaoErro {
    VariacaoErro::ContextoDivergente {
        campo,
        baseline: baseline.to_string(),
        solicitado: solicitado.map(str::to_string),
    }
}

/// Valida a identidade do baseline contra o contexto atual.
///
/// Regras (documentadas):
/// - `astronauta_id` SEMPRE precisa ser idêntico (baseline é individual);
/// - `indicador` precisa ser o mesmo;
/// - `missao_id`: divergente quando AMBOS os lados estão definidos e são
///   diferentes. Um baseline de missão específica nunca é comparado com
///   dados de outra missão;
/// - fase da missão: conferida apenas quando o contexto informa a fase.
fn validar_identidade(
    contexto: &ContextoAtual,
    indicador: Indicador,
    baseline: &BaselineFisiologico,
) -> Result<(), VariacaoErro> {
    if baseline.astronauta_id != contexto.astronauta_id {
        return Err(divergente(
            "astronauta_id",
            &baseline.astronauta_id,
            Some(contexto.astronauta_id),
        ));
    }
    if baseline.indicador != indicador.nome() {
        return Err(VariacaoErro::IndicadorDiferente {
            baseline: baseline.indicador.clone(),
            solicitado: indicador.nome().to_string(),
        });
    }
    if let (Some(bas), Some(atual)) = (&baseline.missao_id, contexto.missao_id)
        && bas != atual
    {
        return Err(divergente("missao_id", bas, Some(atual)));
    }
    if let Some(fase) = contexto.fase_missao
        && baseline.fase_missao != fase
    {
        return Err(divergente("fase_missao", &baseline.fase_missao, Some(fase)));
    }
    Ok(())
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
    contexto: ContextoAtual,
    indicador: Indicador,
    baseline: &BaselineFisiologico,
    amostras_validas: &[(DateTime<Utc>, f64)],
    config: &VariationConfig,
) -> Result<VariacaoResultado, VariacaoErro> {
    config.validar().map_err(VariacaoErro::ConfigInvalida)?;
    validar_identidade(&contexto, indicador, baseline)?;

    let valores: Vec<f64> = amostras_validas.iter().map(|(_, v)| *v).collect();
    let atual = mean(&valores).ok_or(VariacaoErro::SemAmostras)?;
    let inicio = amostras_validas
        .iter()
        .map(|(t, _)| *t)
        .min()
        .ok_or(VariacaoErro::SemAmostras)?;
    let fim = amostras_validas
        .iter()
        .map(|(t, _)| *t)
        .max()
        .ok_or(VariacaoErro::SemAmostras)?;

    let referencia = baseline.valor_referencia;
    if !referencia.is_finite() {
        return Err(VariacaoErro::ConfigInvalida(
            "valor_referencia do baseline não é finito".to_string(),
        ));
    }

    let diferenca = atual - referencia;
    let z = baseline
        .desvio_padrao
        .and_then(|dp| z_score(atual, referencia, dp));

    let baseline_desatualizado = config
        .max_idade_baseline
        .map(|max_idade| baseline_desatualizado(baseline, contexto.agora, max_idade));

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
        baseline_desatualizado,
        quantidade_amostras: valores.len(),
        periodo_inicio: inicio,
        periodo_fim: fim,
    })
}

/// Coleta (repository) → validação (data_quality) → comparação.
pub async fn calcular_variacao_de_repositorio(
    repo: &dyn MeasurementRepository,
    contexto: ContextoAtual<'_>,
    missao_id: Option<&str>,
    indicador: Indicador,
    baseline: &BaselineFisiologico,
    janela_atual: JanelaTempo,
    qualidade: &DataQualityConfig,
    config: &VariationConfig,
) -> Result<VariacaoResultado, VariacaoErro> {
    let resultado = evaluate_data_quality_de_repositorio(
        repo,
        contexto.astronauta_id,
        contexto.missao_id,
        indicador,
        Some(janela_atual),
        qualidade,
    )
    .await?;

    if !resultado.acceptable_quality {
        return Err(VariacaoErro::QualidadeInaceitavel(resultado));
    }

    calcular_variacao(
        contexto,
        indicador,
        baseline,
        &resultado.amostras_validas,
        config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0).unwrap()
    }

    fn baseline(
        indicador: &str,
        astronauta: &str,
        missao: Option<&str>,
        dp: Option<f64>,
    ) -> BaselineFisiologico {
        BaselineFisiologico {
            id: "B1".to_string(),
            astronauta_id: astronauta.to_string(),
            indicador: indicador.to_string(),
            fase_missao: "PRE_VOO".to_string(),
            missao_id: missao.map(str::to_string),
            valor_referencia: 70.0,
            desvio_padrao: dp,
            periodo_inicio: None,
            periodo_fim: None,
            quantidade_amostras: Some(10),
            atualizado_em: ts(0).to_rfc3339(),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn amostras(valores: &[f64]) -> Vec<(DateTime<Utc>, f64)> {
        valores
            .iter()
            .enumerate()
            .map(|(i, &v)| (ts(i as i64 * 5), v))
            .collect()
    }

    fn config() -> VariationConfig {
        VariationConfig {
            z_atencao: 2.0,
            z_critico: 3.0,
            max_idade_baseline: None,
        }
    }

    fn contexto<'a>(
        astronauta: &'a str,
        missao: Option<&'a str>,
        fase: Option<&'a str>,
        agora_segundos: i64,
    ) -> ContextoAtual<'a> {
        ContextoAtual {
            astronauta_id: astronauta,
            missao_id: missao,
            fase_missao: fase,
            agora: ts(agora_segundos),
        }
    }

    #[test]
    fn variacao_normal_dentro_do_z() {
        // média 71, referência 70, dp 5 → z = 0.2 → Normal.
        let b = baseline("FREQUENCIA_CARDIACA", "A1", Some("M1"), Some(5.0));
        let r = calcular_variacao(
            contexto("A1", Some("M1"), Some("PRE_VOO"), 1000),
            Indicador::FrequenciaCardiaca,
            &b,
            &amostras(&[70.0, 72.0]),
            &config(),
        )
        .unwrap();

        assert_eq!(r.nivel, NivelVariacao::Normal);
        assert_eq!(r.direcao, Direcao::Acima);
        assert_eq!(r.z_score, Some(0.2));
        assert_eq!(r.baseline_desatualizado, None);
    }

    #[test]
    fn variacao_acima_da_referencia_dispara_atencao_e_critico() {
        let b = baseline("FREQUENCIA_CARDIACA", "A1", None, Some(5.0));

        // média 81 → z = 2.2 → Atenção.
        let atencao = calcular_variacao(
            contexto("A1", None, None, 1000),
            Indicador::FrequenciaCardiaca,
            &b,
            &amostras(&[80.0, 82.0]),
            &config(),
        )
        .unwrap();
        assert_eq!(atencao.nivel, NivelVariacao::Atencao);
        assert_eq!(atencao.direcao, Direcao::Acima);

        // média 87 → z = 3.4 → Crítico.
        let critico = calcular_variacao(
            contexto("A1", None, None, 1000),
            Indicador::FrequenciaCardiaca,
            &b,
            &amostras(&[86.0, 88.0]),
            &config(),
        )
        .unwrap();
        assert_eq!(critico.nivel, NivelVariacao::Critico);
    }

    #[test]
    fn variacao_abaixo_da_referencia() {
        // SpO2 com referência individual de 98% e dp 2%.
        let mut b = baseline("SPO2", "A1", None, Some(2.0));
        b.valor_referencia = 98.0;

        let r = calcular_variacao(
            contexto("A1", None, None, 1000),
            Indicador::Spo2,
            &b,
            &amostras(&[90.0, 92.0]),
            &config(),
        )
        .unwrap();

        assert_eq!(r.direcao, Direcao::Abaixo);
        assert_eq!(r.z_score, Some(-3.5));
        assert_eq!(r.nivel, NivelVariacao::Critico);
        // (91 - 98)/|98| × 100 ≈ -7.14%.
        assert!((r.diferenca_percentual.unwrap() + 7.142857142857143).abs() < 1e-9);
    }

    #[test]
    fn desvio_padrao_zero_ou_ausente_e_indeterminado() {
        let b_dp_zero = baseline("SPO2", "A1", None, Some(0.0));
        let r = calcular_variacao(
            contexto("A1", None, None, 1000),
            Indicador::Spo2,
            &b_dp_zero,
            &amostras(&[98.0, 98.0]),
            &config(),
        )
        .unwrap();
        assert_eq!(r.nivel, NivelVariacao::Indeterminado);
        assert_eq!(r.z_score, None);

        let b_sem_dp = baseline("SPO2", "A1", None, None);
        let r = calcular_variacao(
            contexto("A1", None, None, 1000),
            Indicador::Spo2,
            &b_sem_dp,
            &amostras(&[98.0, 98.0]),
            &config(),
        )
        .unwrap();
        assert_eq!(r.nivel, NivelVariacao::Indeterminado);
    }

    #[test]
    fn baseline_de_outro_astronauta_e_rejeitado() {
        let b = baseline("SPO2", "OUTRO", None, Some(2.0));
        match calcular_variacao(
            contexto("A1", None, None, 1000),
            Indicador::Spo2,
            &b,
            &amostras(&[98.0]),
            &config(),
        ) {
            Err(VariacaoErro::ContextoDivergente { campo, .. }) => {
                assert_eq!(campo, "astronauta_id")
            }
            _ => panic!("deveria rejeitar baseline de outro astronauta"),
        }
    }

    #[test]
    fn baseline_de_outra_missao_e_rejeitado() {
        let b = baseline("SPO2", "A1", Some("M2"), Some(2.0));
        match calcular_variacao(
            contexto("A1", Some("M1"), None, 1000),
            Indicador::Spo2,
            &b,
            &amostras(&[98.0]),
            &config(),
        ) {
            Err(VariacaoErro::ContextoDivergente { campo, .. }) => assert_eq!(campo, "missao_id"),
            _ => panic!("deveria rejeitar baseline de outra missão"),
        }
    }

    #[test]
    fn fase_divergente_e_rejeitada() {
        let b = baseline("SPO2", "A1", None, Some(2.0)); // fase PRE_VOO
        match calcular_variacao(
            contexto("A1", None, Some("EM_VOO"), 1000),
            Indicador::Spo2,
            &b,
            &amostras(&[98.0]),
            &config(),
        ) {
            Err(VariacaoErro::ContextoDivergente { campo, .. }) => assert_eq!(campo, "fase_missao"),
            _ => panic!("deveria rejeitar fase divergente"),
        }
    }

    #[test]
    fn indicador_incompativel_e_rejeitado() {
        let b = baseline("SPO2", "A1", None, Some(2.0));
        assert!(matches!(
            calcular_variacao(
                contexto("A1", None, None, 1000),
                Indicador::FrequenciaCardiaca,
                &b,
                &amostras(&[98.0]),
                &config(),
            ),
            Err(VariacaoErro::IndicadorDiferente { .. })
        ));
    }

    #[test]
    fn configuracao_incoerente_e_rejeitada() {
        let b = baseline("SPO2", "A1", None, Some(2.0));
        let cfg = VariationConfig {
            z_atencao: 3.0,
            z_critico: 2.0,
            max_idade_baseline: None,
        };
        assert!(
            calcular_variacao(
                contexto("A1", None, None, 1000),
                Indicador::Spo2,
                &b,
                &amostras(&[98.0]),
                &cfg,
            )
            .is_err()
        );

        let cfg = VariationConfig {
            z_atencao: f64::NAN,
            z_critico: 3.0,
            max_idade_baseline: None,
        };
        assert!(
            calcular_variacao(
                contexto("A1", None, None, 1000),
                Indicador::Spo2,
                &b,
                &amostras(&[98.0]),
                &cfg,
            )
            .is_err()
        );
    }

    #[test]
    fn idade_do_baseline_e_reportada_conforme_a_politica() {
        // Baseline atualizado_em = ts(0); agora = ts(7200) → 2h de idade.
        let b = baseline("SPO2", "A1", None, Some(2.0));
        let cfg = VariationConfig {
            z_atencao: 2.0,
            z_critico: 3.0,
            max_idade_baseline: Some(chrono::Duration::hours(1)),
        };
        let r = calcular_variacao(
            contexto("A1", None, None, 7200),
            Indicador::Spo2,
            &b,
            &amostras(&[98.0, 98.0]),
            &cfg,
        )
        .unwrap();
        assert_eq!(r.baseline_desatualizado, Some(true), "2h > 1h de política");

        // Sem política → None.
        let r = calcular_variacao(
            contexto("A1", None, None, 7200),
            Indicador::Spo2,
            &b,
            &amostras(&[98.0, 98.0]),
            &config(),
        )
        .unwrap();
        assert_eq!(r.baseline_desatualizado, None);
    }

    #[test]
    fn serie_vazia_e_rejeitada() {
        let b = baseline("SPO2", "A1", None, Some(2.0));
        assert!(matches!(
            calcular_variacao(
                contexto("A1", None, None, 1000),
                Indicador::Spo2,
                &b,
                &[],
                &config(),
            ),
            Err(VariacaoErro::SemAmostras)
        ));
    }
}
