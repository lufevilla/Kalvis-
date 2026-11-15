/*
PONTO DE ENTRADA DO SPACE HEALTH.

Fio condutor (tudo configurável via ambiente/`.env` — ver `config::settings`):
 1. carrega configurações e inicializa os logs;
 2. inicializa o banco (migrations) e garante os registros base
    (astronauta, missão, participação e módulo — exigidos pelas FKs);
 3. monta os repositórios, o protocolo de avaliação e o cliente
    OPCIONAL da IA copiloto (Ollama indisponível nunca bloqueia nada);
 4. inicia o motor de monitoramento: coleta a cada 5 s, avaliação
    determinística no intervalo configurado e orientação da IA em
    background, sem sobreposição.
*/

use std::sync::Arc;

use anyhow::{Context, anyhow};
use sqlx::SqlitePool;

use back_end::application::collection::collect_measurement::ColetaMedicao;
use back_end::application::copilot::generate_guidance::GeradorOrientacao;
use back_end::application::evaluation::process_assessment::ProcessamentoAvaliacao;
use back_end::config::settings::Settings;
use back_end::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
use back_end::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;
use back_end::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;
use back_end::infrastructure::database::sqlite_initialization::init_db;
use back_end::infrastructure::sensors::motor::{DependenciasMotor, start_monitoring};
use back_end::infrastructure::sensors::sensor_adapter::SensorAdapter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ===== 1. Configuração e logs =====
    let settings = Settings::carregar().map_err(|e| anyhow!("configuração inválida: {e}"))?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    tracing::info!(
        "iniciando o Space Health (banco em '{}')",
        settings.banco.caminho
    );

    // ===== 2. Banco e registros base =====
    let pool = init_db(&settings.banco.caminho)
        .await
        .context("falha ao inicializar o banco de dados")?;
    garantir_registros_base(&pool, &settings).await?;

    // ===== 3. Repositórios, protocolo e IA copiloto =====
    let measurements: Arc<
        dyn back_end::domain::repositories::measurement_repository::MeasurementRepository,
    > = Arc::new(SqliteMeasurementRepository::new(pool.clone()));
    let evaluations: Arc<
        dyn back_end::domain::repositories::evaluation_repository::EvaluationRepository,
    > = Arc::new(SqliteEvaluationRepository::new(pool.clone()));
    let alerts: Arc<dyn back_end::domain::repositories::alert_repository::AlertRepository> =
        Arc::new(SqliteAlertRepository::new(pool.clone()));

    let coleta = Arc::new(ColetaMedicao::novo(measurements.clone()));

    let processamento = Arc::new(
        ProcessamentoAvaliacao::novo(
            measurements.clone(),
            evaluations.clone(),
            alerts.clone(),
            settings.protocolo(),
            settings.analise.janela_analise.as_secs() as i64,
            settings.analise.limite_avaliacoes_anteriores,
            settings.analise.fase_missao,
        )
        .map_err(|e| anyhow!("protocolo de avaliação inválido: {e}"))?,
    );

    // IA copiloto OPCIONAL: `Ok(None)` = desativada ou inconfigurável.
    let cliente_ia = settings
        .cliente_ia()
        .map_err(|e| anyhow!("configuração da IA inválida: {e}"))?;
    match &cliente_ia {
        Some(cliente) => {
            tracing::info!("IA copiloto configurada ({})", cliente.base_url());
        }
        None => tracing::info!("IA copiloto desativada (OLLAMA_ENABLED=false ou ausente)"),
    }
    let copiloto = Arc::new(GeradorOrientacao::novo(
        cliente_ia,
        evaluations.clone(),
        alerts.clone(),
    ));

    // ===== 4. Motor de monitoramento =====
    let sensor = SensorAdapter::new(
        settings.coleta.astronauta_id.clone(),
        settings.coleta.missao_id.clone(),
        settings.coleta.modulo_id.clone(),
    );
    let deps = Arc::new(DependenciasMotor {
        avaliacao: processamento,
        copiloto,
        measurements: measurements.clone(), // repositório de medições
        astronauta_id: settings.coleta.astronauta_id.clone(),
        missao_id: settings.coleta.missao_id.clone(),
        intervalo_avaliacao: settings.analise.intervalo_avaliacao,
        intervalo_coleta: settings.coleta.intervalo,
    });


    tracing::info!(
        "ciclo de coleta: {} s | intervalo de avaliação: {} s",
        settings.coleta.intervalo.as_secs(),
        settings.analise.intervalo_avaliacao.as_secs()
    );

    start_monitoring(sensor, deps).await;

    let app = interfaces::http::rotas(measurements.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await?;

    axum::serve(listener, app).await?;
    Ok(())
}

/// Garante astronauta, missão, participação e módulo de coleta (FKs exigem).
/// `INSERT OR IGNORE`: registros já existentes NUNCA são sobrescritos.
async fn garantir_registros_base(pool: &SqlitePool, settings: &Settings) -> anyhow::Result<()> {
    // Valores de preenchimento dos registros simulados (jamais sobrescrevem
    // dados existentes — a ausência de alteração é a semântica do OR IGNORE).
    const NOME_SIMULADO: &str = "Astronauta simulado";
    const NASCIMENTO_SIMULADO: &str = "1990-01-01";
    const MISSAO_SIMULADA: &str = "Missão de monitoramento";
    const FUNCAO_SIMULADA: &str = "Tripulante";
    const MODULO_SIMULADO: &str = "Módulo de coleta";

    sqlx::query("INSERT OR IGNORE INTO astronautas (id, nome, data_nascimento) VALUES (?, ?, ?)")
        .bind(&settings.coleta.astronauta_id)
        .bind(NOME_SIMULADO)
        .bind(NASCIMENTO_SIMULADO)
        .execute(pool)
        .await
        .context("falha ao garantir o astronauta base")?;

    if let Some(missao_id) = settings.coleta.missao_id.as_deref() {
        sqlx::query("INSERT OR IGNORE INTO missoes (id, nome_missao) VALUES (?, ?)")
            .bind(missao_id)
            .bind(MISSAO_SIMULADA)
            .execute(pool)
            .await
            .context("falha ao garantir a missão base")?;

        sqlx::query(
            "INSERT OR IGNORE INTO participacao_missao
                (id, astronauta_id, missao_id, funcao) VALUES (?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&settings.coleta.astronauta_id)
        .bind(missao_id)
        .bind(FUNCAO_SIMULADA)
        .execute(pool)
        .await
        .context("falha ao garantir a participação na missão")?;

        sqlx::query("INSERT OR IGNORE INTO modulos_nave (id, missao_id, nome) VALUES (?, ?, ?)")
            .bind(&settings.coleta.modulo_id)
            .bind(missao_id)
            .bind(MODULO_SIMULADO)
            .execute(pool)
            .await
            .context("falha ao garantir o módulo de coleta")?;
    } else {
        tracing::warn!(
            "SENSOR_MISSAO_ID não configurado: o módulo de coleta '{}' não pôde \
             ser registrado (exige missão) — medições ambientais falharão",
            settings.coleta.modulo_id
        );
    }

    Ok(())
}
