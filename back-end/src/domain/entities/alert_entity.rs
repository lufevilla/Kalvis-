use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertaSaude {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub timestamp_utc: String,
    pub nivel_risco: String,
    pub resumo: String,
    pub explicacao: Option<String>,
    pub orientacao: Option<String>,
}