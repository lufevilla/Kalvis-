//! Testes de integração do FLUXO APLICACIONAL:
//! coleta → construção de contexto → avaliação determinística → alerta;
//! além de garantir que a IA indisponível NÃO bloqueia nada.

use chrono::{TimeZone, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

use back_end::analysis::repository::FaseMissao;
use back_end::application::collection::collect_measurement::ColetaMedicao;
use back_end::application::copilot::generate_guidance::{GeradorOrientacao, spawn_orientacao};
use back_end::application::evaluation::build_context::ConstrutorContexto;
use back_end::application::evaluation::process_assessment::ProcessamentoAvaliacao;
use back_end::config::settings::Settings;
use back_end::domain::entities::measurement_entity::{AmbienteNave, TelemetriaFisiologica};
use back_end::domain::evaluation::rules::{EstadoIndicador, RiskLevel};
use back_end::infrastructure::ai::client::OllamaClient;
use back_end::infrastructure::ai::models::DisponibilidadeIa;
use back_end::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
use back_end::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;
use back_end::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

// 1_700_000_000 = 2023-11-14T22:13:20Z.
const EPOCA: i64 = 1_700_000_000;

fn ts_iso(segundos: i64) -> String {
    // Mesmo formato do SensorAdapter e dos limites de janela
    // (`to_rfc3339`, offset "+00:00") — as queries comparam timestamps
    // lexicograficamente, então os formatos PRECISAM ser coerentes.
    Utc.timestamp_opt(EPOCA + segundos, 0).unwrap().to_rfc3339()
}

async fn pool_com_base() -> SqlitePool {
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
        "INSERT INTO astronautas (id, nome, data_nascimento) VALUES ('A1', 'K', '1990-05-01')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO missoes (id, nome_missao) VALUES ('M1', 'Artemis III')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO modulos_nave (id, missao_id, nome) VALUES ('MOD1', 'M1', 'Habitat')")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

fn repos(
    pool: &SqlitePool,
) -> (
    Arc<dyn back_end::domain::repositories::measurement_repository::MeasurementRepository>,
    Arc<dyn back_end::domain::repositories::evaluation_repository::EvaluationRepository>,
    Arc<dyn back_end::domain::repositories::alert_repository::AlertRepository>,
) {
    (
        Arc::new(SqliteMeasurementRepository::new(pool.clone())),
        Arc::new(SqliteEvaluationRepository::new(pool.clone())),
        Arc::new(SqliteAlertRepository::new(pool.clone())),
    )
}

/// Protocolo com janela de cálculo larga (os testes semeiam poucas
/// amostras) e persistência sensível a janelas de 10 s.
fn protocolo_teste() -> back_end::domain::entities::protocol_entity::ProtocoloAvaliacao {
    let mut p = Settings::from_env().expect("padrões válidos").protocolo();
    p.baseline.janela_calculo_segundos = 36_000;
    p.baseline.minimo_amostras = 2;
    // Cada amostra é uma janela de persistência (série curta de teste).
    p.persistencia.janela_segundos = 0;
    p
}

fn telemetria(id: &str, spo2: f64, offset: i64) -> TelemetriaFisiologica {
    TelemetriaFisiologica {
        id: id.to_string(),
        astronauta_id: "A1".to_string(),
        missao_id: Some("M1".to_string()),
        timestamp_utc: ts_iso(offset),
        frequencia_cardiaca_bpm: Some(72.0),
        spo2_percentual: Some(spo2),
        temperatura_corporal_c: Some(36.8),
        temperatura_pele_c: None,
        taxa_respiratoria_rpm: Some(14.0),
        hrv_ms: Some(55.0),
        pressao_sistolica_mmhg: Some(118.0),
        pressao_diastolica_mmhg: Some(76.0),
        glicose_mg_dl: Some(95.0),
        origem_medicao: Some("WEARABLE".to_string()),
        updated_at: None,
        deleted_at: None,
    }
}

fn ambiente(id: &str, offset: i64) -> AmbienteNave {
    AmbienteNave {
        id: id.to_string(),
        modulo_id: "MOD1".to_string(),
        timestamp_utc: ts_iso(offset),
        co2_ppm: Some(800.0),
        radiacao_msv_hora: Some(0.005),
        temperatura_cabine_c: Some(21.0),
        pressao_kpa: Some(101.3),
        updated_at: None,
        deleted_at: None,
    }
}

#[tokio::test]
async fn fluxo_completo_da_coleta_ate_o_alerta() {
    let pool = pool_com_base().await;
    let (measurements, evaluations, alerts) = repos(&pool);

    // Baseline individual SEMEADO (ref 98, σ 1, EM_VOO, missão M1).
    sqlx::query(
        "INSERT INTO baseline_fisiologico
            (id, astronauta_id, indicador, fase_missao, missao_id, valor_referencia,
             desvio_padrao, quantidade_amostras, atualizado_em)
         VALUES ('B1', 'A1', 'SPO2', 'EM_VOO', 'M1', 98.0, 1.0, 50, ?)",
    )
    .bind(ts_iso(0))
    .execute(&pool)
    .await
    .unwrap();

    let agora = Utc.timestamp_opt(EPOCA + 60, 0).unwrap();

    // ===== Coleta (caso de uso) =====
    let coleta = ColetaMedicao::novo(measurements.clone());
    let resultado = coleta
        .executar(telemetria("T1", 91.0, 25), ambiente("E1", 25))
        .await
        .expect("coleta válida");
    assert!(resultado.presenca_id.is_some());
    assert!(resultado.avisos.is_empty());

    // ===== Contexto =====
    let construtor = ConstrutorContexto::novo(
        measurements.clone(),
        evaluations.clone(),
        chrono::Duration::minutes(5),
        5,
        protocolo_teste().qualidade,
    );
    let contexto = construtor
        .executar("A1", Some("M1"), FaseMissao::EmVoo, agora)
        .await
        .expect("contexto");
    let spo2 = contexto
        .qualidade
        .get(&back_end::analysis::repository::Indicador::Spo2)
        .expect("SpO2 no contexto");
    assert_eq!(spo2.valid_samples, 1);
    assert!(
        contexto
            .baselines
            .contains_key(&back_end::analysis::repository::Indicador::Spo2),
        "baseline individual recuperado"
    );

    // ===== Avaliação determinística =====
    let processamento = ProcessamentoAvaliacao::novo(
        measurements.clone(),
        evaluations.clone(),
        alerts.clone(),
        protocolo_teste(),
        300,
        5,
        FaseMissao::EmVoo,
    )
    .expect("protocolo válido");

    // Primeira execução: 1 amostra < mínimo (2) → DadosInsuficientes,
    // que NUNCA eleva o risco.
    let relatorio = processamento
        .executar("A1", Some("M1"), agora)
        .await
        .expect("avaliação");
    assert_eq!(relatorio.nivel, RiskLevel::Normal);
    let evid1 = relatorio
        .evidencias
        .iter()
        .find(|e| e.indicador == "SPO2")
        .unwrap();
    assert_eq!(evid1.estado, EstadoIndicador::DadosInsuficientes);
    assert_eq!(relatorio.baselines_criados, 0, "baseline semeado é mantido");

    // ===== Segunda coleta: 2 amostras → variação z = -7 (Crítico) +
    // persistência (todas as janelas alteradas) → CRITICO com alerta. =====
    let coleta2 = ColetaMedicao::novo(measurements.clone());
    coleta2
        .executar(telemetria("T2", 91.0, 65), ambiente("E2", 65))
        .await
        .unwrap();

    let relatorio2 = processamento
        .executar("A1", Some("M1"), agora + chrono::Duration::seconds(5))
        .await
        .unwrap();
    assert_eq!(relatorio2.nivel, RiskLevel::Critico);
    let evid2 = relatorio2
        .evidencias
        .iter()
        .find(|e| e.indicador == "SPO2")
        .unwrap();
    assert_eq!(evid2.estado, EstadoIndicador::Analisado);
    assert_eq!(evid2.nivel_variacao.as_deref(), Some("Critico"));
    assert_eq!(evid2.persistente, Some(true));

    // Alerta criado e deduplicado na repetição da mesma condição.
    let alerta_id = relatorio2.alerta_id().expect("alerta criado");
    let relatorio3 = processamento
        .executar("A1", Some("M1"), agora + chrono::Duration::seconds(15))
        .await
        .unwrap();
    assert_eq!(relatorio3.alerta_id().as_deref(), Some(alerta_id.as_str()));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'ATIVO'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    // Histórico completo de avaliações.
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM avaliacoes_saude")
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
}

#[tokio::test]
async fn ia_indisponivel_nao_bloqueia_o_fluxo() {
    let pool = pool_com_base().await;
    let (measurements, evaluations, alerts) = repos(&pool);

    let coleta = ColetaMedicao::novo(measurements.clone());
    coleta
        .executar(telemetria("T1", 98.0, 25), ambiente("E1", 25))
        .await
        .unwrap();

    let processamento = ProcessamentoAvaliacao::novo(
        measurements.clone(),
        evaluations.clone(),
        alerts.clone(),
        protocolo_teste(),
        300,
        5,
        FaseMissao::EmVoo,
    )
    .unwrap();
    let relatorio = processamento
        .executar("A1", Some("M1"), Utc.timestamp_opt(EPOCA + 60, 0).unwrap())
        .await
        .unwrap();

    // IA configurada mas INACESSÍVEL (porta reservada 1).
    let cliente = OllamaClient::novo("http://127.0.0.1:1", "modelo-teste", 2).unwrap();
    let copiloto = Arc::new(GeradorOrientacao::novo(
        Some(cliente),
        evaluations.clone(),
        alerts.clone(),
    ));

    let alerta_id = relatorio.alerta_id();
    let solicitacao = relatorio.orientacao.expect("solicitação gerada");
    let resposta = copiloto.gerar(&solicitacao).await;
    assert!(!resposta.disponivel);
    assert!(matches!(
        resposta.disponibilidade,
        DisponibilidadeIa::Indisponivel(_)
    ));

    // Mesmo indisponível, `processar` NUNCA falha o fluxo.
    let resposta = copiloto
        .processar(&solicitacao, alerta_id.as_deref())
        .await
        .unwrap();
    assert!(!resposta.disponivel);

    // A avaliação segue persistida e inalterada (IA indisponível não escreve).
    let gravada = evaluations
        .buscar_por_id(&relatorio.avaliacao.id)
        .await
        .unwrap()
        .expect("avaliação");
    assert_eq!(gravada.nivel_risco, "NORMAL");
    assert!(gravada.explicacao.is_none() && gravada.orientacao.is_none());
}

#[tokio::test]
async fn spawn_da_orientacao_respeita_a_trava_e_desarma_a_flag() {
    let pool = pool_com_base().await;
    let (_measurements, evaluations, alerts) = repos(&pool);

    evaluations
        .salvar(
            &back_end::domain::entities::evaluation_entity::AvaliacaoSaude {
                id: "AV1".into(),
                astronauta_id: "A1".into(),
                missao_id: None,
                timestamp_utc: "2023-11-14T22:13:00Z".into(),
                nivel_risco: "ATENCAO".into(),
                resumo: "resumo determinístico".into(),
                indicadores_analisados_json: "[]".into(),
                fatores_contextuais_json: None,
                explicacao: None,
                orientacao: None,
                updated_at: None,
                deleted_at: None,
            },
        )
        .await
        .unwrap();

    // Sem cliente configurado: spawn devolve false.
    let copiloto = Arc::new(GeradorOrientacao::novo(None, evaluations.clone(), alerts));

    let solicitacao = back_end::infrastructure::ai::models::SolicitacaoOrientacao {
        astronauta_id: "A1".into(),
        avaliacao_id: "AV1".into(),
        timestamp_utc: "2023-11-14T22:13:00Z".into(),
        nivel_risco: "ATENCAO".into(),
        inconclusivo: false,
        resumo_deterministico: "resumo determinístico".into(),
        justificativas: vec!["SPO2: variação nível Atencao (z=-2.20).".into()],
        evidencias: Vec::new(),
        fatores_contextuais: None,
    };

    assert!(!spawn_orientacao(copiloto.clone(), &solicitacao, None));

    // A avaliação NÃO foi alterada (nenhuma orientação em curso).
    let depois = evaluations.buscar_por_id("AV1").await.unwrap().unwrap();
    assert!(depois.explicacao.is_none() && depois.orientacao.is_none());
}
