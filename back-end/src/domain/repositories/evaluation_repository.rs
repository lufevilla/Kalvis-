/*
REPOSITÓRIO DE AVALIAÇÕES DE SAÚDE (tabela `avaliacoes_saude`).

Responsável pela persistência completa do histórico de avaliações:
- salvar cada avaliação produzida pelas regras determinísticas;
- consultar avaliações anteriores e a mais recente do astronauta;
- consultar por missão e período;
- atualizar `explicacao`/`orientacao` quando a orientação da IA
  (copiloto explicativo) ficar pronta — sem alterar o nível de risco.
*/

use async_trait::async_trait;

use crate::domain::entities::evaluation_entity::AvaliacaoSaude;

#[async_trait]
pub trait EvaluationRepository: Send + Sync {
    /// Persiste uma nova avaliação (INSERT; `id` é gerado pelo chamador).
    async fn salvar(&self, avaliacao: &AvaliacaoSaude) -> Result<(), sqlx::Error>;

    /// Busca uma avaliação pelo `id`, ignorando removidas logicamente.
    async fn buscar_por_id(&self, id: &str) -> Result<Option<AvaliacaoSaude>, sqlx::Error>;

    /// Lista avaliações anteriores do astronauta, em ordem cronológica
    /// DECRESCENTE. `antes_de` (timestamp ISO) limita o corte superior.
    async fn listar_anteriores(
        &self,
        astronauta_id: &str,
        antes_de: Option<&str>,
        limite: i64,
    ) -> Result<Vec<AvaliacaoSaude>, sqlx::Error>;

    /// Avaliação mais recente de um astronauta.
    async fn mais_recente(
        &self,
        astronauta_id: &str,
    ) -> Result<Option<AvaliacaoSaude>, sqlx::Error>;

    /// Avaliações de uma missão dentro de um período (ISO, inclusivo),
    /// ordenadas cronologicamente DECRESCENTE.
    async fn listar_por_missao_e_periodo(
        &self,
        missao_id: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
        limite: i64,
    ) -> Result<Vec<AvaliacaoSaude>, sqlx::Error>;

    /// Preenche `explicacao`/`orientacao` geradas pela orientação da IA.
    /// NÃO altera nível de risco, resumo nem evidências.
    async fn atualizar_orientacao(
        &self,
        id: &str,
        explicacao: Option<&str>,
        orientacao: Option<&str>,
        updated_at: &str,
    ) -> Result<(), sqlx::Error>;
}
