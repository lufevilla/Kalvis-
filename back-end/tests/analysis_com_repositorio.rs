//! Testes das funções de `analysis` que buscam dados do repositório.
//!
//! Monta um banco SQLite em memória com o schema das migrations e um
//! dataset mínimo, e exercita as variações `*_de_repositorio` de cada
//! módulo de análise.

use back_end::analysis::baseline::{
    comparar_sono_com_pre_voo_de_repositorio, comparar_telemetria_com_pre_voo_de_repositorio,
    calcular_baseline_pre_voo_de_repositorio, INDICADOR_FREQUENCIA_CARDIACA, INDICADOR_HORAS_SONO,
};
use back_end::analysis::data_quality::{evaluate_data_quality_de_repositorio, DataQualityConfig};
use back_end::analysis::environmental_exposure::{
    calculate_environmental_exposure_de_repositorio, EnvironmentalThresholds,
};
use back_end::analysis::persistence::evaluate_persistence_de_repositorio;
use back_end::analysis::statistics::{
    median_de_repositorio, moving_average_de_repositorio, pearson_correlation_de_repositorio,
    standard_deviation_de_repositorio,
};
use back_end::analysis::trend::analyze_trend_de_repositorio;
use back_end::domain::repositories::measurement_repository::MeasurementRepository;
use back_end::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn pool_com_dataset() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("conectar");

    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");

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
             temperatura_corporal_c, origem_medicao)
         VALUES
            ('T1', 'A1', 'M1', '2025-01-15T08:00:00Z', 70.0, 98.0, 36.8, 'WEARABLE'),
            ('T2', 'A1', 'M1', '2025-01-15T12:00:00Z', 75.0, 97.0, NULL, 'WEARABLE'),
            ('T3', 'A1', 'M1', '2025-01-16T08:00:00Z', 72.0, NULL, NULL, 'WEARABLE')",
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO registros_diarios
            (id, astronauta_id, data_registro, horas_sono_total)
         VALUES ('R1', 'A1', '2025-01-15', 7.5)",
    ).execute(&pool).await.unwrap();

    // Baselines pré-voô com os nomes oficiais de indicador (maiúsculas).
    sqlx::query(
        "INSERT INTO baseline_fisiologico
            (id, astronauta_id, indicador, fase_missao, missao_id, valor_referencia,
             desvio_padrao, quantidade_amostras, atualizado_em)
         VALUES ('B1', 'A1', 'FREQUENCIA_CARDIACA', 'PRE_VOO', 'M1', 70.0, 5.0, 50, '2025-01-01T00:00:00Z'),
                ('B2', 'A1', 'HORAS_SONO', 'PRE_VOO', NULL, 8.0, 1.0, 50, '2025-01-01T00:00:00Z')",
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO ambiente_nave (id, modulo_id, timestamp_utc, co2_ppm, radiacao_msv_hora, temperatura_cabine_c, pressao_kpa)
         VALUES ('ENV1', 'MOD1', '2025-01-15T08:00:00Z', 1200.0, 0.15, 21.5, 101.3),
                ('ENV2', 'MOD1', '2025-01-15T12:00:00Z', 1500.0, 0.20, 22.0, 101.2)",
    ).execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO presenca_modulo (id, astronauta_id, modulo_id, inicio_utc) VALUES ('P1', 'A1', 'MOD1', '2025-01-15T07:00:00Z')")
        .execute(&pool).await.unwrap();

    pool
}

#[tokio::test]
async fn baseline_pre_voo_calculado_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let baseline = calcular_baseline_pre_voo_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
        3,
        "2025-01-17T00:00:00Z",
    )
    .await
    .unwrap()
    .expect("baseline com amostras suficientes");

    assert_eq!(baseline.fase_missao, "PRE_VOO");
    assert_eq!(baseline.quantidade_amostras, Some(3));
    assert!((baseline.valor_referencia - (70.0 + 75.0 + 72.0) / 3.0).abs() < 1e-9);
    assert_eq!(
        baseline.periodo_inicio.as_deref(),
        Some("2025-01-15T08:00:00Z")
    );
    assert_eq!(
        baseline.periodo_fim.as_deref(),
        Some("2025-01-16T08:00:00Z")
    );

    // Amostras insuficientes -> None.
    let insuficiente = calcular_baseline_pre_voo_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
        10,
        "2025-01-17T00:00:00Z",
    )
    .await
    .unwrap();
    assert!(insuficiente.is_none());

    // Indicador sem série (ou desconhecido) -> None.
    let vazio = calcular_baseline_pre_voo_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        "INDICADOR_INEXISTENTE",
        None,
        None,
        1,
        "2025-01-17T00:00:00Z",
    )
    .await
    .unwrap();
    assert!(vazio.is_none());
}

#[tokio::test]
async fn comparacao_da_telemetria_usa_baselines_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let telemetria = repo
        .find_telemetry_by_id("T2")
        .await
        .unwrap()
        .expect("telemetria T2");

    let comparacoes = comparar_telemetria_com_pre_voo_de_repositorio(&repo, &telemetria)
        .await
        .unwrap();

    let frequencia = comparacoes
        .iter()
        .find(|c| c.indicador == INDICADOR_FREQUENCIA_CARDIACA)
        .expect("comparação de frequência cardíaca");

    assert_eq!(frequencia.valor_atual, 75.0);
    assert_eq!(frequencia.valor_referencia_pre_voo, 70.0);
    assert_eq!(frequencia.diferenca, 5.0);
    assert_eq!(frequencia.z_score, Some(1.0));
    assert_eq!(
        comparacoes
            .iter()
            .find(|c| c.indicador == INDICADOR_FREQUENCIA_CARDIACA)
            .unwrap()
            .direcao,
        back_end::analysis::baseline::DesvioDirecao::Acima
    );

    // spo2 não tem baseline pré-voô no dataset: não entra na comparação.
    assert!(!comparacoes.iter().any(|c| c.indicador == "SPO2"));
}

