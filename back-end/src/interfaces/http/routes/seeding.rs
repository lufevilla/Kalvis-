use axum::{routing::post, Router, extract::State};
use sqlx::SqlitePool;
use crate::interfaces::http::handlers::seeding_handler::seed_data;

pub fn rotas() -> Router {
    Router::new()
        .route("/seed", post(seed_data))
}
