use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::domain::entities::alert_entity::{Alerta, AlertaSaude, EntradaAlerta};
use crate::domain::repositories::alert_repository::{AlertRepository, ContagemNivelRisco};

/// Busca um alerta pelo `id` (usado internamente após INSERT/UPDATE).
async fn buscar_alerta(
    executor: impl sqlx::SqliteExecutor<'_>,
    id: &str,
) -> Result<Alerta, sqlx::Error> {
    sqlx::query_as::<_, Alerta>(
        r#"
        SELECT
            id, avaliacao_id, astronauta_id, missao_id, condicao,
            timestamp_utc, nivel_risco, resumo, explicacao, orientacao,
            estado, atualizado_em, resolvido_em, updated_at, deleted_at
        FROM alertas
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_one(executor)
    .await
}

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

    /// Escrita — cria o alerta ATIVO da condição ou atualiza o existente.
    /// Transação garante que SELECT + INSERT/UPDATE permanecem coerentes
    /// (o índice único parcial do banco é a última barreira contra duplicidade).
    async fn criar_ou_atualizar_alerta(
        &self,
        entrada: &EntradaAlerta,
    ) -> Result<(Alerta, bool), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let existente = sqlx::query_as::<_, Alerta>(
            r#"
            SELECT
                id, avaliacao_id, astronauta_id, missao_id, condicao,
                timestamp_utc, nivel_risco, resumo, explicacao, orientacao,
                estado, atualizado_em, resolvido_em, updated_at, deleted_at
            FROM alertas
            WHERE astronauta_id = ?1
              AND condicao = ?2
              AND estado = 'ATIVO'
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(&entrada.astronauta_id)
        .bind(&entrada.condicao)
        .fetch_optional(&mut *tx)
        .await?;
        let criado = existente.is_none();

        let alerta = match existente {
            Some(atual) => {
                // A condição persiste: o MESMO registro evolui (o início,
                // `timestamp_utc`, não muda). O `updated_at` anterior é
                // preservado como histórico.
                sqlx::query(
                    r#"
                    UPDATE alertas
                    SET avaliacao_id = ?2,
                        nivel_risco = ?3,
                        resumo = ?4,
                        explicacao = ?5,
                        orientacao = ?6,
                        atualizado_em = ?7,
                        updated_at = ?8
                    WHERE id = ?1
                      AND estado = 'ATIVO'
                      AND deleted_at IS NULL
                    "#,
                )
                .bind(&atual.id)
                .bind(&entrada.avaliacao_id)
                .bind(&entrada.nivel_risco)
                .bind(&entrada.resumo)
                .bind(&entrada.explicacao)
                .bind(&entrada.orientacao)
                .bind(&entrada.atualizado_em)
                .bind(&entrada.atualizado_em)
                .execute(&mut *tx)
                .await?;

                buscar_alerta(&mut *tx, &atual.id).await?
            }
            None => {
                let id = Uuid::new_v4().to_string();
                sqlx::query(
                    r#"
                    INSERT INTO alertas (
                        id, avaliacao_id, astronauta_id, missao_id, condicao,
                        timestamp_utc, nivel_risco, resumo, explicacao, orientacao,
                        estado, atualizado_em
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'ATIVO', ?11)
                    "#,
                )
                .bind(&id)
                .bind(&entrada.avaliacao_id)
                .bind(&entrada.astronauta_id)
                .bind(&entrada.missao_id)
                .bind(&entrada.condicao)
                .bind(&entrada.timestamp_utc)
                .bind(&entrada.nivel_risco)
                .bind(&entrada.resumo)
                .bind(&entrada.explicacao)
                .bind(&entrada.orientacao)
                .bind(&entrada.atualizado_em)
                .execute(&mut *tx)
                .await?;

                buscar_alerta(&mut *tx, &id).await?
            }
        };

        tx.commit().await?;
        Ok((alerta, criado))
    }

    /// Leitura — alertas ATIVOS, do mais recente para o mais antigo.
    async fn listar_alertas_ativos(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
    ) -> Result<Vec<Alerta>, sqlx::Error> {
        sqlx::query_as::<_, Alerta>(
            r#"
            SELECT
                id, avaliacao_id, astronauta_id, missao_id, condicao,
                timestamp_utc, nivel_risco, resumo, explicacao, orientacao,
                estado, atualizado_em, resolvido_em, updated_at, deleted_at
            FROM alertas
            WHERE estado = 'ATIVO'
              AND deleted_at IS NULL
              AND (?1 IS NULL OR astronauta_id = ?1)
              AND (?2 IS NULL OR missao_id = ?2)
            ORDER BY timestamp_utc DESC
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Leitura — alerta ATIVO de uma condição específica.
    async fn buscar_alerta_ativo(
        &self,
        astronauta_id: &str,
        condicao: &str,
    ) -> Result<Option<Alerta>, sqlx::Error> {
        sqlx::query_as::<_, Alerta>(
            r#"
            SELECT
                id, avaliacao_id, astronauta_id, missao_id, condicao,
                timestamp_utc, nivel_risco, resumo, explicacao, orientacao,
                estado, atualizado_em, resolvido_em, updated_at, deleted_at
            FROM alertas
            WHERE astronauta_id = ?1
              AND condicao = ?2
              AND estado = 'ATIVO'
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(astronauta_id)
        .bind(condicao)
        .fetch_optional(&self.pool)
        .await
    }

    /// Escrita — marca um alerta como RESOLVIDO.
    async fn resolver_alerta(&self, id: &str, resolvido_em: &str) -> Result<bool, sqlx::Error> {
        let resultado = sqlx::query(
            r#"
            UPDATE alertas
            SET estado = 'RESOLVIDO',
                resolvido_em = ?2,
                updated_at = ?2
            WHERE id = ?1
              AND estado = 'ATIVO'
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(resolvido_em)
        .execute(&self.pool)
        .await?;
        Ok(resultado.rows_affected() > 0)
    }

    /// Escrita — resolve TODOS os alertas ativos do astronauta (e missão).
    async fn resolver_todos(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        resolvido_em: &str,
    ) -> Result<u64, sqlx::Error> {
        let resultado = sqlx::query(
            r#"
            UPDATE alertas
            SET estado = 'RESOLVIDO',
                resolvido_em = ?3,
                updated_at = ?3
            WHERE astronauta_id = ?1
              AND estado = 'ATIVO'
              AND deleted_at IS NULL
              AND (?2 IS NULL OR missao_id = ?2)
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .bind(resolvido_em)
        .execute(&self.pool)
        .await?;
        Ok(resultado.rows_affected())
    }

    /// Escrita — orientação da IA no alerta (não altera nível nem estado).
    async fn atualizar_orientacao_alerta(
        &self,
        id: &str,
        explicacao: Option<&str>,
        orientacao: Option<&str>,
        atualizado_em: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE alertas
            SET explicacao = ?2,
                orientacao = ?3,
                updated_at = ?4
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(explicacao)
        .bind(orientacao)
        .bind(atualizado_em)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }
}
