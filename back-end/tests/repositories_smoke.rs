//! Teste de fumaça da camada de repositórios SQLite.
//!
//! Monta um banco em memória com o schema das migrations, popula um dataset
//! mínimo e exercita todos os métodos de leitura dos quatro repositórios.

use back_end::domain::entities::alert_entity::AlertaSaude;
use back_end::domain::entities::measurement_entity::TelemetriaFisiologica;
use back_end::domain::repositories::alert_repository::AlertRepository;
use back_end::domain::repositories::astronaut_repository::AstronautRepository;
use back_end::domain::repositories::measurement_repository::MeasurementRepository;
use back_end::domain::repositories::mission_repository::MissionRepository;
use back_end::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
use back_end::infrastructure::database::repositories::astronaut_repository::SqliteAstronautRepository;
use back_end::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;
use back_end::infrastructure::database::repositories::mission_repository::SqliteMissionRepository;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn pool_com_dataset() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("conectar");

    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");

    // ===== Dados de exemplo =====
    sqlx::query("INSERT INTO astronautas (id, nome, data_nascimento) VALUES ('A1', 'Kalvis', '1990-05-01')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO missoes (id, nome_missao, data_lancamento) VALUES ('M1', 'Artemis III', '2025-01-10T00:00:00Z')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO participacao_missao (id, astronauta_id, missao_id, funcao) VALUES ('PM1', 'A1', 'M1', 'COMANDANTE')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO modulos_nave (id, missao_id, nome) VALUES ('MOD1', 'M1', 'Habitat A')")
        .execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO telemetria_fisiologica
            (id, astronauta_id, missao_id, timestamp_utc, frequencia_cardiaca_bpm, spo2_percentual,
             temperatura_corporal_c, taxa_respiratoria_rpm, hrv_ms, pressao_sistolica_mmhg,
             pressao_diastolica_mmhg, glicose_mg_dl, origem_medicao)
         VALUES
            ('T1', 'A1', 'M1', '2025-01-15T08:00:00Z', 70.0, 98.0, 36.8, 14.0, 50.0, 120.0, 80.0, 95.0, 'WEARABLE'),
            ('T2', 'A1', 'M1', '2025-01-15T12:00:00Z', 75.0, 97.0, NULL, NULL, NULL, NULL, NULL, NULL, 'WEARABLE'),
            ('T3', 'A1', 'M1', '2025-01-16T08:00:00Z', 72.0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'WEARABLE')",
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO registros_diarios
            (id, astronauta_id, data_registro, horas_sono_total, qualidade_sono_score,
             exercicio_resistencia_min, exercicio_aerobico_min, ingestao_agua_ml,
             calorias_consumidas, score_teste_cognitivo, nivel_estresse_subjetivo,
             humor_score, fadiga_score)
         VALUES ('R1', 'A1', '2025-01-15', 7.5, 8, 30, 45, 2500, 2200, 88.0, 3, 7, 4)",
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO baseline_fisiologico
            (id, astronauta_id, indicador, fase_missao, missao_id, valor_referencia,
             desvio_padrao, quantidade_amostras, atualizado_em)
         VALUES ('B1', 'A1', 'frequencia_cardiaca_bpm', 'EM_VOO', 'M1', 65.0, 8.0, 100, '2025-01-01T00:00:00Z'),
                ('B2', 'A1', 'spo2_percentual', 'EM_VOO', 'M1', 98.0, 1.0, 100, '2025-01-01T00:00:00Z')",
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO ambiente_nave (id, modulo_id, timestamp_utc, co2_ppm, radiacao_msv_hora, temperatura_cabine_c, pressao_kpa)
         VALUES ('ENV1', 'MOD1', '2025-01-15T08:00:00Z', 1200.0, 0.15, 21.5, 101.3),
                ('ENV2', 'MOD1', '2025-01-15T12:00:00Z', 1500.0, 0.20, 22.0, 101.2)",
    ).execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO presenca_modulo (id, astronauta_id, modulo_id, inicio_utc) VALUES ('P1', 'A1', 'MOD1', '2025-01-15T07:00:00Z')")
        .execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO exames_clinicos (id, astronauta_id, missao_id, fase_missao, data_coleta, categoria, resultados_json)
         VALUES ('EX1', 'A1', 'M1', 'EM_VOO', '2025-01-14', 'SANGUE', json_object('hemoglobina', 14.2))",
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO avaliacoes_saude (id, astronauta_id, missao_id, timestamp_utc, nivel_risco, resumo, indicadores_analisados_json)
         VALUES ('AL1', 'A1', 'M1', '2025-01-15T08:05:00Z', 'ATENCAO', 'Frequência cardíaca acima do baseline.', json_array('frequencia_cardiaca_bpm'))",
    ).execute(&pool).await.unwrap();

    pool
}

