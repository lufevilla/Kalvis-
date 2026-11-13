use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncQueue {
    pub id: String,
    pub tabela: String,
    pub registro_id: String,
    pub operacao: String,
    pub criado_em: String,
    pub tentativas: Option<i64>,
    pub ultimo_erro: Option<String>,
}