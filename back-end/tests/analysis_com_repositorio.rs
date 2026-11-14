//! Testes das funções de `analysis` que buscam dados do repositório.
//!
//! Monta um banco SQLite em memória com o schema das migrations e um
//! dataset mínimo, e exercita as variações `*_de_repositorio` de cada
//! módulo de análise.

use back_end::analysis::baseline::{BaselineConfig, calcular_baseline_de_repositorio};
use back_end::analysis::data_quality::{DataQualityConfig, evaluate_data_quality_de_repositorio};
use back_end::analysis::environmental_exposure::{
    EnvironmentalThresholds, calculate_environmental_exposure_de_repositorio,
};
use back_end::analysis::persistence::{
    CriterioAlteracao, PersistenceConfig, evaluate_persistence_de_repositorio,
};
use back_end::analysis::repository::{FaseMissao, Indicador};
use back_end::analysis::trend::{TrendConfig, analyze_trend_de_repositorio};
use back_end::analysis::variation::{
    ContextoAtual, VariationConfig, calcular_variacao_de_repositorio,
};
use back_end::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

const INDICADOR_FC: &str = "FREQUENCIA_CARDIACA";

fn ts(valor: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(valor)
        .unwrap()
        .with_timezone(&Utc)
}

async fn pool_com_dataset() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("conectar");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    sqlx::query(
        "INSERT INTO astronautas (id, nome, data_nascimento) VALUES ('A1', 'Kalvis', '1990-05-01')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO missoes (id, nome_missao, data_lancamento) VALUES ('M1', 'Artemis III', '2025-01-10T00:00:00Z')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO participacao_missao (id, astronauta_id, missao_id, funcao) VALUES ('PM1', 'A1', 'M1', 'COMANDANTE')")
        .execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO modulos_nave (id, missao_id, nome) VALUES ('MOD1', 'M1', 'Habitat A')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO telemetria_fisiologica
            (id, astronauta_id, missao_id, timestamp_utc, frequencia_cardiaca_bpm, spo2_percentual,
             temperatura_corporal_c, origem_medicao)
         VALUES
            ('T1', 'A1', 'M1', '2025-01-15T08:00:00Z', 70.0, 98.0, 36.8, 'WEARABLE'),
            ('T2', 'A1', 'M1', '2025-01-15T12:00:00Z', 75.0, 97.0, NULL, 'WEARABLE'),
            ('T3', 'A1', 'M1', '2025-01-16T08:00:00Z', 72.0, NULL, NULL, 'WEARABLE')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO registros_diarios
            (id, astronauta_id, data_registro, horas_sono_total)
         VALUES ('R1', 'A1', '2025-01-15', 7.5)",
    )
    .execute(&pool)
    .await
    .unwrap();

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
async fn qualidade_do_dado_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let qualidade = evaluate_data_quality_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        Indicador::FrequenciaCardiaca,
        None,
        &DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca),
    )
    .await
    .unwrap();

    // 3 amostras válidas; as lacunas (4h e 20h) ficam registradas sem
    // rejeitar as amostras posteriores.
    assert_eq!(qualidade.total_samples, 3);
    assert_eq!(qualidade.valid_samples, 3);
    assert!(qualidade.acceptable_quality);
    assert_eq!(qualidade.temporal_gaps, 2);
    assert_eq!(qualidade.largest_gap_seconds, 20 * 3600);
}

#[tokio::test]
async fn baseline_calculado_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    let janela = back_end::analysis::data_quality::JanelaTempo {
        inicio: ts("2025-01-15T00:00:00Z"),
        fim: ts("2025-01-17T00:00:00Z"),
    };

    let baseline = calcular_baseline_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        Indicador::FrequenciaCardiaca,
        FaseMissao::PreVoo,
        janela,
        &DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca),
        &BaselineConfig {
            k_mad: 3.0,
            minimum_samples_after_filter: 3,
        },
        ts("2025-01-17T00:00:00Z"),
    )
    .await
    .unwrap();

    assert_eq!(baseline.fase_missao, "PRE_VOO");
    assert_eq!(baseline.missao_id.as_deref(), Some("M1"));
    assert_eq!(baseline.astronauta_id, "A1");
    assert_eq!(baseline.indicador, INDICADOR_FC);
    assert_eq!(baseline.quantidade_amostras, Some(3));
    assert!((baseline.valor_referencia - (70.0 + 75.0 + 72.0) / 3.0).abs() < 1e-9);
    assert!(baseline.desvio_padrao.unwrap() > 0.0);
    assert_eq!(
        baseline.periodo_inicio.as_deref(),
        Some("2025-01-15T08:00:00Z")
    );
    assert_eq!(
        baseline.periodo_fim.as_deref(),
        Some("2025-01-16T08:00:00Z")
    );
}

