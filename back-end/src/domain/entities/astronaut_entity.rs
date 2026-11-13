use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Astronaut {
    pub id: String,
    pub nome: String,
    pub data_nascimento: String,
    pub tipo_sanguineo: Option<String>,
    pub peso_baseline_kg: Option<f64>,
    pub altura_cm: Option<f64>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}