#[tokio::test]
async fn astronautas_find_e_list() {
    let pool = pool_com_dataset().await;
    let repo = SqliteAstronautRepository::new(pool);

    let astro = repo.find_by_id("A1").await.unwrap().expect("encontrar A1");
    assert_eq!(astro.nome, "Kalvis");
    assert!(repo.find_by_id("inexistente").await.unwrap().is_none());

    let lista = repo.list(10, 0).await.unwrap();
    assert_eq!(lista.len(), 1);
}

#[tokio::test]
async fn missoes_todas_as_queries() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMissionRepository::new(pool);

    assert!(repo.find_by_id("M1").await.unwrap().is_some());
    assert_eq!(repo.list(10, 0).await.unwrap().len(), 1);

    let participacoes = repo.list_participations_by_astronaut("A1").await.unwrap();
    assert_eq!(participacoes.len(), 1);
    assert_eq!(participacoes[0].nome_missao, "Artemis III");

    let modulos = repo.list_modules_by_mission("M1").await.unwrap();
    assert_eq!(modulos.len(), 1);
    assert_eq!(modulos[0].nome, "Habitat A");
}

#[tokio::test]
async fn medicoes_basicas() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let telemetria: TelemetriaFisiologica = repo
        .find_telemetry_by_id("T1")
        .await
        .unwrap()
        .expect("telemetria T1");
    assert_eq!(telemetria.frequencia_cardiaca_bpm, Some(70.0));

    let registro = repo.find_daily_log_by_id("R1").await.unwrap().unwrap();
    assert_eq!(registro.qualidade_sono_score, Some(8));

    let ambiente = repo.find_environment_by_id("ENV1").await.unwrap().unwrap();
    assert_eq!(ambiente.co2_ppm, Some(1200.0));

    assert_eq!(repo.list_presences("A1").await.unwrap().len(), 1);
    assert_eq!(repo.list_clinical_exams("A1").await.unwrap().len(), 1);

    let filtrados = repo
        .list_clinical_exams_filtered("A1", Some("M1"), Some("EM_VOO"), Some("SANGUE"), Some("2025-01-01"), Some("2025-01-31"), 10, 0)
        .await
        .unwrap();
    assert_eq!(filtrados.len(), 1);

    let sem_resultado = repo
        .list_clinical_exams_filtered("A1", None, None, Some("RAIO_X"), None, None, 10, 0)
        .await
        .unwrap();
    assert!(sem_resultado.is_empty());

    let baselines = repo.list_baselines("A1", Some("M1"), Some("EM_VOO")).await.unwrap();
    assert_eq!(baselines.len(), 2);
}

#[tokio::test]
async fn comparacao_com_baseline() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let comparacoes = repo
        .compare_telemetry_with_baseline("A1", "EM_VOO", Some("M1"))
        .await
        .unwrap();
    assert_eq!(comparacoes.len(), 9, "um resultado por indicador suportado");

    let frequencia = comparacoes
        .iter()
        .find(|c| c.indicador == "frequencia_cardiaca_bpm")
        .expect("indicador frequencia");
    assert_eq!(frequencia.valor_baseline, Some(65.0));
    assert_eq!(frequencia.quantidade_amostras, Some(3));
    assert!((frequencia.media_observada.unwrap() - (70.0 + 75.0 + 72.0) / 3.0).abs() < 1e-9);
    assert!(frequencia.desvio_medio.unwrap() > 0.0);
    assert!(frequencia.desvio_em_z.is_some());

    // Indicador sem baseline: colunas do baseline nulas, estatísticas observadas presentes.
    let sem_baseline = comparacoes
        .iter()
        .find(|c| c.indicador == "hrv_ms")
        .expect("indicador hrv");
    assert_eq!(sem_baseline.valor_baseline, None);
    assert_eq!(sem_baseline.quantidade_amostras, Some(1));
}

