use axum::{routing::get, Router};

use crate::interfaces::http::handlers::tracking_handler::get_tracking_data;

pub fn rotas() -> Router {
    Router::new()
        .route("/tracking", get(get_tracking_data))
}