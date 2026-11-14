/*
CASO DE USO: ORIENTAÇÃO DA IA COPILOTO EXPLICATIVA (OPCIONAL).

Recebe a SolicitacaoOrientacao JÁ PRODUZIDA pelas regras determinísticas
(nível de risco imutável), consulta o Ollama quando configurado e
PERSISTE apenas texto explicativo (`explicacao`/`orientacao`) na
avaliação e no alerta correspondente.

A IA NUNCA:
- altera o nível de risco, o resumo ou as evidências;
- cria, altera ou resolve alertas;
- bloqueia o monitoramento: indisponibilidade devolve uma
  RespostaOrientacao CONTROLADA (`disponivel = false`).

A execução é protegida por uma trava (`AtomicBool`): o MOTOR spawna a
tarefa em background e a trava garante que no máximo UMA orientação
esteja em curso — chamadas concorrentes são simplesmente ignoradas.
*/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{SecondsFormat, Utc};

use crate::domain::repositories::alert_repository::AlertRepository;
use crate::domain::repositories::evaluation_repository::EvaluationRepository;
use crate::infrastructure::ai::client::{ErroIa, OllamaClient};
use crate::infrastructure::ai::models::{
    DisponibilidadeIa, RespostaOrientacao, SolicitacaoOrientacao, interpretar_resposta,
};
use crate::shared::errors::{AppErro, AppResultado};

pub struct GeradorOrientacao {
    cliente: Option<OllamaClient>,
    evaluations: Arc<dyn EvaluationRepository>,
    alertas: Arc<dyn AlertRepository>,
    /// `true` enquanto uma orientação está em curso.
    em_execucao: Arc<AtomicBool>,
}

impl GeradorOrientacao {
    pub fn novo(
        cliente: Option<OllamaClient>,
        evaluations: Arc<dyn EvaluationRepository>,
        alertas: Arc<dyn AlertRepository>,
    ) -> Self {
        Self {
            cliente,
            evaluations,
            alertas,
            em_execucao: Arc::new(AtomicBool::new(false)),
        }
    }

    /// `true` quando há cliente configurado (IA ativa por configuração).
    pub fn disponibilidade_configurada(&self) -> bool {
        self.cliente.is_some()
    }

    /// Trava compartilhada (usada pelo helper de spawn).
    pub fn em_execucao(&self) -> bool {
        self.em_execucao.load(Ordering::SeqCst)
    }

    /// Consulta a IA (quando configurada) e devolve uma resposta CONTROLADA.
    /// Este método NUNCA persiste e NUNCA devolve erro: falha da IA vira
    /// `disponivel = false` com o motivo — o sistema segue funcionando.
    pub async fn gerar(&self, solicitacao: &SolicitacaoOrientacao) -> RespostaOrientacao {
        let Some(cliente) = &self.cliente else {
            return RespostaOrientacao {
                disponivel: false,
                disponibilidade: DisponibilidadeIa::Desativada,
                resumo: None,
                explicacao: None,
                orientacao: None,
            };
        };

        let prompt = crate::infrastructure::ai::models::construir_prompt(solicitacao);
        match cliente.gerar(&prompt).await {
            Ok(texto) => {
                let (resumo, limitacoes, recomendacoes) = interpretar_resposta(&texto);
                // `explicacao` = limitações da análise (seção LIMITACOES);
                // sem seções reconhecíveis, o texto bruto é preservado.
                let explicacao = limitacoes
                    .or_else(|| (!texto.trim().is_empty()).then(|| texto.trim().to_string()));
                RespostaOrientacao {
                    disponivel: true,
                    disponibilidade: DisponibilidadeIa::Disponivel,
                    resumo,
                    explicacao,
                    orientacao: recomendacoes,
                }
            }
            Err(erro) => {
                let motivo = descricao_erro(&erro);
                tracing::warn!("orientação da IA indisponível: {motivo}");
                RespostaOrientacao {
                    disponivel: false,
                    disponibilidade: DisponibilidadeIa::Indisponivel(motivo),
                    resumo: None,
                    explicacao: None,
                    orientacao: None,
                }
            }
        }
    }