#[tokio::test]
async fn variacao_usa_o_baseline_individual() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    // Baseline calculado na própria série: média 72.33, dp > 0.
    let janela = back_end::analysis::data_quality::JanelaTempo {
        inicio: ts("2025-01-15T00:00:00Z"),
        fim: ts("2025-01-17T00:00:00Z"),
    };
    let baseline = calcular_baseline_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        Indicador::FrequenciaCardiaca,
        FaseMissao::PreVoo,
        janela,
        &DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca),
        &BaselineConfig {
            k_mad: 3.0,
            minimum_samples_after_filter: 3,
        },
        ts("2025-01-17T00:00:00Z"),
    )
    .await
    .unwrap();

    // Janela atual igual à do baseline → z = 0 → Normal.
    let resultado = calcular_variacao_de_repositorio(
        &repo,
        ContextoAtual {
            astronauta_id: "A1",
            missao_id: Some("M1"),
            fase_missao: Some("PRE_VOO"),
            agora: ts("2025-01-17T00:00:00Z"),
        },
        Some("M1"),
        Indicador::FrequenciaCardiaca,
        &baseline,
        janela,
        &DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca),
        &VariationConfig {
            z_atencao: 2.0,
            z_critico: 3.0,
            max_idade_baseline: Some(chrono::Duration::days(30)),
        },
    )
    .await
    .unwrap();

    assert_eq!(resultado.quantidade_amostras, 3);
    assert!(resultado.z_score.unwrap().abs() < 1e-9);
    assert_eq!(
        resultado.nivel,
        back_end::analysis::variation::NivelVariacao::Normal
    );
    assert_eq!(resultado.baseline_desatualizado, Some(false));

    // Baseline de outro astronauta → erro de contexto.
    let mut alheio = baseline.clone();
    alheio.astronauta_id = "A2".to_string();
    let erro = calcular_variacao_de_repositorio(
        &repo,
        ContextoAtual {
            astronauta_id: "A1",
            missao_id: Some("M1"),
            fase_missao: None,
            agora: ts("2025-01-17T00:00:00Z"),
        },
        Some("M1"),
        Indicador::FrequenciaCardiaca,
        &alheio,
        janela,
        &DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca),
        &VariationConfig {
            z_atencao: 2.0,
            z_critico: 3.0,
            max_idade_baseline: None,
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(
        erro,
        back_end::analysis::variation::VariacaoErro::ContextoDivergente {
            campo: "astronauta_id",
            ..
        }
    ));
}

