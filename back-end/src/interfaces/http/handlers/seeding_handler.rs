use axum::{extract::State, Json, response::IntoResponse};
use sqlx::SqlitePool;
use serde::Serialize;
use crate::domain::repositories::measurement_repository::MeasurementRepository;

#[derive(Serialize)]
pub struct SeedingResponse {
    message: String,
}

pub async fn seed_data<M: MeasurementRepository>(
    State(repo): State<std::sync::Arc<M>>,
) -> impl IntoResponse {
    let pool = repo.pool();
    
    // Insert simulated data
    let _ = sqlx::query("INSERT OR IGNORE INTO astronautas (id, nome, data_nascimento) VALUES (?, ?, ?)")
        .bind("astro_1")
        .bind("Astronauta Alpha")
        .bind("1985-05-20")
        .execute(pool)
        .await;

    let _ = sqlx::query("INSERT OR IGNORE INTO missoes (id, nome_missao) VALUES (?, ?)")
        .bind("missao_1")
        .bind("Missão Espacial Alpha")
        .execute(pool)
        .await;

    Json(SeedingResponse {
        message: "Dados simulados inseridos com sucesso!".to_string(),
    })
}
