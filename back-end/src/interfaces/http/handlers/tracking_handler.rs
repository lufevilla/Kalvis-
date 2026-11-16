use axum::{extract::{State, Query}, Json, debug_handler};
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::repositories::measurement_repository::{MeasurementRepository, PainelAtual, ResumoAmbientalModulo};
use crate::interfaces::http::dto::tracking_dto::TrackingDataResponse;
use crate::shared::errors::{AppErro, AppResultado};
use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

#[debug_handler]
pub async fn get_tracking_data(
    State(measurements): State<Arc<SqliteMeasurementRepository>>,
    Query(params): Query<HashMap<String, String>>,
) -> AppResultado<Json<TrackingDataResponse>> {
    let astronauta_id = params.get("astronauta_id")
        .cloned()
        .unwrap_or_else(|| "AST001".to_string());

    // Get current panel data
    let panel = measurements.get_current_panel(&astronauta_id).await
        .map_err(AppErro::Repositorio)?
        .ok_or_else(|| AppErro::NaoEncontrado("Painel não encontrado para o astronauta".to_string()))?;

    // Get mission ID from panel to fetch environmental data
    let env_summaries = if let Some(mid) = &panel.missao_id {
        measurements.get_environmental_summary_by_mission(mid, None, None).await
            .map_err(AppErro::Repositorio)?
    } else {
        vec![]
    };

    let response = TrackingDataResponse::from_panel_and_env(panel, env_summaries, astronauta_id);

    Ok(Json(response))
}