use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::domain::entities::measurement_entity::{
    AmbienteNave, BaselineFisiologico, ExameClinico, PresencaModulo, RegistroDiario,
    TelemetriaFisiologica,
};
use crate::domain::repositories::measurement_repository::{
    ComparacaoBaseline, ContextoMedicao, MeasurementRepository, PainelAtual, PontoSerie,
    ResumoAmbientalModulo, ResumoDiarioTelemetria, ResumoIndicador,
};

/// Indicadores fisiológicos suportados nas queries de série temporal e resumo.
const INDICADORES_TELEMETRIA: [&str; 9] = [
    "frequencia_cardiaca_bpm",
    "spo2_percentual",
    "temperatura_corporal_c",
    "temperatura_pele_c",
    "taxa_respiratoria_rpm",
    "hrv_ms",
    "pressao_sistolica_mmhg",
    "pressao_diastolica_mmhg",
    "glicose_mg_dl",
];

/// Monta a expressão `CASE :param WHEN ... END` que extrai o valor do indicador
/// a partir do parâmetro posicionado (`?N`). O indicador real é sempre vinculado
/// via bind, evitando interpolação de valores externos no SQL.
fn caso_indicador(param: &str) -> String {
    let mut expressao = format!("CASE {param} ");
    for indicador in INDICADORES_TELEMETRIA {
        expressao.push_str(&format!("WHEN '{indicador}' THEN {indicador} "));
    }
    expressao.push_str("END");
    expressao
}

pub struct SqliteMeasurementRepository {
    pool: SqlitePool,
}

