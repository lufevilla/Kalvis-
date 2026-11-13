use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::domain::entities::measurement_entity::{
    AmbienteNave, BaselineFisiologico, ExameClinico, PresencaModulo, RegistroDiario,
    TelemetriaFisiologica,
};

/// Projeção da Query 2 — comparação das medições de telemetria com o baseline fisiológico.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ComparacaoBaseline {
    pub indicador: String,
    pub valor_baseline: Option<f64>,
    pub desvio_padrao_baseline: Option<f64>,
    pub media_observada: Option<f64>,
    pub minimo_observado: Option<f64>,
    pub maximo_observado: Option<f64>,
    pub quantidade_amostras: Option<i64>,
    pub desvio_medio: Option<f64>,
    pub desvio_em_z: Option<f64>,
}

/// Projeção da Query 3.1 — contexto diário de uma medição de telemetria.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContextoMedicao {
    pub telemetria_id: String,
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
    pub registro_diario_id: Option<String>,
    pub data_registro: Option<String>,
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
    pub medicoes_no_dia: Option<i64>,
}

/// Projeção da Query 3.2 — ponto de série temporal de um indicador fisiológico.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PontoSerie {
    pub momento_utc: String,
    pub valor: f64,
}

/// Projeção da Query 3.3 — resumo estatístico de um indicador fisiológico.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResumoIndicador {
    pub indicador: String,
    pub media: Option<f64>,
    pub minimo: Option<f64>,
    pub maximo: Option<f64>,
    pub quantidade_amostras: Option<i64>,
    pub dias_com_registros: Option<i64>,
    /// Variância populacional calculada em SQL (`E[x²] - E[x]²`).
    pub variancia: Option<f64>,
}

impl ResumoIndicador {
    /// Desvio-padrão populacional derivado da variância (o SQLite empacotado
    /// pelo sqlx não possui a função `sqrt`).
    pub fn desvio_padrao(&self) -> Option<f64> {
        self.variancia.and_then(|v| {
            if v >= 0.0 {
                Some(v.sqrt())
            } else {
                None
            }
        })
    }
}

/// Projeção da Query 4 — resumo diário da telemetria correlacionado ao registro diário.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResumoDiarioTelemetria {
    pub data_referencia: String,
    pub medicoes_no_dia: i64,
    pub primeira_medicao_utc: String,
    pub ultima_medicao_utc: String,
    pub media_frequencia_cardiaca: Option<f64>,
    pub media_spo2: Option<f64>,
    pub media_temperatura_corporal: Option<f64>,
    pub media_hrv: Option<f64>,
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
}

/// Projeção da Query 5 — resumo ambiental de um módulo da nave.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResumoAmbientalModulo {
    pub modulo_id: String,
    pub nome_modulo: String,
    pub missao_id: String,
    pub nome_missao: String,
    pub quantidade_amostras: Option<i64>,
    pub media_co2_ppm: Option<f64>,
    pub max_co2_ppm: Option<f64>,
    pub media_radiacao_msv_hora: Option<f64>,
    pub max_radiacao_msv_hora: Option<f64>,
    pub media_temperatura_cabine_c: Option<f64>,
    pub min_temperatura_cabine_c: Option<f64>,
    pub max_temperatura_cabine_c: Option<f64>,
    pub media_pressao_kpa: Option<f64>,
    pub min_pressao_kpa: Option<f64>,
    pub max_pressao_kpa: Option<f64>,
    pub primeira_amostra_utc: Option<String>,
    pub ultima_amostra_utc: Option<String>,
}

/// Projeção da Query 6 — painel atual do astronauta: última telemetria, último registro
/// diário e presença atual em módulo.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PainelAtual {
    pub telemetria_id: String,
    pub ultima_telemetria_utc: String,
    pub frequencia_cardiaca_bpm: Option<f64>,
    pub spo2_percentual: Option<f64>,
    pub temperatura_corporal_c: Option<f64>,
    pub temperatura_pele_c: Option<f64>,
    pub taxa_respiratoria_rpm: Option<f64>,
    pub hrv_ms: Option<f64>,
    pub pressao_sistolica_mmhg: Option<f64>,
    pub pressao_diastolica_mmhg: Option<f64>,
    pub glicose_mg_dl: Option<f64>,
    pub missao_id: Option<String>,
    pub nome_missao: Option<String>,
    pub registro_diario_id: Option<String>,
    pub ultimo_registro_data: Option<String>,
    pub horas_sono_total: Option<f64>,
    pub qualidade_sono_score: Option<i64>,
    pub nivel_estresse_subjetivo: Option<i64>,
    pub humor_score: Option<i64>,
    pub fadiga_score: Option<i64>,
    pub presenca_id: Option<String>,
    pub nome_modulo_atual: Option<String>,
    pub presenca_inicio_utc: Option<String>,
}

