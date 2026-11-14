use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::domain::entities::mission_entity::{Missao, ModuloNave};
use crate::domain::repositories::mission_repository::{MissionRepository, ParticipacaoAstronauta};

pub struct SqliteMissionRepository {
    pool: SqlitePool,
}

impl SqliteMissionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MissionRepository for SqliteMissionRepository {
    /// Query 1.3
    async fn find_by_id(&self, id: &str) -> Result<Option<Missao>, sqlx::Error> {
        sqlx::query_as::<_, Missao>(
            r#"
            SELECT
                id,
                nome_missao,
                data_lancamento,
                data_retorno,
                updated_at,
                deleted_at
            FROM missoes
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 1.4
    async fn list(&self, limite: i64, deslocamento: i64) -> Result<Vec<Missao>, sqlx::Error> {
        sqlx::query_as::<_, Missao>(
            r#"
            SELECT
                id,
                nome_missao,
                data_lancamento,
                data_retorno,
                updated_at,
                deleted_at
            FROM missoes
            WHERE deleted_at IS NULL
            ORDER BY data_lancamento DESC
            LIMIT ?1 OFFSET ?2
            "#,
        )
        .bind(limite)
        .bind(deslocamento)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 1.5
    async fn list_participations_by_astronaut(
        &self,
        astronauta_id: &str,
    ) -> Result<Vec<ParticipacaoAstronauta>, sqlx::Error> {
        sqlx::query_as::<_, ParticipacaoAstronauta>(
            r#"
            SELECT
                pm.id,
                pm.astronauta_id,
                pm.missao_id,
                pm.funcao,
                m.nome_missao,
                m.data_lancamento,
                m.data_retorno
            FROM participacao_missao pm
            JOIN astronautas a
                 ON a.id = pm.astronauta_id
                AND a.deleted_at IS NULL
            JOIN missoes m
                 ON m.id = pm.missao_id
                AND m.deleted_at IS NULL
            WHERE pm.astronauta_id = ?1
              AND pm.deleted_at IS NULL
            ORDER BY m.data_lancamento DESC
            "#,
        )
        .bind(astronauta_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 1.8
    async fn list_modules_by_mission(
        &self,
        missao_id: &str,
    ) -> Result<Vec<ModuloNave>, sqlx::Error> {
        sqlx::query_as::<_, ModuloNave>(
            r#"
            SELECT
                id,
                missao_id,
                nome,
                updated_at,
                deleted_at
            FROM modulos_nave
            WHERE missao_id = ?1
              AND deleted_at IS NULL
            ORDER BY nome
            "#,
        )
        .bind(missao_id)
        .fetch_all(&self.pool)
        .await
    }
}
