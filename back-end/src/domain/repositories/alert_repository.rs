use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::domain::entities::alert_entity::AlertaSaude;

/// Projeção da Query 7 — quantidade de alertas por nível de risco.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContagemNivelRisco {
    pub nivel_risco: String,
    pub quantidade: i64,
}

#[async_trait]
pub trait AlertRepository: Send + Sync {
    /// Query 7.1 — busca um alerta de saúde pelo `id`.
    async fn find_by_id(&self, id: &str) -> Result<Option<AlertaSaude>, sqlx::Error>;

    /// Query 7.2 — lista alertas com filtros opcionais e paginação.
    async fn list(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
        nivel_risco: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<AlertaSaude>, sqlx::Error>;

    /// Query 7.3 — lista os alertas mais recentes de um astronauta.
    async fn list_recent_by_astronaut(
        &self,
        astronauta_id: &str,
        limite: i64,
    ) -> Result<Vec<AlertaSaude>, sqlx::Error>;

    /// Query 7.4 — conta os alertas agrupados por nível de risco.
    async fn count_by_risk_level(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
    ) -> Result<Vec<ContagemNivelRisco>, sqlx::Error>;
}