#[tokio::test]
async fn tendencia_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    // 70 → 75 → 72 em 24 h: inclinação positiva pequena.
    let tendencia = analyze_trend_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FC,
        None,
        None,
        &TrendConfig::default(),
    )
    .await
    .unwrap()
    .expect("tendência");

    assert!(tendencia.slope_per_hour > 0.0);
    assert_eq!(tendencia.first_value, 70.0);
    assert_eq!(tendencia.last_value, 72.0);

    // Período mínimo de 3 dias: a série cobre 24 h → None.
    let cfg = TrendConfig {
        minimum_period_seconds: 3 * 86_400,
        ..TrendConfig::default()
    };
    assert!(
        analyze_trend_de_repositorio(&repo, "A1", Some("M1"), INDICADOR_FC, None, None, &cfg)
            .await
            .unwrap()
            .is_none()
    );

    // Indicador sem série → None.
    assert!(
        analyze_trend_de_repositorio(
            &repo,
            "A1",
            Some("M1"),
            "INEXISTENTE",
            None,
            None,
            &TrendConfig::default(),
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn persistencia_da_serie_do_repositorio() {
    let pool = pool_com_dataset().await;
    let repo = SqliteMeasurementRepository::new(pool);

    // Limiar 74 → janelas [falso, verdadeiro, falso]: pico pontual.
    let persistencia = evaluate_persistence_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FC,
        None,
        None,
        &CriterioAlteracao::AcimaDe(74.0),
        &PersistenceConfig {
            janela_segundos: 0,
            min_persistence_percentage: 30.0,
            min_consecutive_windows: 2,
        },
        None,
    )
    .await
    .unwrap()
    .expect("persistência");

    assert_eq!(persistencia.total_windows, 3);
    assert_eq!(persistencia.altered_windows, 1);
    assert!(!persistencia.is_persistent);

    // Critério de limite inferior (SpO2 <= 98): a série tem dois valores
    // (98 e 97; o terceiro é NULL e não entra) → [verdadeiro, verdadeiro].
    let persistencia = evaluate_persistence_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        "SPO2",
        None,
        None,
        &CriterioAlteracao::AbaixoDe(98.0),
        &PersistenceConfig {
            janela_segundos: 0,
            min_persistence_percentage: 50.0,
            min_consecutive_windows: 2,
        },
        None,
    )
    .await
    .unwrap()
    .expect("persistência de SpO2");

    assert_eq!(persistencia.altered_windows, 2);
    assert!(persistencia.is_persistent);

    // Com verificação de qualidade: amostras suficientes → analisável.
    let com_qualidade = evaluate_persistence_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FC,
        None,
        None,
        &CriterioAlteracao::AcimaDe(74.0),
        &PersistenceConfig {
            janela_segundos: 0,
            min_persistence_percentage: 30.0,
            min_consecutive_windows: 2,
        },
        Some(&DataQualityConfig {
            minimum_samples: 3,
            ..DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca)
        }),
    )
    .await
    .unwrap();
    assert!(com_qualidade.is_some());

    // Exigindo mais amostras do que existem → Ok(None) (não analisável).
    let sem_dados = evaluate_persistence_de_repositorio(
        &repo,
        "A1",
        Some("M1"),
        INDICADOR_FC,
        None,
        None,
        &CriterioAlteracao::AcimaDe(74.0),
        &PersistenceConfig {
            janela_segundos: 0,
            min_persistence_percentage: 30.0,
            min_consecutive_windows: 2,
        },
        Some(&DataQualityConfig {
            minimum_samples: 10,
            ..DataQualityConfig::padrao_para(Indicador::FrequenciaCardiaca)
        }),
    )
    .await
    .unwrap();
    assert!(sem_dados.is_none());

    // Indicador desconhecido → Ok(None).
    assert!(
        evaluate_persistence_de_repositorio(
            &repo,
            "A1",
            Some("M1"),
            "INEXISTENTE",
            None,
            None,
            &CriterioAlteracao::AcimaDe(74.0),
            &PersistenceConfig {
                janela_segundos: 0,
                min_persistence_percentage: 30.0,
                min_consecutive_windows: 1,
            },
            None,
        )
        .await
        .unwrap()
        .is_none()
    );
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
            pressure_kpa_min: 95.0,
            pressure_kpa_max: 105.0,
        },
        14_400, // gap máximo de 4h: apenas o par 08:00 -> 12:00 conta
    )
    .await
    .unwrap();

    // 4h de exposição: excesso médio de CO2 (200+500)/2 = 350 ppm.
    assert!((resultado.co2_exposure_ppm_hours - 1400.0).abs() < 1e-6);
    // As duas leituras estão acima de 1000 ppm → 4h acima do limite.
    assert_eq!(resultado.co2_time_above_seconds, 14_400.0);
    assert!((resultado.radiation_dose_msv - ((0.15 + 0.20) / 2.0) * 4.0).abs() < 1e-6);
    assert!((resultado.temperature_exposure_degree_hours - 3.0).abs() < 1e-6);
    // Pressão sempre dentro da faixa → zero.
    assert!(resultado.pressure_exposure_kpa_hours.abs() < 1e-9);
    assert_eq!(resultado.pressure_time_outside_range_seconds, 0.0);
    // Tempo analisado por indicador (todos com os dois pares presentes).
    for valor in [
        resultado.co2_analyzed_seconds,
        resultado.radiation_analyzed_seconds,
        resultado.temperature_analyzed_seconds,
        resultado.pressure_analyzed_seconds,
    ] {
        assert_eq!(valor, 14_400.0);
    }
    assert_eq!(resultado.analyzed_seconds, 14_400.0);
}
