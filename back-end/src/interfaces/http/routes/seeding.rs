use axum::{routing::post, Router, extract::State};
use sqlx::SqlitePool;
use crate::interfaces::http::handlers::seeding_handler::seed_data;
use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;
use std::sync::Arc;

pub fn rotas() -> Router<Arc<SqliteMeasurementRepository>> {
    Router::new()
        .route("/seed", post(seed_data))
}
