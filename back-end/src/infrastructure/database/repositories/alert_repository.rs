use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::domain::entities::alert_entity::AlertaSaude;
use crate::domain::repositories::alert_repository::{AlertRepository, ContagemNivelRisco};

pub struct SqliteAlertRepository {
    pool: SqlitePool,
}

impl SqliteAlertRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AlertRepository for SqliteAlertRepository {
    /// Query 7.1
    async fn find_by_id(&self, id: &str) -> Result<Option<AlertaSaude>, sqlx::Error> {
        sqlx::query_as::<_, AlertaSaude>(
            r#"
            SELECT
                id,
                astronauta_id,
                missao_id,
                timestamp_utc,
                nivel_risco,
                resumo,
                explicacao,
                orientacao
            FROM avaliacoes_saude
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 7.2
    async fn list(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
        nivel_risco: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<AlertaSaude>, sqlx::Error> {
        sqlx::query_as::<_, AlertaSaude>(
            r#"
            SELECT
                id,
                astronauta_id,
                missao_id,
                timestamp_utc,
                nivel_risco,
                resumo,
                explicacao,
                orientacao
            FROM avaliacoes_saude
            WHERE deleted_at IS NULL
              AND (?1 IS NULL OR astronauta_id = ?1)
              AND (?2 IS NULL OR missao_id = ?2)
              AND (?3 IS NULL OR nivel_risco = ?3)
            ORDER BY timestamp_utc DESC
            LIMIT ?4 OFFSET ?5
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .bind(nivel_risco)
        .bind(limite)
        .bind(deslocamento)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 7.3
    async fn list_recent_by_astronaut(
        &self,
        astronauta_id: &str,
        limite: i64,
    ) -> Result<Vec<AlertaSaude>, sqlx::Error> {
        sqlx::query_as::<_, AlertaSaude>(
            r#"
            SELECT
                id,
                astronauta_id,
                missao_id,
                timestamp_utc,
                nivel_risco,
                resumo,
                explicacao,
                orientacao
            FROM avaliacoes_saude
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
            ORDER BY timestamp_utc DESC
            LIMIT ?2
            "#,
        )
        .bind(astronauta_id)
        .bind(limite)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 7.4
    async fn count_by_risk_level(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
    ) -> Result<Vec<ContagemNivelRisco>, sqlx::Error> {
        sqlx::query_as::<_, ContagemNivelRisco>(
            r#"
            SELECT
                nivel_risco,
                COUNT(*) AS quantidade
            FROM avaliacoes_saude
            WHERE deleted_at IS NULL
              AND (?1 IS NULL OR astronauta_id = ?1)
              AND (?2 IS NULL OR missao_id = ?2)
            GROUP BY nivel_risco
            ORDER BY quantidade DESC
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .fetch_all(&self.pool)
        .await
    }
}
