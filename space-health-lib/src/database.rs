use sqlx::{SqlitePool, Row};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::models::{TrackingVital, TrackingEnvironment, TrackingData};

/// Database connection manager
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Initialize database connection pool
    pub async fn new(db_path: &str) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let database_url = format!("sqlite://{}?mode=rwc", db_path);
        let pool = SqlitePool::connect(&database_url)
            .await
            .context("Failed to connect to SQLite database")?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run database migrations")?;

        info!("Database initialized at: {}", db_path);

        Ok(Self { pool })
    }

    /// Get current panel data for astronaut (latest telemetry + daily log + module)
    pub async fn get_current_panel(&self, astronaut_id: &str) -> Result<Option<CurrentPanel>> {
        let row = sqlx::query(
            r#"
            SELECT
                t.id AS telemetria_id,
                t.timestamp_utc AS ultima_telemetria_utc,
                t.frequencia_cardiaca_bpm,
                t.spo2_percentual,
                t.temperatura_corporal_c,
                t.temperatura_pele_c,
                t.taxa_respiratoria_rpm,
                t.hrv_ms,
                t.pressao_sistolica_mmhg,
                t.pressao_diastolica_mmhg,
                t.glicose_mg_dl,
                t.missao_id,
                m.nome_missao,
                r.id AS registro_diario_id,
                r.data_registro AS ultimo_registro_data,
                r.horas_sono_total,
                r.qualidade_sono_score,
                r.nivel_estresse_subjetivo,
                r.humor_score,
                r.fadiga_score,
                p.id AS presenca_id,
                mn.nome AS nome_modulo_atual,
                p.inicio_utc AS presenca_inicio_utc
            FROM telemetria_fisiologica t
            LEFT JOIN missoes m
                   ON m.id = t.missao_id
                  AND m.deleted_at IS NULL
            LEFT JOIN registros_diarios r
                   ON r.id = (
                       SELECT r2.id
                       FROM registros_diarios r2
                       WHERE r2.astronauta_id = t.astronauta_id
                         AND r2.deleted_at IS NULL
                       ORDER BY r2.data_registro DESC
                       LIMIT 1
                   )
            LEFT JOIN presenca_modulo p
                   ON p.id = (
                       SELECT p2.id
                       FROM presenca_modulo p2
                       WHERE p2.astronauta_id = t.astronauta_id
                         AND p2.fim_utc IS NULL
                         AND p2.deleted_at IS NULL
                       ORDER BY p2.inicio_utc DESC
                       LIMIT 1
                   )
            LEFT JOIN modulos_nave mn
                   ON mn.id = p.modulo_id
                  AND mn.deleted_at IS NULL
            WHERE t.id = (
                SELECT t2.id
                FROM telemetria_fisiologica t2
                WHERE t2.astronauta_id = ?1
                  AND t2.deleted_at IS NULL
                ORDER BY t2.timestamp_utc DESC
                LIMIT 1
            )
            "#,
        )
        .bind(astronaut_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query current panel")?;

        if let Some(row) = row {
            Ok(Some(CurrentPanel {
                telemetria_id: row.get("telemetria_id"),
                ultima_telemetria_utc: row.get("ultima_telemetria_utc"),
                frequencia_cardiaca_bpm: row.get("frequencia_cardiaca_bpm"),
                spo2_percentual: row.get("spo2_percentual"),
                temperatura_corporal_c: row.get("temperatura_corporal_c"),
                temperatura_pele_c: row.get("temperatura_pele_c"),
                taxa_respiratoria_rpm: row.get("taxa_respiratoria_rpm"),
                hrv_ms: row.get("hrv_ms"),
                pressao_sistolica_mmhg: row.get("pressao_sistolica_mmhg"),
                pressao_diastolica_mmhg: row.get("pressao_diastolica_mmhg"),
                glicose_mg_dl: row.get("glicose_mg_dl"),
                missao_id: row.get("missao_id"),
                nome_missao: row.get("nome_missao"),
                registro_diario_id: row.get("registro_diario_id"),
                ultimo_registro_data: row.get("ultimo_registro_data"),
                horas_sono_total: row.get("horas_sono_total"),
                qualidade_sono_score: row.get("qualidade_sono_score"),
                nivel_estresse_subjetivo: row.get("nivel_estresse_subjetivo"),
                humor_score: row.get("humor_score"),
                fadiga_score: row.get("fadiga_score"),
                presenca_id: row.get("presenca_id"),
                nome_modulo_atual: row.get("nome_modulo_atual"),
                presenca_inicio_utc: row.get("presenca_inicio_utc"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get environmental summary by mission
    pub async fn get_environmental_summary_by_mission(
        &self,
        missao_id: &str,
    ) -> Result<Vec<EnvironmentalSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                mn.id AS modulo_id,
                mn.nome AS nome_modulo,
                m.id AS missao_id,
                m.nome_missao AS nome_missao,
                COUNT(a.id) AS quantidade_amostras,
                AVG(a.co2_ppm) AS media_co2_ppm,
                MAX(a.co2_ppm) AS max_co2_ppm,
                AVG(a.radiacao_msv_hora) AS media_radiacao_msv_hora,
                MAX(a.radiacao_msv_hora) AS max_radiacao_msv_hora,
                AVG(a.temperatura_cabine_c) AS media_temperatura_cabine_c,
                MIN(a.temperatura_cabine_c) AS min_temperatura_cabine_c,
                MAX(a.temperatura_cabine_c) AS max_temperatura_cabine_c,
                AVG(a.pressao_kpa) AS media_pressao_kpa,
                MIN(a.pressao_kpa) AS min_pressao_kpa,
                MAX(a.pressao_kpa) AS max_pressao_kpa,
                MIN(a.timestamp_utc) AS primeira_amostra_utc,
                MAX(a.timestamp_utc) AS ultima_amostra_utc
            FROM modulos_nave mn
            JOIN missoes m
                 ON m.id = mn.missao_id
                AND m.deleted_at IS NULL
            LEFT JOIN ambiente_nave a
                   ON a.modulo_id = mn.id
                  AND a.deleted_at IS NULL
            WHERE mn.missao_id = ?1
              AND mn.deleted_at IS NULL
            GROUP BY mn.id, mn.nome, m.id, m.nome_missao
            ORDER BY mn.nome
            "#,
        )
        .bind(missao_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query environmental summary")?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(EnvironmentalSummary {
                modulo_id: row.get("modulo_id"),
                nome_modulo: row.get("nome_modulo"),
                missao_id: row.get("missao_id"),
                nome_missao: row.get("nome_missao"),
                quantidade_amostras: row.get("quantidade_amostras"),
                media_co2_ppm: row.get("media_co2_ppm"),
                max_co2_ppm: row.get("max_co2_ppm"),
                media_radiacao_msv_hora: row.get("media_radiacao_msv_hora"),
                max_radiacao_msv_hora: row.get("max_radiacao_msv_hora"),
                media_temperatura_cabine_c: row.get("media_temperatura_cabine_c"),
                min_temperatura_cabine_c: row.get("min_temperatura_cabine_c"),
                max_temperatura_cabine_c: row.get("max_temperatura_cabine_c"),
                media_pressao_kpa: row.get("media_pressao_kpa"),
                min_pressao_kpa: row.get("min_pressao_kpa"),
                max_pressao_kpa: row.get("max_pressao_kpa"),
                primeira_amostra_utc: row.get("primeira_amostra_utc"),
                ultima_amostra_utc: row.get("ultima_amostra_utc"),
            });
        }

        Ok(summaries)
    }

    /// Build tracking data from panel and environmental summaries
    pub fn build_tracking_data(
        &self,
        panel: CurrentPanel,
        env_summaries: Vec<EnvironmentalSummary>,
        astronaut_id: String,
    ) -> TrackingData {
        let mission_name = panel.nome_missao.clone();
        let current_module = panel.nome_modulo_atual.clone();
        let last_update = panel.ultima_telemetria_utc.clone();

        // Build crew vitals from panel
        let mut crew_vitals = Vec::new();

        // Heart Rate
        if let Some(hr) = panel.frequencia_cardiaca_bpm {
            crew_vitals.push(TrackingVital::new(
                "Frequência Cardíaca".to_string(),
                hr.round().to_string(),
                "bpm".to_string(),
                if hr >= 60.0 && hr <= 100.0 { "normal" } else { "attention" }.to_string(),
                if hr >= 60.0 && hr <= 100.0 { "Normal" } else { "Atenção" }.to_string(),
                Some(40.0),
                Some(140.0),
                Some(60.0),
                Some(100.0),
                None,
                None,
            ));
        }

        // SpO2
        if let Some(spo2) = panel.spo2_percentual {
            crew_vitals.push(TrackingVital::new(
                "Oxigenação (SpO2)".to_string(),
                spo2.round().to_string(),
                "%".to_string(),
                if spo2 >= 95.0 { "normal" } else { "alert" }.to_string(),
                if spo2 >= 95.0 { "Normal" } else { "Alerta" }.to_string(),
                Some(85.0),
                Some(100.0),
                Some(95.0),
                Some(100.0),
                None,
                None,
            ));
        }

        // Temperature
        if let Some(temp) = panel.temperatura_corporal_c {
            crew_vitals.push(TrackingVital::new(
                "Temperatura Corporal".to_string(),
                format!("{:.1}", temp),
                "°C".to_string(),
                if temp >= 36.1 && temp <= 37.2 { "normal" } else { "attention" }.to_string(),
                if temp >= 36.1 && temp <= 37.2 { "Normal" } else { "Atenção" }.to_string(),
                Some(35.0),
                Some(40.0),
                Some(36.1),
                Some(37.2),
                None,
                None,
            ));
        }

        // Fatigue/Stress from daily log
        let fatigue_score = panel.fadiga_score.unwrap_or(0);
        let stress_score = panel.nivel_estresse_subjetivo.unwrap_or(0);
        let fatigue_pct = ((fatigue_score + stress_score) as f64 / 20.0 * 100.0).min(100.0).round() as i64;
        
        crew_vitals.push(TrackingVital::new(
            "Nível de Fadiga / Estresse".to_string(),
            fatigue_pct.to_string(),
            "%".to_string(),
            if fatigue_pct > 70 { "alert" } else if fatigue_pct > 40 { "attention" } else { "normal" }.to_string(),
            if fatigue_pct > 70 { "Alerta" } else if fatigue_pct > 40 { "Atenção" } else { "Normal" }.to_string(),
            Some(0.0),
            Some(100.0),
            Some(0.0),
            Some(40.0),
            None,
            None,
        ));

        // Build environment metrics from summaries
        let mut environment_metrics = Vec::new();

        if let Some(env) = env_summaries.first() {
            // Radiation
            if let Some(rad) = env.media_radiacao_msv_hora {
                environment_metrics.push(TrackingEnvironment::new(
                    "Radiação Cósmica".to_string(),
                    format!("{:.2}", rad),
                    "mSv/h".to_string(),
                    if rad > 1.0 { "alert" } else { "normal" }.to_string(),
                    if rad > 1.0 { "Alerta: Alta" } else { "Normal" }.to_string(),
                    if rad > 1.0 { Some("acima do limiar de 1.0 mSv/h".to_string()) } else { None },
                    if rad > 1.0 { Some("Reduzir exposição externa".to_string()) } else { None },
                ));
            }

            // Cabin Pressure
            if let Some(press) = env.media_pressao_kpa {
                environment_metrics.push(TrackingEnvironment::new(
                    "Pressão da Cabine".to_string(),
                    format!("{:.1}", press),
                    "kPa".to_string(),
                    if press >= 95.0 && press <= 105.0 { "normal" } else { "attention" }.to_string(),
                    if press >= 95.0 && press <= 105.0 { "Estável" } else { "Variação" }.to_string(),
                    None,
                    None,
                ));
            }

            // CO2
            if let Some(co2) = env.media_co2_ppm {
                let co2_pct = co2 / 10000.0; // convert ppm to %
                environment_metrics.push(TrackingEnvironment::new(
                    "Nível de CO2".to_string(),
                    format!("{:.2}", co2_pct),
                    "%".to_string(),
                    if co2_pct <= 0.5 { "normal" } else { "attention" }.to_string(),
                    if co2_pct <= 0.5 { "Seguro" } else { "Elevado" }.to_string(),
                    None,
                    None,
                ));
            }

            // O2 - default
            environment_metrics.push(TrackingEnvironment::new(
                "Oxigênio da Cabine (O2)".to_string(),
                "21.1".to_string(),
                "%".to_string(),
                "normal".to_string(),
                "Normal".to_string(),
                None,
                None,
            ));
        }

        TrackingData::new(
            crew_vitals,
            environment_metrics,
            last_update,
            astronaut_id,
            mission_name,
            current_module,
        )
    }
}

/// Current panel data (equivalent to PainelAtual from back-end)
#[derive(Debug)]
pub struct CurrentPanel {
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

/// Environmental summary (equivalent to ResumoAmbientalModulo)
#[derive(Debug)]
pub struct EnvironmentalSummary {
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