    /// Gera a orientação e PERSISTE o texto em `avaliacoes_saude` e no
    /// alerta da condição (quando existir). Falhas de persistência são
    /// devolvidas como erro (o texto não se perde silenciosamente).
    pub async fn processar(
        &self,
        solicitacao: &SolicitacaoOrientacao,
        alerta_id: Option<&str>,
    ) -> AppResultado<RespostaOrientacao> {
        let resposta = self.gerar(solicitacao).await;

        if !resposta.disponivel {
            return Ok(resposta); // nada a persistir; indisponibilidade não bloqueia
        }

        let agora = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let explicacao = resposta.explicacao.as_deref();
        let orientacao = resposta.orientacao.as_deref();

        self.evaluations
            .atualizar_orientacao(&solicitacao.avaliacao_id, explicacao, orientacao, &agora)
            .await
            .map_err(AppErro::Repositorio)?;

        if let Some(alerta_id) = alerta_id {
            self.alertas
                .atualizar_orientacao_alerta(alerta_id, explicacao, orientacao, &agora)
                .await
                .map_err(AppErro::Repositorio)?;
        }

        Ok(resposta)
    }
}

fn descricao_erro(erro: &ErroIa) -> String {
    match erro {
        ErroIa::Timeout => "tempo limite excedido".to_string(),
        outro => outro.to_string(),
    }
}

// ============================================================
// TAREFA EM BACKGROUND (spawna da pelo MOTOR)
// ============================================================

/// Spawn NÃO sobreposto da orientação:
/// - IA desativada por configuração → `false` (nada a fazer);
/// - orientação já em curso → `false` (a nova execução é ignorada);
/// - caso contrário → spawna `tarefa_orientacao` e devolve `true`.
///
/// A trava é liberada pela própria tarefa ao terminar (com ou sem erro).
pub fn spawn_orientacao(
    gerador: Arc<GeradorOrientacao>,
    solicitacao: &SolicitacaoOrientacao,
    alerta_id: Option<String>,
) -> bool {
    if !gerador.disponibilidade_configurada() {
        return false;
    }
    if gerador
        .em_execucao
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return false;
    }

    let tarefa_solicitacao = solicitacao.clone();
    let tarefa_alerta_id = alerta_id;
    tokio::spawn(async move {
        if let Err(erro) =
            tarefa_orientacao(gerador.clone(), tarefa_solicitacao, tarefa_alerta_id).await
        {
            tracing::warn!("orientação da IA não persistida: {erro}");
        }
        gerador.em_execucao.store(false, Ordering::SeqCst);
    });
    true
}

