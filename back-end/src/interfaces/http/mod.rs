pub mod dto;
pub mod handlers;
pub mod routes;

use axum::Router;
use std::sync::Arc;
use crate::domain::repositories::measurement_repository::MeasurementRepository;

pub fn rotas<M: MeasurementRepository + 'static>(
    measurements: Arc<M>,
) -> Router<Arc<M>> {
    Router::<Arc<M>>::new()
        .merge(routes::analises_ws::rotas())
        .merge(routes::seeding::rotas())
        .merge(routes::tracking::rotas())
        .with_state(measurements)
}