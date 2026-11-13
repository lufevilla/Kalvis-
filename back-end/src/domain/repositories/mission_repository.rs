use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::domain::entities::mission_entity::{Missao, ModuloNave};

/// Projeção da Query 1.5: participação de um astronauta com os dados da missão.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ParticipacaoAstronauta {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: String,
    pub funcao: Option<String>,
    pub nome_missao: String,
    pub data_lancamento: Option<String>,
    pub data_retorno: Option<String>,
}

#[async_trait]
pub trait MissionRepository: Send + Sync {
    /// Query 1.3 — busca uma missão pelo `id`, ignorando registros removidos logicamente.
    async fn find_by_id(&self, id: &str) -> Result<Option<Missao>, sqlx::Error>;

    /// Query 1.4 — lista missões com paginação (`limite` / `deslocamento`).
    async fn list(&self, limite: i64, deslocamento: i64) -> Result<Vec<Missao>, sqlx::Error>;

    /// Query 1.5 — lista as participações de um astronauta, com os dados da missão.
    async fn list_participations_by_astronaut(
        &self,
        astronauta_id: &str,
    ) -> Result<Vec<ParticipacaoAstronauta>, sqlx::Error>;

    /// Query 1.8 — lista os módulos da nave de uma missão.
    async fn list_modules_by_mission(
        &self,
        missao_id: &str,
    ) -> Result<Vec<ModuloNave>, sqlx::Error>;
}