async fn tarefa_orientacao(
    gerador: Arc<GeradorOrientacao>,
    solicitacao: SolicitacaoOrientacao,
    alerta_id: Option<String>,
) -> AppResultado<RespostaOrientacao> {
    gerador.processar(&solicitacao, alerta_id.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::domain::evaluation::rules::{EstadoIndicador, EvidenciaIndicador};
    use crate::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
    use crate::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;

    fn solicitacao() -> SolicitacaoOrientacao {
        SolicitacaoOrientacao {
            astronauta_id: "A1".into(),
            avaliacao_id: "AV1".into(),
            timestamp_utc: "2025-01-15T08:00:00Z".into(),
            nivel_risco: "ATENCAO".into(),
            inconclusivo: false,
            resumo_deterministico: "resumo determinístico".into(),
            justificativas: vec!["SPO2: variação nível Atencao (z=-2.20).".into()],
            evidencias: vec![EvidenciaIndicador {
                indicador: "SPO2".into(),
                estado: EstadoIndicador::Analisado,
                nivel_variacao: Some("Atencao".into()),
                z_score: Some(-2.2),
                valor_atual: Some(91.0),
                valor_referencia: Some(98.0),
                direcao: Some("Abaixo".into()),
                tendencia: Some("Estavel".into()),
                persistente: Some(false),
                baseline_desatualizado: None,
                qualidade_aceitavel: true,
                amostras_validas: 5,
                amostras_ausentes: 0,
                amostras_invalidas: 0,
                justificativa: String::new(),
            }],
            fatores_contextuais: None,
        }
    }

    async fn repos() -> (
        Arc<dyn EvaluationRepository>,
        Arc<dyn AlertRepository>,
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
        sqlx::query(
            "INSERT INTO avaliacoes_saude (id, astronauta_id, timestamp_utc, nivel_risco,
                resumo, indicadores_analisados_json)
             VALUES ('AV1', 'A1', '2025-01-15T08:00:00Z', 'ATENCAO', 'resumo determinístico', '[]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        (
            Arc::new(SqliteEvaluationRepository::new(pool.clone())),
            Arc::new(SqliteAlertRepository::new(pool.clone())),
            pool,
        )
    }

    fn gerador_sem_ia(
        evaluations: Arc<dyn EvaluationRepository>,
        alertas: Arc<dyn AlertRepository>,
    ) -> GeradorOrientacao {
        GeradorOrientacao::novo(None, evaluations, alertas)
    }

    #[tokio::test]
    async fn ia_desativada_devolve_resposta_controlada() {
        let (evaluations, alertas, pool) = repos().await;
        let gerador = gerador_sem_ia(evaluations, alertas);

        let resposta = gerador.gerar(&solicitacao()).await;
        assert!(!resposta.disponivel);
        assert_eq!(resposta.disponibilidade, DisponibilidadeIa::Desativada);
        assert!(resposta.explicacao.is_none() && resposta.orientacao.is_none());

        // `processar` também é um caminho seguro sem IA.
        let resposta = gerador.processar(&solicitacao(), None).await.unwrap();
        assert!(!resposta.disponivel);

        // Nada foi escrito na avaliação.
        let explicacao: Option<String> =
            sqlx::query_scalar("SELECT explicacao FROM avaliacoes_saude WHERE id = 'AV1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(explicacao, None);
    }

    #[tokio::test]
    async fn ia_indisponivel_nao_bloqueia_e_nao_persiste() {
        let (evaluations, alertas, pool) = repos().await;
        // Porta reservada (1): conexão falha rápido, sem Ollama real.
        let cliente = OllamaClient::novo("http://127.0.0.1:1", "modelo-teste", 2).unwrap();
        let gerador = GeradorOrientacao::novo(Some(cliente), evaluations, alertas);

        let resposta = gerador.gerar(&solicitacao()).await;
        assert!(!resposta.disponivel);
        assert!(matches!(
            resposta.disponibilidade,
            DisponibilidadeIa::Indisponivel(_)
        ));

        // processar devolve Ok (indisponibilidade NÃO é erro do fluxo).
        let resposta = gerador.processar(&solicitacao(), None).await.unwrap();
        assert!(!resposta.disponivel);

        let explicacao: Option<String> =
            sqlx::query_scalar("SELECT explicacao FROM avaliacoes_saude WHERE id = 'AV1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(explicacao, None, "nada persistido sem IA");
    }

    #[tokio::test]
    async fn spawn_orientacao_ignora_ia_desativada() {
        let (evaluations, alertas, _pool) = repos().await;
        let gerador = Arc::new(gerador_sem_ia(evaluations, alertas));

        assert!(
            !spawn_orientacao(gerador.clone(), &solicitacao(), None),
            "sem cliente configurado não há tarefa"
        );
        assert!(!gerador.em_execucao());
    }

    #[tokio::test]
    async fn spawn_orientacao_libera_a_trava_ao_terminar() {
        let (evaluations, alertas, _pool) = repos().await;
        let cliente = OllamaClient::novo("http://127.0.0.1:1", "modelo-teste", 2).unwrap();
        let gerador = Arc::new(GeradorOrientacao::novo(Some(cliente), evaluations, alertas));

        assert!(spawn_orientacao(gerador.clone(), &solicitacao(), None));
        assert!(gerador.em_execucao(), "trava ativa durante a execução");

        // Aguarda o fim das tarefas pendentes (a IA falha rápido).
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(!gerador.em_execucao(), "trava liberada após a tarefa");

        // Segunda chamada volta a ser aceita.
        assert!(spawn_orientacao(gerador.clone(), &solicitacao(), None));
    }
}
