use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::domain::entities::evaluation_entity::AvaliacaoSaude;
use crate::domain::repositories::evaluation_repository::EvaluationRepository;

pub struct SqliteEvaluationRepository {
    pool: SqlitePool,
}

impl SqliteEvaluationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const COLUNAS: &str = r#"
    id,
    astronauta_id,
    missao_id,
    timestamp_utc,
    nivel_risco,
    resumo,
    indicadores_analisados_json,
    fatores_contextuais_json,
    explicacao,
    orientacao,
    updated_at,
    deleted_at
"#;

#[async_trait]
impl EvaluationRepository for SqliteEvaluationRepository {
    /// Persiste uma nova avaliação.
    async fn salvar(&self, avaliacao: &AvaliacaoSaude) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO avaliacoes_saude (
                id, astronauta_id, missao_id, timestamp_utc, nivel_risco,
                resumo, indicadores_analisados_json, fatores_contextuais_json,
                explicacao, orientacao, updated_at, deleted_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(&avaliacao.id)
        .bind(&avaliacao.astronauta_id)
        .bind(&avaliacao.missao_id)
        .bind(&avaliacao.timestamp_utc)
        .bind(&avaliacao.nivel_risco)
        .bind(&avaliacao.resumo)
        .bind(&avaliacao.indicadores_analisados_json)
        .bind(&avaliacao.fatores_contextuais_json)
        .bind(&avaliacao.explicacao)
        .bind(&avaliacao.orientacao)
        .bind(&avaliacao.updated_at)
        .bind(&avaliacao.deleted_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn buscar_por_id(&self, id: &str) -> Result<Option<AvaliacaoSaude>, sqlx::Error> {
        sqlx::query_as::<_, AvaliacaoSaude>(&format!(
            "SELECT {COLUNAS} FROM avaliacoes_saude WHERE id = ?1 AND deleted_at IS NULL"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Anteriores do astronauta, ordenadas cronologicamente DECRESCENTE.
    async fn listar_anteriores(
        &self,
        astronauta_id: &str,
        antes_de: Option<&str>,
        limite: i64,
    ) -> Result<Vec<AvaliacaoSaude>, sqlx::Error> {
        sqlx::query_as::<_, AvaliacaoSaude>(&format!(
            r#"
            SELECT {COLUNAS}
            FROM avaliacoes_saude
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
              AND (?2 IS NULL OR timestamp_utc < ?2)
            ORDER BY timestamp_utc DESC
            LIMIT ?3
            "#
        ))
        .bind(astronauta_id)
        .bind(antes_de)
        .bind(limite)
        .fetch_all(&self.pool)
        .await
    }

    /// A avaliação mais recente de um astronauta.
    async fn mais_recente(
        &self,
        astronauta_id: &str,
    ) -> Result<Option<AvaliacaoSaude>, sqlx::Error> {
        sqlx::query_as::<_, AvaliacaoSaude>(&format!(
            r#"
            SELECT {COLUNAS}
            FROM avaliacoes_saude
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
            ORDER BY timestamp_utc DESC
            LIMIT 1
            "#
        ))
        .bind(astronauta_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Avaliações de uma missão em um período (inclusivo), desc. cronológica.
    async fn listar_por_missao_e_periodo(
        &self,
        missao_id: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
        limite: i64,
    ) -> Result<Vec<AvaliacaoSaude>, sqlx::Error> {
        sqlx::query_as::<_, AvaliacaoSaude>(&format!(
            r#"
            SELECT {COLUNAS}
            FROM avaliacoes_saude
            WHERE missao_id = ?1
              AND deleted_at IS NULL
              AND (?2 IS NULL OR timestamp_utc >= ?2)
              AND (?3 IS NULL OR timestamp_utc <= ?3)
            ORDER BY timestamp_utc DESC
            LIMIT ?4
            "#
        ))
        .bind(missao_id)
        .bind(inicio_utc)
        .bind(fim_utc)
        .bind(limite)
        .fetch_all(&self.pool)
        .await
    }

    /// Preenche explicação/orientação da IA (sem tocar no nível de risco).
    async fn atualizar_orientacao(
        &self,
        id: &str,
        explicacao: Option<&str>,
        orientacao: Option<&str>,
        updated_at: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE avaliacoes_saude
            SET explicacao = ?2,
                orientacao = ?3,
                updated_at = ?4
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(explicacao)
        .bind(orientacao)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }
}
