use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::domain::entities::astronaut_entity::Astronaut;
use crate::domain::repositories::astronaut_repository::AstronautRepository;

pub struct SqliteAstronautRepository {
    pool: SqlitePool,
}

impl SqliteAstronautRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AstronautRepository for SqliteAstronautRepository {
    /// Query 1.1
    async fn find_by_id(&self, id: &str) -> Result<Option<Astronaut>, sqlx::Error> {
        sqlx::query_as::<_, Astronaut>(
            r#"
            SELECT
                id,
                nome,
                data_nascimento,
                tipo_sanguineo,
                peso_baseline_kg,
                altura_cm,
                updated_at,
                deleted_at
            FROM astronautas
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 1.2
    async fn list(&self, limite: i64, deslocamento: i64) -> Result<Vec<Astronaut>, sqlx::Error> {
        sqlx::query_as::<_, Astronaut>(
            r#"
            SELECT
                id,
                nome,
                data_nascimento,
                tipo_sanguineo,
                peso_baseline_kg,
                altura_cm,
                updated_at,
                deleted_at
            FROM astronautas
            WHERE deleted_at IS NULL
            ORDER BY nome
            LIMIT ?1 OFFSET ?2
            "#,
        )
        .bind(limite)
        .bind(deslocamento)
        .fetch_all(&self.pool)
        .await
    }
}