#[tokio::test]
async fn comparacao_do_sono_busca_registro_e_baseline() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let comparacao = comparar_sono_com_pre_voo_de_repositorio(&repo, "R1")
        .await
        .unwrap()
        .expect("comparação de sono");

    assert_eq!(comparacao.indicador, INDICADOR_HORAS_SONO);
    assert_eq!(comparacao.valor_atual, 7.5);
    assert_eq!(comparacao.valor_referencia_pre_voo, 8.0);
    assert_eq!(comparacao.diferenca, -0.5);
    assert_eq!(comparacao.z_score, Some(-0.5));
    assert_eq!(
        comparacao.direcao,
        back_end::analysis::baseline::DesvioDirecao::Abaixo
    );

    // Registro inexistente -> None.
    assert!(comparar_sono_com_pre_voo_de_repositorio(&repo, "inexistente")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn tendencia_persistencia_e_qualidade_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    // Tendência: 70 -> 75 -> 72 em ~24h, inclinação positiva pequena.
    let tendencia = analyze_trend_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
        0.0,
    )
    .await
    .unwrap()
    .expect("tendência");

    assert!(tendencia.slope_per_hour > 0.0);
    assert_eq!(tendencia.first_value, 70.0);
    assert_eq!(tendencia.last_value, 72.0);

    // Persistência: limiar 74 -> janelas [falso, verdadeiro, falso].
    let persistencia = evaluate_persistence_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
        74.0,
        30.0,
        2,
    )
    .await
    .unwrap()
    .expect("persistência");

    assert_eq!(persistencia.total_windows, 3);
    assert_eq!(persistencia.altered_windows, 1);
    assert!(!persistencia.is_persistent);

    // Qualidade do dado: 3 amostras válidas sem lacunas.
    let qualidade = evaluate_data_quality_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
        &DataQualityConfig {
            minimum_samples: 3,
            minimum_valid_percentage: 100.0,
            max_gap_seconds: 86_400,
            min_value: None,
            max_value: None,
            max_step: None,
        },
    )
    .await
    .unwrap()
    .expect("qualidade");

    assert_eq!(qualidade.total_samples, 3);
    assert_eq!(qualidade.valid_samples, 3);
    assert!(qualidade.acceptable_quality);

    // Indicador sem série -> None em todas as variações.
    assert!(analyze_trend_de_repositorio(
        &repo, "A1", Some("M1"), "INEXISTENTE", None, None, 0.0
    )
    .await
    .unwrap()
    .is_none());
    assert!(evaluate_persistence_de_repositorio(
        &repo, "A1", Some("M1"), "INEXISTENTE", None, None, 74.0, 30.0, 1
    )
    .await
    .unwrap()
    .is_none());
    assert!(evaluate_data_quality_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        "INEXISTENTE",
        None,
        None,
        &DataQualityConfig {
            minimum_samples: 1,
            minimum_valid_percentage: 100.0,
            max_gap_seconds: 86_400,
            min_value: None,
            max_value: None,
            max_step: None,
        },
    )
    .await
    .unwrap()
    .is_none());
}

#[tokio::test]
async fn estatisticas_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let media = moving_average_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
    )
    .await
    .unwrap()
    .expect("média");

    assert!((media - (70.0 + 75.0 + 72.0) / 3.0).abs() < 1e-9);

    let desvio = standard_deviation_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
    )
    .await
    .unwrap()
    .expect("desvio padrão");

    assert!(desvio > 0.0);

    let mediana = median_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        None,
        None,
    )
    .await
    .unwrap()
    .expect("mediana");

    assert_eq!(mediana, 72.0);

    // Correlação FC x SpO2: somente 08:00 e 12:00 têm os dois valores
    // (70, 75) x (98, 97) -> correlação -1.0.
    let correlacao = pearson_correlation_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FREQUENCIA_CARDIACA,
        "SPO2",
        None,
        None,
    )
    .await
    .unwrap()
    .expect("correlação");

    assert!((correlacao - (-1.0)).abs() < 1e-9);
}

#[tokio::test]
async fn exposicao_ambiental_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let resultado = calculate_environmental_exposure_de_repositorio(
        &repo,
        "A1",
        &EnvironmentalThresholds {
            co2_ppm: 1000.0,
            temperature_c: 21.0,
        },
        14_400, // gap máximo de 4h: apenas o par 08:00 -> 12:00 conta
    )
    .await
    .unwrap();

    // 4h de exposição: excesso médio de CO2 (200+500)/2 = 350 ppm.
    assert!((resultado.co2_exposure_ppm_hours - 1400.0).abs() < 1e-6);
    assert_eq!(resultado.co2_time_above_seconds, 14_400.0);
    assert!((resultado.radiation_dose_msv - ((0.15 + 0.20) / 2.0) * 4.0).abs() < 1e-6);
    assert!((resultado.temperature_exposure_degree_hours - 3.0).abs() < 1e-6);
    assert_eq!(resultado.analyzed_seconds, 14_400.0);
}
