use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Missao {
    pub id: String,
    pub nome_missao: String,
    pub data_lancamento: Option<String>,
    pub data_retorno: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ParticipacaoMissao {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: String,
    pub funcao: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModuloNave {
    pub id: String,
    pub missao_id: String,
    pub nome: String,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}
