pub mod dto;
pub mod handlers;
pub mod routes;

use axum::Router;
use std::sync::Arc;
use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

pub fn rotas(
    measurements: Arc<SqliteMeasurementRepository>,
) -> Router<Arc<SqliteMeasurementRepository>> {
    Router::new()
        .merge(routes::analises_ws::rotas())
        .merge(routes::seeding::rotas())
        .merge(routes::tracking::rotas())
        .with_state(measurements)
}