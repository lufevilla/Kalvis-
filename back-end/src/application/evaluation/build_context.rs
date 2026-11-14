/*
CASO DE USO: CONSTRUÇÃO DO CONTEXTO DA AVALIAÇÃO.

Reúne, de forma centralizada, tudo que as regras determinísticas precisam:
- dados fisiológicos recentes (janela de análise) COM o veredito de
  qualidade de cada indicador (DataQualityResult);
- baselines individuais aplicáveis (missão específica ou sem missão);
- avaliações anteriores relevantes.

DISTINÇÕES EXIGIDAS (nada é convertido em zero):
- dados disponíveis  → amostras válidas no resultado de qualidade;
- dados ausentes     → nenhum registro na janela (total_samples = 0);
- dados inválidos    → rejeitados pela validação (contagens separadas);
- inconclusivos      → qualidade inaceitável ou amostras insuficientes.

A conversão de nomes de indicadores/timestamps é responsabilidade única
de `analysis::repository` — este módulo não espalha conversões.
*/

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::analysis::data_quality::{
    DataQualityConfig, DataQualityResult, JanelaTempo, evaluate_data_quality_de_repositorio,
};
use crate::analysis::repository::{FaseMissao, Indicador};
use crate::domain::entities::evaluation_entity::AvaliacaoSaude;
use crate::domain::entities::measurement_entity::BaselineFisiologico;
use crate::domain::entities::protocol_entity::ParametrosQualidade;
use crate::domain::repositories::evaluation_repository::EvaluationRepository;
use crate::domain::repositories::measurement_repository::MeasurementRepository;
use crate::shared::errors::{AppErro, AppResultado};

pub struct ConstrutorContexto {
    measurements: Arc<dyn MeasurementRepository>,
    evaluations: Arc<dyn EvaluationRepository>,
    /// Duração da janela de análise.
    janela_analise: chrono::Duration,
    /// Quantidade de avaliações anteriores incluídas no contexto.
    limite_anteriores: i64,
    /// Parâmetros de qualidade do protocolo (aplicados sobre os limites
    /// de plausibilidade por indicador de `analysis::data_quality`).
    qualidade: ParametrosQualidade,
}

/// Contexto consolidado de uma avaliação.
#[derive(Debug)]
pub struct ContextoAvaliacao {
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub fase_missao: FaseMissao,
    pub agora: DateTime<Utc>,
    pub janela_atual: JanelaTempo,
    /// Veredito de qualidade por indicador (inclui amostras válidas).
    pub qualidade: HashMap<Indicador, DataQualityResult>,
    /// Amostras aprovadas, ordenadas cronologicamente, por indicador.
    pub amostras_validas: HashMap<Indicador, Vec<(DateTime<Utc>, f64)>>,
    /// Baseline individual aplicável por indicador (quando existente).
    pub baselines: HashMap<Indicador, BaselineFisiologico>,
    /// Avaliações anteriores do astronauta (mais recentes primeiro).
    pub avaliacoes_anteriores: Vec<AvaliacaoSaude>,
    /// Avisos da construção (falhas parciais por indicador etc.).
    pub avisos: Vec<String>,
}

impl ContextoAvaliacao {
    /// Amostras válidas de um indicador, se analisável.
    pub fn amostras(&self, indicador: Indicador) -> Option<&Vec<(DateTime<Utc>, f64)>> {
        self.amostras_validas.get(&indicador)
    }

    /// Indicadores sem NENHUMA amostra na janela (ausência ≠ resultado normal).
    pub fn indicadores_ausentes(&self) -> Vec<Indicador> {
        self.qualidade
            .iter()
            .filter(|(_, r)| r.total_samples == 0)
            .map(|(i, _)| *i)
            .collect()
    }
}