impl SqliteMeasurementRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MeasurementRepository for SqliteMeasurementRepository {
    /// Query 1.6
    async fn find_telemetry_by_id(
        &self,
        id: &str,
    ) -> Result<Option<TelemetriaFisiologica>, sqlx::Error> {
        sqlx::query_as::<_, TelemetriaFisiologica>(
            r#"
            SELECT
                id,
                astronauta_id,
                missao_id,
                timestamp_utc,
                frequencia_cardiaca_bpm,
                spo2_percentual,
                temperatura_corporal_c,
                temperatura_pele_c,
                taxa_respiratoria_rpm,
                hrv_ms,
                pressao_sistolica_mmhg,
                pressao_diastolica_mmhg,
                glicose_mg_dl,
                origem_medicao,
                updated_at,
                deleted_at
            FROM telemetria_fisiologica
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 1.7
    async fn find_daily_log_by_id(
        &self,
        id: &str,
    ) -> Result<Option<RegistroDiario>, sqlx::Error> {
        sqlx::query_as::<_, RegistroDiario>(
            r#"
            SELECT
                id,
                astronauta_id,
                data_registro,
                horas_sono_total,
                qualidade_sono_score,
                exercicio_resistencia_min,
                exercicio_aerobico_min,
                ingestao_agua_ml,
                calorias_consumidas,
                score_teste_cognitivo,
                nivel_estresse_subjetivo,
                humor_score,
                fadiga_score,
                updated_at,
                deleted_at
            FROM registros_diarios
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 1.9
    async fn find_environment_by_id(
        &self,
        id: &str,
    ) -> Result<Option<AmbienteNave>, sqlx::Error> {
        sqlx::query_as::<_, AmbienteNave>(
            r#"
            SELECT
                id,
                modulo_id,
                timestamp_utc,
                co2_ppm,
                radiacao_msv_hora,
                temperatura_cabine_c,
                pressao_kpa,
                updated_at,
                deleted_at
            FROM ambiente_nave
            WHERE id = ?1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 1.10
    async fn list_presences(
        &self,
        astronauta_id: &str,
    ) -> Result<Vec<PresencaModulo>, sqlx::Error> {
        sqlx::query_as::<_, PresencaModulo>(
            r#"
            SELECT
                id,
                astronauta_id,
                modulo_id,
                inicio_utc,
                fim_utc,
                updated_at,
                deleted_at
            FROM presenca_modulo
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
            ORDER BY inicio_utc DESC
            "#,
        )
        .bind(astronauta_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 1.11
    async fn list_clinical_exams(
        &self,
        astronauta_id: &str,
    ) -> Result<Vec<ExameClinico>, sqlx::Error> {
        sqlx::query_as::<_, ExameClinico>(
            r#"
            SELECT
                id,
                astronauta_id,
                missao_id,
                fase_missao,
                data_coleta,
                categoria,
                resultados_json,
                observacoes_medicas,
                updated_at,
                deleted_at
            FROM exames_clinicos
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
            ORDER BY data_coleta DESC
            "#,
        )
        .bind(astronauta_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 6.6
    async fn list_clinical_exams_filtered(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        fase_missao: Option<&str>,
        categoria: Option<&str>,
        data_inicio: Option<&str>,
        data_fim: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<ExameClinico>, sqlx::Error> {
        sqlx::query_as::<_, ExameClinico>(
            r#"
            SELECT
                id,
                astronauta_id,
                missao_id,
                fase_missao,
                data_coleta,
                categoria,
                resultados_json,
                observacoes_medicas,
                updated_at,
                deleted_at
            FROM exames_clinicos
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
              AND (?2 IS NULL OR missao_id = ?2)
              AND (?3 IS NULL OR fase_missao = ?3)
              AND (?4 IS NULL OR categoria = ?4)
              AND (?5 IS NULL OR data_coleta >= ?5)
              AND (?6 IS NULL OR data_coleta <= ?6)
            ORDER BY data_coleta DESC
            LIMIT ?7 OFFSET ?8
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .bind(fase_missao)
        .bind(categoria)
        .bind(data_inicio)
        .bind(data_fim)
        .bind(limite)
        .bind(deslocamento)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 1.12
    async fn list_baselines(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        fase_missao: Option<&str>,
    ) -> Result<Vec<BaselineFisiologico>, sqlx::Error> {
        sqlx::query_as::<_, BaselineFisiologico>(
            r#"
            SELECT
                id,
                astronauta_id,
                indicador,
                fase_missao,
                missao_id,
                valor_referencia,
                desvio_padrao,
                periodo_inicio,
                periodo_fim,
                quantidade_amostras,
                atualizado_em,
                updated_at,
                deleted_at
            FROM baseline_fisiologico
            WHERE astronauta_id = ?1
              AND deleted_at IS NULL
              AND (?2 IS NULL OR missao_id = ?2)
              AND (?3 IS NULL OR fase_missao = ?3)
            ORDER BY indicador, fase_missao
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .bind(fase_missao)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 2 — comparação das medições observadas com o baseline fisiológico.
    async fn compare_telemetry_with_baseline(
        &self,
        astronauta_id: &str,
        fase_missao: &str,
        missao_id: Option<&str>,
    ) -> Result<Vec<ComparacaoBaseline>, sqlx::Error> {
        sqlx::query_as::<_, ComparacaoBaseline>(
            r#"
            WITH telemetria AS (
                SELECT
                    frequencia_cardiaca_bpm,
                    spo2_percentual,
                    temperatura_corporal_c,
                    temperatura_pele_c,
                    taxa_respiratoria_rpm,
                    hrv_ms,
                    pressao_sistolica_mmhg,
                    pressao_diastolica_mmhg,
                    glicose_mg_dl
                FROM telemetria_fisiologica
                WHERE astronauta_id = ?1
                  AND deleted_at IS NULL
                  AND (?3 IS NULL OR missao_id = ?3)
            ),
            amostras AS (
                SELECT 'frequencia_cardiaca_bpm' AS indicador, frequencia_cardiaca_bpm AS valor FROM telemetria
                UNION ALL SELECT 'spo2_percentual', spo2_percentual FROM telemetria
                UNION ALL SELECT 'temperatura_corporal_c', temperatura_corporal_c FROM telemetria
                UNION ALL SELECT 'temperatura_pele_c', temperatura_pele_c FROM telemetria
                UNION ALL SELECT 'taxa_respiratoria_rpm', taxa_respiratoria_rpm FROM telemetria
                UNION ALL SELECT 'hrv_ms', hrv_ms FROM telemetria
                UNION ALL SELECT 'pressao_sistolica_mmhg', pressao_sistolica_mmhg FROM telemetria
                UNION ALL SELECT 'pressao_diastolica_mmhg', pressao_diastolica_mmhg FROM telemetria
                UNION ALL SELECT 'glicose_mg_dl', glicose_mg_dl FROM telemetria
            ),
            observado AS (
                SELECT
                    indicador,
                    AVG(valor) AS media_observada,
                    MIN(valor) AS minimo_observado,
                    MAX(valor) AS maximo_observado,
                    COUNT(valor) AS quantidade_amostras
                FROM amostras
                WHERE valor IS NOT NULL
                GROUP BY indicador
            ),
            indicadores AS (
                SELECT 'frequencia_cardiaca_bpm' AS indicador
                UNION ALL SELECT 'spo2_percentual'
                UNION ALL SELECT 'temperatura_corporal_c'
                UNION ALL SELECT 'temperatura_pele_c'
                UNION ALL SELECT 'taxa_respiratoria_rpm'
                UNION ALL SELECT 'hrv_ms'
                UNION ALL SELECT 'pressao_sistolica_mmhg'
                UNION ALL SELECT 'pressao_diastolica_mmhg'
                UNION ALL SELECT 'glicose_mg_dl'
            )
            SELECT
                i.indicador,
                b.valor_referencia AS valor_baseline,
                b.desvio_padrao AS desvio_padrao_baseline,
                o.media_observada,
                o.minimo_observado,
                o.maximo_observado,
                o.quantidade_amostras,
                CASE
                    WHEN b.valor_referencia IS NOT NULL AND o.media_observada IS NOT NULL
                    THEN o.media_observada - b.valor_referencia
                END AS desvio_medio,
                CASE
                    WHEN b.desvio_padrao IS NOT NULL
                         AND b.desvio_padrao > 0
                         AND o.media_observada IS NOT NULL
                    THEN (o.media_observada - b.valor_referencia) / b.desvio_padrao
                END AS desvio_em_z
            FROM indicadores i
            LEFT JOIN observado o
                   ON o.indicador = i.indicador
            LEFT JOIN baseline_fisiologico b
                   ON b.indicador = i.indicador
                  AND b.astronauta_id = ?1
                  AND b.fase_missao = ?2
                  AND b.deleted_at IS NULL
                  AND ((?3 IS NULL AND b.missao_id IS NULL) OR b.missao_id = ?3)
            ORDER BY i.indicador
            "#,
        )
        .bind(astronauta_id)
        .bind(fase_missao)
        .bind(missao_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 3.1 — contexto diário de uma medição de telemetria.
    async fn get_daily_context_for_telemetry(
        &self,
        telemetria_id: &str,
    ) -> Result<Option<ContextoMedicao>, sqlx::Error> {
        sqlx::query_as::<_, ContextoMedicao>(
            r#"
            SELECT
                t.id AS telemetria_id,
                t.astronauta_id,
                t.missao_id,
                t.timestamp_utc,
                t.frequencia_cardiaca_bpm,
                t.spo2_percentual,
                t.temperatura_corporal_c,
                t.temperatura_pele_c,
                t.taxa_respiratoria_rpm,
                t.hrv_ms,
                t.pressao_sistolica_mmhg,
                t.pressao_diastolica_mmhg,
                t.glicose_mg_dl,
                t.origem_medicao,
                r.id AS registro_diario_id,
                r.data_registro,
                r.horas_sono_total,
                r.qualidade_sono_score,
                r.exercicio_resistencia_min,
                r.exercicio_aerobico_min,
                r.ingestao_agua_ml,
                r.calorias_consumidas,
                r.score_teste_cognitivo,
                r.nivel_estresse_subjetivo,
                r.humor_score,
                r.fadiga_score,
                (
                    SELECT COUNT(*)
                    FROM telemetria_fisiologica t2
                    WHERE t2.astronauta_id = t.astronauta_id
                      AND date(t2.timestamp_utc) = date(t.timestamp_utc)
                      AND t2.deleted_at IS NULL
                ) AS medicoes_no_dia
            FROM telemetria_fisiologica t
            LEFT JOIN registros_diarios r
                   ON r.astronauta_id = t.astronauta_id
                  AND r.data_registro = date(t.timestamp_utc)
                  AND r.deleted_at IS NULL
            WHERE t.id = ?1
              AND t.deleted_at IS NULL
            "#,
        )
        .bind(telemetria_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Query 3.2 — série temporal de um indicador fisiológico.
    async fn list_indicator_time_series(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        indicador: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<PontoSerie>, sqlx::Error> {
        let sql = format!(
            r#"
            WITH telemetria AS (
                SELECT
                    id,
                    timestamp_utc,
                    {} AS valor
                FROM telemetria_fisiologica
                WHERE astronauta_id = ?1
                  AND deleted_at IS NULL
                  AND (?2 IS NULL OR missao_id = ?2)
                  AND (?3 IS NULL OR timestamp_utc >= ?3)
                  AND (?4 IS NULL OR timestamp_utc <= ?4)
            )
            SELECT
                timestamp_utc AS momento_utc,
                valor
            FROM telemetria
            WHERE valor IS NOT NULL
            ORDER BY timestamp_utc
            LIMIT ?6 OFFSET ?7
            "#,
            caso_indicador("?5")
        );

        sqlx::query_as::<_, PontoSerie>(&sql)
            .bind(astronauta_id)
            .bind(missao_id)
            .bind(inicio_utc)
            .bind(fim_utc)
            .bind(indicador)
            .bind(limite)
            .bind(deslocamento)
            .fetch_all(&self.pool)
            .await
    }

    /// Query 3.3 — resumo estatístico de um indicador fisiológico.
    async fn get_indicator_summary(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        indicador: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
    ) -> Result<Option<ResumoIndicador>, sqlx::Error> {
        let sql = format!(
            r#"
            WITH telemetria AS (
                SELECT
                    {} AS valor,
                    date(timestamp_utc) AS dia
                FROM telemetria_fisiologica
                WHERE astronauta_id = ?1
                  AND deleted_at IS NULL
                  AND (?3 IS NULL OR missao_id = ?3)
                  AND (?4 IS NULL OR timestamp_utc >= ?4)
                  AND (?5 IS NULL OR timestamp_utc <= ?5)
            )
            SELECT
                ?2 AS indicador,
                AVG(valor) AS media,
                MIN(valor) AS minimo,
                MAX(valor) AS maximo,
                COUNT(valor) AS quantidade_amostras,
                COUNT(DISTINCT dia) AS dias_com_registros,
                CASE
                    WHEN AVG(valor * valor) - AVG(valor) * AVG(valor) >= 0
                    THEN AVG(valor * valor) - AVG(valor) * AVG(valor)
                END AS variancia
            FROM telemetria
            WHERE valor IS NOT NULL
            "#,
            caso_indicador("?2")
        );

        let resumo = sqlx::query_as::<_, ResumoIndicador>(&sql)
            .bind(astronauta_id)
            .bind(indicador)
            .bind(missao_id)
            .bind(inicio_utc)
            .bind(fim_utc)
            .fetch_optional(&self.pool)
            .await?;

        // Uma agregação sobre conjunto vazio ainda retorna uma linha com valores
        // nulos; nesse caso consideramos que não há resumo.
        Ok(resumo.filter(|r| r.quantidade_amostras.unwrap_or(0) > 0))
    }

    /// Query 4 — resumo diário da telemetria correlacionado aos registros diários.
    async fn list_daily_summaries(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        data_inicio: Option<&str>,
        data_fim: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<ResumoDiarioTelemetria>, sqlx::Error> {
        sqlx::query_as::<_, ResumoDiarioTelemetria>(
            r#"
            SELECT
                date(t.timestamp_utc) AS data_referencia,
                COUNT(*) AS medicoes_no_dia,
                MIN(t.timestamp_utc) AS primeira_medicao_utc,
                MAX(t.timestamp_utc) AS ultima_medicao_utc,
                AVG(t.frequencia_cardiaca_bpm) AS media_frequencia_cardiaca,
                AVG(t.spo2_percentual) AS media_spo2,
                AVG(t.temperatura_corporal_c) AS media_temperatura_corporal,
                AVG(t.hrv_ms) AS media_hrv,
                r.horas_sono_total,
                r.qualidade_sono_score,
                r.exercicio_resistencia_min,
                r.exercicio_aerobico_min,
                r.ingestao_agua_ml,
                r.calorias_consumidas,
                r.score_teste_cognitivo,
                r.nivel_estresse_subjetivo,
                r.humor_score,
                r.fadiga_score
            FROM telemetria_fisiologica t
            LEFT JOIN registros_diarios r
                   ON r.astronauta_id = t.astronauta_id
                  AND r.data_registro = date(t.timestamp_utc)
                  AND r.deleted_at IS NULL
            WHERE t.astronauta_id = ?1
              AND t.deleted_at IS NULL
              AND (?2 IS NULL OR t.missao_id = ?2)
              AND (?3 IS NULL OR date(t.timestamp_utc) >= ?3)
              AND (?4 IS NULL OR date(t.timestamp_utc) <= ?4)
            GROUP BY data_referencia, r.id
            ORDER BY data_referencia DESC
            LIMIT ?5 OFFSET ?6
            "#,
        )
        .bind(astronauta_id)
        .bind(missao_id)
        .bind(data_inicio)
        .bind(data_fim)
        .bind(limite)
        .bind(deslocamento)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 5.1 — medições ambientais de um módulo, com paginação.
    async fn list_environment_by_module(
        &self,
        modulo_id: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<AmbienteNave>, sqlx::Error> {
        sqlx::query_as::<_, AmbienteNave>(
            r#"
            SELECT
                id,
                modulo_id,
                timestamp_utc,
                co2_ppm,
                radiacao_msv_hora,
                temperatura_cabine_c,
                pressao_kpa,
                updated_at,
                deleted_at
            FROM ambiente_nave
            WHERE modulo_id = ?1
              AND deleted_at IS NULL
              AND (?2 IS NULL OR timestamp_utc >= ?2)
              AND (?3 IS NULL OR timestamp_utc <= ?3)
            ORDER BY timestamp_utc
            LIMIT ?4 OFFSET ?5
            "#,
        )
        .bind(modulo_id)
        .bind(inicio_utc)
        .bind(fim_utc)
        .bind(limite)
        .bind(deslocamento)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 5.2 — resumo ambiental por módulo de uma missão.
    async fn get_environmental_summary_by_mission(
        &self,
        missao_id: &str,
        inicio_utc: Option<&str>,
        fim_utc: Option<&str>,
    ) -> Result<Vec<ResumoAmbientalModulo>, sqlx::Error> {
        sqlx::query_as::<_, ResumoAmbientalModulo>(
            r#"
            SELECT
                mn.id AS modulo_id,
                mn.nome AS nome_modulo,
                m.id AS missao_id,
                m.nome_missao AS nome_missao,
                COUNT(a.id) AS quantidade_amostras,
                AVG(a.co2_ppm) AS media_co2_ppm,
                MAX(a.co2_ppm) AS max_co2_ppm,
                AVG(a.radiacao_msv_hora) AS media_radiacao_msv_hora,
                MAX(a.radiacao_msv_hora) AS max_radiacao_msv_hora,
                AVG(a.temperatura_cabine_c) AS media_temperatura_cabine_c,
                MIN(a.temperatura_cabine_c) AS min_temperatura_cabine_c,
                MAX(a.temperatura_cabine_c) AS max_temperatura_cabine_c,
                AVG(a.pressao_kpa) AS media_pressao_kpa,
                MIN(a.pressao_kpa) AS min_pressao_kpa,
                MAX(a.pressao_kpa) AS max_pressao_kpa,
                MIN(a.timestamp_utc) AS primeira_amostra_utc,
                MAX(a.timestamp_utc) AS ultima_amostra_utc
            FROM modulos_nave mn
            JOIN missoes m
                 ON m.id = mn.missao_id
                AND m.deleted_at IS NULL
            LEFT JOIN ambiente_nave a
                   ON a.modulo_id = mn.id
                  AND a.deleted_at IS NULL
                  AND (?2 IS NULL OR a.timestamp_utc >= ?2)
                  AND (?3 IS NULL OR a.timestamp_utc <= ?3)
            WHERE mn.missao_id = ?1
              AND mn.deleted_at IS NULL
            GROUP BY mn.id, mn.nome, m.id, m.nome_missao
            ORDER BY mn.nome
            "#,
        )
        .bind(missao_id)
        .bind(inicio_utc)
        .bind(fim_utc)
        .fetch_all(&self.pool)
        .await
    }

    /// Query 6 — painel atual do astronauta.
    async fn get_current_panel(
        &self,
        astronauta_id: &str,
    ) -> Result<Option<PainelAtual>, sqlx::Error> {
        sqlx::query_as::<_, PainelAtual>(
            r#"
            SELECT
                t.id AS telemetria_id,
                t.timestamp_utc AS ultima_telemetria_utc,
                t.frequencia_cardiaca_bpm,
                t.spo2_percentual,
                t.temperatura_corporal_c,
                t.temperatura_pele_c,
                t.taxa_respiratoria_rpm,
                t.hrv_ms,
                t.pressao_sistolica_mmhg,
                t.pressao_diastolica_mmhg,
                t.glicose_mg_dl,
                t.missao_id,
                m.nome_missao,
                r.id AS registro_diario_id,
                r.data_registro AS ultimo_registro_data,
                r.horas_sono_total,
                r.qualidade_sono_score,
                r.nivel_estresse_subjetivo,
                r.humor_score,
                r.fadiga_score,
                p.id AS presenca_id,
                mn.nome AS nome_modulo_atual,
                p.inicio_utc AS presenca_inicio_utc
            FROM telemetria_fisiologica t
            LEFT JOIN missoes m
                   ON m.id = t.missao_id
                  AND m.deleted_at IS NULL
            LEFT JOIN registros_diarios r
                   ON r.id = (
                       SELECT r2.id
                       FROM registros_diarios r2
                       WHERE r2.astronauta_id = t.astronauta_id
                         AND r2.deleted_at IS NULL
                       ORDER BY r2.data_registro DESC
                       LIMIT 1
                   )
            LEFT JOIN presenca_modulo p
                   ON p.id = (
                       SELECT p2.id
                       FROM presenca_modulo p2
                       WHERE p2.astronauta_id = t.astronauta_id
                         AND p2.fim_utc IS NULL
                         AND p2.deleted_at IS NULL
                       ORDER BY p2.inicio_utc DESC
                       LIMIT 1
                   )
            LEFT JOIN modulos_nave mn
                   ON mn.id = p.modulo_id
                  AND mn.deleted_at IS NULL
            WHERE t.id = (
                SELECT t2.id
                FROM telemetria_fisiologica t2
                WHERE t2.astronauta_id = ?1
                  AND t2.deleted_at IS NULL
                ORDER BY t2.timestamp_utc DESC
                LIMIT 1
            )
            "#,
        )
        .bind(astronauta_id)
        .fetch_optional(&self.pool)
        .await
    }
}