#[async_trait]
pub trait MeasurementRepository: Send + Sync {
    // ===== Consultas básicas =====

    /// Query 1.6 — busca uma medição de telemetria fisiológica pelo `id`.
    async fn find_telemetry_by_id(
        &self,
        id: &str,
    ) -> Result<Option<TelemetriaFisiologica>, sqlx::Error>;

    /// Query 1.7 — busca um registro diário pelo `id`.
    async fn find_daily_log_by_id(
        &self,
        id: &str,
    ) -> Result<Option<RegistroDiario>, sqlx::Error>;

    /// Query 1.9 — busca uma medição ambiental pelo `id`.
    async fn find_environment_by_id(
        &self,
        id: &str,
    ) -> Result<Option<AmbienteNave>, sqlx::Error>;

    /// Query 1.10 — lista as presenças de um astronauta em módulos da nave.
    async fn list_presences(&self, astronauta_id: &str)
        -> Result<Vec<PresencaModulo>, sqlx::Error>;

    /// Query 1.11 — lista todos os exames clínicos de um astronauta.
    async fn list_clinical_exams(&self, astronauta_id: &str)
        -> Result<Vec<ExameClinico>, sqlx::Error>;

    /// Query 6.6 — lista os exames clínicos de um astronauta com filtros opcionais.
    async fn list_clinical_exams_filtered(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        fase_missao: Option<&str>,
        categoria: Option<&str>,
        data_inicio: Option<&str>,
        data_fim: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<ExameClinico>, sqlx::Error>;

    /// Query 1.12 — lista os baselines fisiológicos de um astronauta.
    async fn list_baselines(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        fase_missao: Option<&str>,
    ) -> Result<Vec<BaselineFisiologico>, sqlx::Error>;

    // ===== Comparação e Contexto =====

    /// Query 2 — compara as estatísticas observadas de cada indicador de telemetria
    /// com o baseline fisiológico do astronauta na fase de missão informada.
    async fn compare_telemetry_with_baseline(
        &self,
        astronauta_id: &str,
        fase_missao: &str,
        missao_id: Option<&str>,
    ) -> Result<Vec<ComparacaoBaseline>, sqlx::Error>;

    /// Query 3.1 — busca o contexto diário (registro diário do mesmo dia) de uma
    /// medição de telemetria.
    async fn get_daily_context_for_telemetry(
        &self,
        telemetria_id: &str,
    ) -> Result<Option<ContextoMedicao>, sqlx::Error>;

    /// Query 3.2 — lista a série temporal de um indicador fisiológico.
    async fn list_indicator_time_series(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        indicador: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<PontoSerie>, sqlx::Error>;

    /// Query 3.3 — resumo estatístico de um indicador fisiológico.
    async fn get_indicator_summary(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        indicador: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
    ) -> Result<Option<ResumoIndicador>, sqlx::Error>;

    /// Query 4 — resumo diário da telemetria correlacionado aos registros diários.
    async fn list_daily_summaries(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        data_inicio: Option<&str>,
        data_fim: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<ResumoDiarioTelemetria>, sqlx::Error>;

    // ===== Ambiente da nave =====

    /// Query 5.1 — lista as medições ambientais de um módulo, com paginação.
    async fn list_environment_by_module(
        &self,
        modulo_id: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<AmbienteNave>, sqlx::Error>;

    /// Query 5.2 — resumo ambiental por módulo de uma missão.
    async fn get_environmental_summary_by_mission(
        &self,
        missao_id: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
    ) -> Result<Vec<ResumoAmbientalModulo>, sqlx::Error>;

    // ===== Painel =====

    /// Query 6 — painel atual do astronauta: última telemetria, último registro diário
    /// e presença atual em módulo.
    async fn get_current_panel(
        &self,
        astronauta_id: &str,
    ) -> Result<Option<PainelAtual>, sqlx::Error>;
}