impl ConstrutorContexto {
    pub fn novo(
        measurements: Arc<dyn MeasurementRepository>,
        evaluations: Arc<dyn EvaluationRepository>,
        janela_analise: chrono::Duration,
        limite_anteriores: i64,
        qualidade: ParametrosQualidade,
    ) -> Self {
        Self {
            measurements,
            evaluations,
            janela_analise,
            limite_anteriores,
            qualidade,
        }
    }

    /// Constrói o contexto completo para o momento `agora`.
    pub async fn executar(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        fase_missao: FaseMissao,
        agora: DateTime<Utc>,
    ) -> AppResultado<ContextoAvaliacao> {
        if astronauta_id.trim().is_empty() {
            return Err(AppErro::DadosInvalidos(
                "astronauta_id não pode ser vazio".to_string(),
            ));
        }

        let janela_atual = JanelaTempo {
            inicio: agora - self.janela_analise,
            fim: agora,
        };

        // ===== Dados fisiológicos + qualidade, POR INDICADOR =====
        let mut qualidade = HashMap::new();
        let mut amostras_validas = HashMap::new();
        let mut avisos = Vec::new();

        for indicador in Indicador::TODOS {
            let config = config_qualidade(indicador, &self.qualidade);
            match evaluate_data_quality_de_repositorio(
                self.measurements.as_ref(),
                astronauta_id,
                missao_id,
                indicador,
                Some(janela_atual),
                &config,
            )
            .await
            {
                Ok(resultado) => {
                    amostras_validas.insert(indicador, resultado.amostras_validas.clone());
                    qualidade.insert(indicador, resultado);
                }
                Err(erro) => {
                    // Falha em UM indicador não impede a análise dos demais:
                    // fica registrada como aviso e como ausência de dados.
                    avisos.push(format!(
                        "indicador {}: falha ao recuperar a série ({erro})",
                        indicador.nome()
                    ));
                }
            }
        }

        // ===== Baselines aplicáveis =====
        let mut baselines = HashMap::new();
        for indicador in Indicador::TODOS {
            let mut baseline = self
                .measurements
                .buscar_baseline(
                    astronauta_id,
                    indicador.nome(),
                    fase_missao.nome(),
                    missao_id,
                )
                .await
                .map_err(AppErro::Repositorio)?;

            if baseline.is_none() {
                // Fallback: baseline individual sem missão associada
                // (padrão pré-voo do astronauta).
                baseline = self
                    .measurements
                    .buscar_baseline(astronauta_id, indicador.nome(), fase_missao.nome(), None)
                    .await
                    .map_err(AppErro::Repositorio)?;
            }

            if let Some(b) = baseline {
                baselines.insert(indicador, b);
            }
        }

        // ===== Avaliações anteriores =====
        let avaliacoes_anteriores = self
            .evaluations
            .listar_anteriores(astronauta_id, None, self.limite_anteriores.max(0))
            .await
            .map_err(AppErro::Repositorio)?;

        Ok(ContextoAvaliacao {
            astronauta_id: astronauta_id.to_string(),
            missao_id: missao_id.map(str::to_string),
            fase_missao,
            agora,
            janela_atual,
            qualidade,
            amostras_validas,
            baselines,
            avaliacoes_anteriores,
            avisos,
        })
    }
}

