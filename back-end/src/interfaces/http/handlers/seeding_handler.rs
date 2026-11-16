use axum::{extract::State, Json, response::IntoResponse, debug_handler};
use sqlx::SqlitePool;
use serde::Serialize;
use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

#[derive(Serialize)]
pub struct SeedingResponse {
    message: String,
}

#[debug_handler]
pub async fn seed_data(
    State(repo): State<std::sync::Arc<SqliteMeasurementRepository>>,
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
