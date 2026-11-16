use axum::{routing::get, Router};

use crate::interfaces::http::handlers::tracking_handler::get_tracking_data;
use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;
use std::sync::Arc;

pub fn rotas() -> Router<Arc<SqliteMeasurementRepository>> {
    Router::new()
        .route("/tracking", get(get_tracking_data))
}