#[tokio::test]
async fn contexto_e_serie_temporal() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let contexto = repo
        .get_daily_context_for_telemetry("T1")
        .await
        .unwrap()
        .expect("contexto de T1");
    assert_eq!(contexto.registro_diario_id.as_deref(), Some("R1"));
    assert_eq!(contexto.medicoes_no_dia, Some(2));
    assert!(repo.get_daily_context_for_telemetry("inexistente").await.unwrap().is_none());

    let serie = repo
        .list_indicator_time_series("A1", Some("M1"), "frequencia_cardiaca_bpm", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(serie.len(), 3);
    assert_eq!(serie[0].valor, 70.0);

    let serie_paginada = repo
        .list_indicator_time_series("A1", None, "frequencia_cardiaca_bpm", None, None, 2, 1)
        .await
        .unwrap();
    assert_eq!(serie_paginada.len(), 2);
    assert_eq!(serie_paginada[0].valor, 75.0);
    assert_eq!(serie_paginada[1].valor, 72.0);

    let resumo = repo
        .get_indicator_summary("A1", Some("M1"), "frequencia_cardiaca_bpm", None, None)
        .await
        .unwrap()
        .expect("resumo com amostras");
    assert_eq!(resumo.quantidade_amostras, Some(3));
    assert_eq!(resumo.dias_com_registros, Some(2));
    assert!(resumo.desvio_padrao().unwrap() > 0.0);

    // Janela sem amostras -> None.
    let vazio = repo
        .get_indicator_summary("A1", Some("M1"), "frequencia_cardiaca_bpm", None, Some("2025-01-01"))
        .await
        .unwrap();
    assert!(vazio.is_none());
}

#[tokio::test]
async fn resumos_diarios_e_ambiente() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let resumos = repo
        .list_daily_summaries("A1", Some("M1"), None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(resumos.len(), 2);
    let dia15 = resumos.iter().find(|r| r.data_referencia == "2025-01-15").unwrap();
    assert_eq!(dia15.medicoes_no_dia, 2);
    assert_eq!(dia15.horas_sono_total, Some(7.5));
    assert_eq!(dia15.qualidade_sono_score, Some(8));

    let ambiente = repo
        .list_environment_by_module("MOD1", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(ambiente.len(), 2);

    let resumo_ambiente = repo
        .get_environmental_summary_by_mission("M1", None, None)
        .await
        .unwrap();
    assert_eq!(resumo_ambiente.len(), 1);
    let modulo = &resumo_ambiente[0];
    assert_eq!(modulo.nome_modulo, "Habitat A");
    assert_eq!(modulo.quantidade_amostras, Some(2));
    assert_eq!(modulo.media_co2_ppm, Some(1350.0));
    assert_eq!(modulo.max_co2_ppm, Some(1500.0));
}

#[tokio::test]
async fn painel_atual() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let painel = repo.get_current_panel("A1").await.unwrap().expect("painel");
    assert_eq!(painel.telemetria_id, "T3"); // última telemetria por timestamp
    assert_eq!(painel.nome_missao.as_deref(), Some("Artemis III"));
    assert_eq!(painel.registro_diario_id.as_deref(), Some("R1"));
    assert_eq!(painel.nome_modulo_atual.as_deref(), Some("Habitat A"));

    assert!(repo.get_current_panel("inexistente").await.unwrap().is_none());
}

#[tokio::test]
async fn alertas_todas_as_queries() {
    let pool = pool_com_dataset().await;
    let repo = SqliteAlertRepository::new(pool);

    let alerta: AlertaSaude = repo.find_by_id("AL1").await.unwrap().expect("alerta AL1");
    assert_eq!(alerta.nivel_risco, "ATENCAO");

    assert_eq!(repo.list(None, None, None, 10, 0).await.unwrap().len(), 1);
    assert_eq!(repo.list(Some("A1"), None, Some("ATENCAO"), 10, 0).await.unwrap().len(), 1);
    assert!(repo.list(Some("A1"), None, Some("CRITICO"), 10, 0).await.unwrap().is_empty());

    assert_eq!(repo.list_recent_by_astronaut("A1", 5).await.unwrap().len(), 1);

    let contagem = repo.count_by_risk_level(Some("A1"), None).await.unwrap();
    assert_eq!(contagem.len(), 1);
    assert_eq!(contagem[0].nivel_risco, "ATENCAO");
    assert_eq!(contagem[0].quantidade, 1);
}

#[tokio::test]
async fn soft_delete_eh_respeitado() {
    let pool = pool_com_dataset().await;
    let repo = SqliteAstronautRepository::new(pool.clone());
    let mission_repo = SqliteMissionRepository::new(pool.clone());
    let measurement_repo = SqliteMeasurementRepository::new(pool.clone());

    sqlx::query("UPDATE astronautas SET deleted_at = '2025-01-20T00:00:00Z' WHERE id = 'A1'")
        .execute(&pool).await.unwrap();

    assert!(repo.find_by_id("A1").await.unwrap().is_none());
    assert!(repo.list(10, 0).await.unwrap().is_empty());
    assert!(mission_repo.list_participations_by_astronaut("A1").await.unwrap().is_empty());

    sqlx::query("UPDATE telemetria_fisiologica SET deleted_at = '2025-01-20T00:00:00Z' WHERE id = 'T1'")
        .execute(&pool).await.unwrap();
    assert!(measurement_repo.find_telemetry_by_id("T1").await.unwrap().is_none());
}
