use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TelemetriaFisiologica {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub timestamp_utc: String,
    pub frequencia_cardiaca_bpm: Option<f64>,
    pub spo2_percentual: Option<f64>,
    pub temperatura_corporal_c: Option<f64>,
    pub temperatura_pele_c: Option<f64>,
    pub taxa_respiratoria_rpm: Option<f64>,
    pub hrv_ms: Option<f64>,
    pub pressao_sistolica_mmhg: Option<f64>,
    pub pressao_diastolica_mmhg: Option<f64>,
    pub glicose_mg_dl: Option<f64>,
    pub origem_medicao: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegistroDiario {
    pub id: String,
    pub astronauta_id: String,
    pub data_registro: String,
    pub horas_sono_total: Option<f64>,
    pub qualidade_sono_score: Option<i64>,
    pub exercicio_resistencia_min: Option<i64>,
    pub exercicio_aerobico_min: Option<i64>,
    pub ingestao_agua_ml: Option<i64>,
    pub calorias_consumidas: Option<i64>,
    pub score_teste_cognitivo: Option<f64>,
    pub nivel_estresse_subjetivo: Option<i64>,
    pub humor_score: Option<i64>,
    pub fadiga_score: Option<i64>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AmbienteNave {
    pub id: String,
    pub modulo_id: String,
    pub timestamp_utc: String,
    pub co2_ppm: Option<f64>,
    pub radiacao_msv_hora: Option<f64>,
    pub temperatura_cabine_c: Option<f64>,
    pub pressao_kpa: Option<f64>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PresencaModulo {
    pub id: String,
    pub astronauta_id: String,
    pub modulo_id: String,
    pub inicio_utc: String,
    pub fim_utc: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExameClinico {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub fase_missao: Option<String>,
    pub data_coleta: String,
    pub categoria: String,
    pub resultados_json: String,
    pub observacoes_medicas: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BaselineFisiologico {
    pub id: String,
    pub astronauta_id: String,
    pub indicador: String,
    pub fase_missao: String,
    pub missao_id: Option<String>,
    pub valor_referencia: f64,
    pub desvio_padrao: Option<f64>,
    pub periodo_inicio: Option<String>,
    pub periodo_fim: Option<String>,
    pub quantidade_amostras: Option<i64>,
    pub atualizado_em: String,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}