/// Configuração de qualidade: os parâmetros gerais do PROTOCOLO (janela,
/// percentuais, lacuna) são aplicados sobre os limites de PLAUSIBILIDADE
/// DE COLETA por indicador (`data_quality::padrao_para`), que continuam
/// responsabilidade única do módulo de análise. O registro diário
/// (HorasSono) mantém a tolerância longa de lacunas (amostras diárias).
/// Também usada na JANELA DE CÁLCULO do baseline (process_assessment).
pub fn config_qualidade(indicador: Indicador, p: &ParametrosQualidade) -> DataQualityConfig {
    let mut config = DataQualityConfig::padrao_para(indicador);
    config.minimum_samples = p.minimo_amostras;
    config.minimum_valid_percentage = p.minimo_percentual_valido;
    config.max_invalid_percentage = p.maximo_percentual_invalido;
    config.max_noisy_percentage = p.maximo_percentual_ruido;
    if indicador != Indicador::HorasSono {
        config.max_gap_seconds = p.lacuna_maxima_segundos;
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::domain::entities::evaluation_entity::AvaliacaoSaude;
    use crate::domain::entities::protocol_entity::ParametrosQualidade;
    use crate::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;
    use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

    fn qualidade_padrao() -> ParametrosQualidade {
        ParametrosQualidade {
            minimo_amostras: 2,
            minimo_percentual_valido: 80.0,
            maximo_percentual_invalido: 5.0,
            maximo_percentual_ruido: 5.0,
            lacuna_maxima_segundos: 300,
        }
    }

    fn ts(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0).unwrap()
    }

    async fn setup() -> (
        Arc<dyn MeasurementRepository>,
        Arc<dyn EvaluationRepository>,
        SqlitePool,
    ) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("conectar");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        sqlx::query(
            "INSERT INTO astronautas (id, nome, data_nascimento) VALUES ('A1', 'K', '1990-05-01')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Telemetria: 3 amostras de FC e SpO2 em ~15 s.
        sqlx::query(
            "INSERT INTO telemetria_fisiologica
                (id, astronauta_id, timestamp_utc, frequencia_cardiaca_bpm, spo2_percentual)
             VALUES
                ('T1', 'A1', '2023-11-14T22:29:30Z', 70.0, 98.0),
                ('T2', 'A1', '2023-11-14T22:29:35Z', 72.0, 97.5),
                ('T3', 'A1', '2023-11-14T22:29:40Z', 71.0, 98.2)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Uma medição inválida (HRV negativa — fora da plausibilidade).
        sqlx::query(
            "INSERT INTO telemetria_fisiologica (id, astronauta_id, timestamp_utc, hrv_ms)
             VALUES ('T4', 'A1', '2023-11-14T22:29:40Z', -5.0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Baseline individual pré-voo para FC.
        sqlx::query(
            "INSERT INTO baseline_fisiologico
                (id, astronauta_id, indicador, fase_missao, valor_referencia,
                 desvio_padrao, quantidade_amostras, atualizado_em)
             VALUES ('B1', 'A1', 'FREQUENCIA_CARDIACA', 'PRE_VOO', 70.0, 4.0, 50,
                     '2023-11-14T20:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Uma avaliação anterior.
        sqlx::query(
            "INSERT INTO avaliacoes_saude (id, astronauta_id, timestamp_utc, nivel_risco,
                resumo, indicadores_analisados_json)
             VALUES ('AV1', 'A1', '2023-11-14T22:00:00Z', 'NORMAL', 'ok', '[]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        (
            Arc::new(SqliteMeasurementRepository::new(pool.clone())),
            Arc::new(SqliteEvaluationRepository::new(pool.clone())),
            pool,
        )
    }

    #[tokio::test]
    async fn contexto_reune_dados_qualidade_e_baseline() {
        let (measurements, evaluations, _pool) = setup().await;
        let construtor = ConstrutorContexto::novo(
            measurements,
            evaluations,
            chrono::Duration::minutes(5),
            5,
            qualidade_padrao(),
        );

        // 1_700_001_000 = 22:30:00Z — a janela de 5 min cobre as amostras
        // semeadas (22:29:30–22:29:40).
        let agora = ts(1000);
        let contexto = construtor
            .executar("A1", None, FaseMissao::PreVoo, agora)
            .await
            .expect("contexto");

        assert_eq!(
            contexto.janela_atual.inicio,
            agora - chrono::Duration::minutes(5)
        );
        assert_eq!(contexto.avaliacoes_anteriores.len(), 1);
        assert!(contexto.avisos.is_empty());

        // FC: 3 amostras válidas, qualidade aceitável, baseline presente.
        let fc = contexto
            .qualidade
            .get(&Indicador::FrequenciaCardiaca)
            .unwrap();
        assert_eq!(fc.valid_samples, 3);
        assert!(fc.acceptable_quality);
        assert_eq!(
            contexto
                .baselines
                .get(&Indicador::FrequenciaCardiaca)
                .unwrap()
                .id,
            "B1"
        );

        // HRV: medição com valor inválido → rejeitada (não vira zero).
        let hrv = contexto.qualidade.get(&Indicador::Hrv).unwrap();
        assert_eq!(hrv.out_of_range_samples, 1);
        assert_eq!(hrv.valid_samples, 0);

        // Indicador sem nenhuma medição → ausência explícita (total 0).
        let ausentes = contexto.indicadores_ausentes();
        assert!(ausentes.contains(&Indicador::Glicose));
        let glicose = contexto.qualidade.get(&Indicador::Glicose).unwrap();
        assert_eq!(glicose.total_samples, 0);
        assert_eq!(
            contexto.amostras(Indicador::Glicose).unwrap().len(),
            0,
            "ausência NUNCA é convertida em valor zero"
        );
    }

    #[tokio::test]
    async fn avaliacoes_anteriores_sao_incluidas() {
        let (measurements, evaluations, _pool) = setup().await;
        let repo_av = evaluations.clone();
        let construtor = ConstrutorContexto::novo(
            measurements,
            evaluations,
            chrono::Duration::minutes(5),
            5,
            qualidade_padrao(),
        );

        // Registra mais uma avaliação (mais recente).
        repo_av
            .salvar(&AvaliacaoSaude {
                id: "AV2".into(),
                astronauta_id: "A1".into(),
                missao_id: None,
                timestamp_utc: "2023-11-14T22:29:00Z".into(),
                nivel_risco: "ATENCAO".into(),
                resumo: "recente".into(),
                indicadores_analisados_json: "[]".into(),
                fatores_contextuais_json: None,
                explicacao: None,
                orientacao: None,
                updated_at: None,
                deleted_at: None,
            })
            .await
            .unwrap();

        let contexto = construtor
            .executar("A1", None, FaseMissao::PreVoo, ts(0))
            .await
            .unwrap();

        assert_eq!(contexto.avaliacoes_anteriores.len(), 2);
        assert_eq!(
            contexto.avaliacoes_anteriores[0].id, "AV2",
            "mais recente primeiro"
        );
    }

    #[tokio::test]
    async fn astronauta_invalido_e_rejeitado() {
        let (measurements, evaluations, _pool) = setup().await;
        let construtor = ConstrutorContexto::novo(
            measurements,
            evaluations,
            chrono::Duration::minutes(5),
            5,
            qualidade_padrao(),
        );
        let erro = construtor
            .executar(" ", None, FaseMissao::PreVoo, ts(0))
            .await
            .unwrap_err();
        assert!(matches!(erro, AppErro::DadosInvalidos(_)));
    }

    #[tokio::test]
    async fn parametros_do_protocolo_sao_aplicados_a_qualidade() {
        let (measurements, evaluations, _pool) = setup().await;
        // Exigindo mais amostras do que existem → qualidade inaceitável.
        let exigente = ParametrosQualidade {
            minimo_amostras: 50,
            ..qualidade_padrao()
        };
        let construtor = ConstrutorContexto::novo(
            measurements,
            evaluations,
            chrono::Duration::minutes(5),
            5,
            exigente,
        );

        let contexto = construtor
            .executar("A1", None, FaseMissao::PreVoo, ts(1000))
            .await
            .unwrap();

        let fc = contexto
            .qualidade
            .get(&Indicador::FrequenciaCardiaca)
            .unwrap();
        assert_eq!(fc.valid_samples, 3);
        assert!(
            !fc.acceptable_quality,
            "3 amostras < minimo_amostras=50 → inconclusivo, mas amostras seguem registradas"
        );
        assert!(
            contexto
                .amostras(Indicador::FrequenciaCardiaca)
                .unwrap()
                .len()
                == 3,
            "as amostras válidas continuam disponíveis para análise justificada"
        );
    }
}
