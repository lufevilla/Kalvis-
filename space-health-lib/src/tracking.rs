use std::sync::OnceLock;
use crate::database::Database;
use crate::models::{TrackingData, TrackingVital, TrackingEnvironment};
use anyhow::Result;
use flutter_rust_bridge::frb;

static DB_INSTANCE: OnceLock<Database> = OnceLock::new();

/// Initialize database connection (must be called once at startup)
/// 
/// # Arguments
/// * `db_path` - Path to SQLite database file
/// 
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` on failure
#[frb(sync)]
pub fn init_database(db_path: String) -> Result<(), String> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init();

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
    
    rt.block_on(async {
        let db = Database::new(&db_path).await.map_err(|e| format!("Database init failed: {}", e))?;
        DB_INSTANCE.set(db).map_err(|_| "Database already initialized".to_string())?;
        Ok(())
    })
}

/// Get tracking data for an astronaut
/// 
/// # Arguments
/// * `astronaut_id` - ID of the astronaut (e.g., "AST001")
/// 
/// # Returns
/// * `Ok(TrackingData)` with vitals, environment, and metadata
/// * `Err(String)` on failure
#[frb(sync)]
pub fn get_tracking_data(astronaut_id: String) -> Result<TrackingData, String> {
    let db = DB_INSTANCE.get().ok_or_else(|| "Database not initialized. Call init_database() first.".to_string())?;
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
    
    rt.block_on(async {
        let panel = db.get_current_panel(&astronaut_id)
            .await
            .map_err(|e| format!("Failed to get panel: {}", e))?
            .ok_or_else(|| format!("No data found for astronaut: {}", astronaut_id))?;

        let missao_id = panel.missao_id.clone().unwrap_or_default();
        let env_summaries = if !missao_id.is_empty() {
            db.get_environmental_summary_by_mission(&missao_id)
                .await
                .map_err(|e| format!("Failed to get environmental data: {}", e))?
        } else {
            vec![]
        };

        let tracking_data = db.build_tracking_data(panel, env_summaries, astronaut_id);
        Ok(tracking_data)
    })
}

/// Get only crew vitals (lighter query)
#[frb(sync)]
pub fn get_crew_vitals(astronaut_id: String) -> Result<Vec<crate::models::TrackingVital>, String> {
    let data = get_tracking_data(astronaut_id)?;
    Ok(data.crew_vitals)
}

/// Get only environment metrics (lighter query)
#[frb(sync)]
pub fn get_environment_metrics(astronaut_id: String) -> Result<Vec<crate::models::TrackingEnvironment>, String> {
    let data = get_tracking_data(astronaut_id)?;
    Ok(data.environment_metrics)
}