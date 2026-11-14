//! Testes de integração das ESCRITAS dos repositórios:
//! - baselines (salvar / atualizar / buscar, incluindo o caso sem missão);
//! - avaliações de saúde (salvar / consultar / atualizar_orientacao);
//! - ciclo de vida dos alertas operacionais (tabela `alertas`, migration 008).

use chrono::{SecondsFormat, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

use back_end::domain::entities::alert_entity::EntradaAlerta;
use back_end::domain::entities::evaluation_entity::AvaliacaoSaude;
use back_end::domain::entities::measurement_entity::BaselineFisiologico;
use back_end::domain::repositories::alert_repository::AlertRepository;
use back_end::domain::repositories::evaluation_repository::EvaluationRepository;
use back_end::domain::repositories::measurement_repository::MeasurementRepository;
use back_end::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
use back_end::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;
use back_end::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

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
    pool
}

fn baseline(id: &str, fase: &str, missao: Option<&str>, referencia: f64) -> BaselineFisiologico {
    BaselineFisiologico {
        id: id.to_string(),
        astronauta_id: "A1".to_string(),
        indicador: "FREQUENCIA_CARDIACA".to_string(),
        fase_missao: fase.to_string(),
        missao_id: missao.map(str::to_string),
        valor_referencia: referencia,
        desvio_padrao: Some(4.0),
        periodo_inicio: Some("2025-01-15T08:00:00Z".to_string()),
        periodo_fim: Some("2025-01-15T09:00:00Z".to_string()),
        quantidade_amostras: Some(50),
        atualizado_em: "2025-01-15T09:00:00Z".to_string(),
        updated_at: None,
        deleted_at: None,
    }
}

fn avaliacao(id: &str, nivel: &str) -> AvaliacaoSaude {
    AvaliacaoSaude {
        id: id.to_string(),
        astronauta_id: "A1".to_string(),
        missao_id: Some("M1".to_string()),
        timestamp_utc: "2025-01-15T08:00:00Z".to_string(),
        nivel_risco: nivel.to_string(),
        resumo: "resumo determinístico".to_string(),
        indicadores_analisados_json: "[]".to_string(),
        fatores_contextuais_json: None,
        explicacao: None,
        orientacao: None,
        updated_at: None,
        deleted_at: None,
    }
}

// ============================================================
// BASELINE FISIOLÓGICO
// ============================================================

#[tokio::test]
async fn baseline_salvar_buscar_e_atualizar() {
    let pool = pool_com_base().await;
    let repo = SqliteMeasurementRepository::new(pool.clone());

    // Salvar e buscar com missão específica.
    repo.salvar_baseline(&baseline("B1", "EM_VOO", Some("M1"), 70.0))
        .await
        .unwrap();
    let encontrado = repo
        .buscar_baseline("A1", "FREQUENCIA_CARDIACA", "EM_VOO", Some("M1"))
        .await
        .unwrap()
        .expect("baseline com missão");
    assert_eq!(encontrado.id, "B1");
    assert!((encontrado.valor_referencia - 70.0).abs() < 1e-9);

    // Busca SEM missão NÃO devolve o baseline de missão específica.
    assert!(
        repo.buscar_baseline("A1", "FREQUENCIA_CARDIACA", "EM_VOO", None)
            .await
            .unwrap()
            .is_none()
    );

    // Salvar baseline sem missão (padrão pré-voo).
    repo.salvar_baseline(&baseline("B2", "PRE_VOO", None, 68.0))
        .await
        .unwrap();
    let sem_missao = repo
        .buscar_baseline("A1", "FREQUENCIA_CARDIACA", "PRE_VOO", None)
        .await
        .unwrap()
        .expect("baseline sem missão");
    assert_eq!(sem_missao.id, "B2");

    // Atualizar: MESMO id e MESMA linha (chave única do banco).
    let mut atualizado = baseline("B1", "EM_VOO", Some("M1"), 75.5);
    atualizado.desvio_padrao = Some(3.2);
    atualizado.quantidade_amostras = Some(80);
    repo.atualizar_baseline(&atualizado).await.unwrap();

    let depois = repo
        .buscar_baseline("A1", "FREQUENCIA_CARDIACA", "EM_VOO", Some("M1"))
        .await
        .unwrap()
        .expect("baseline atualizado");
    assert_eq!(depois.id, "B1");
    assert!((depois.valor_referencia - 75.5).abs() < 1e-9);
    assert_eq!(depois.desvio_padrao, Some(3.2));
    assert_eq!(depois.quantidade_amostras, Some(80));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM baseline_fisiologico")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2,
        "atualização NÃO cria nova linha"
    );
}

#[tokio::test]
async fn baseline_salvar_baselines_iguais_rejeitados_por_chave_unica() {
    let pool = pool_com_base().await;
    let repo = SqliteMeasurementRepository::new(pool);

    repo.salvar_baseline(&baseline("B1", "EM_VOO", Some("M1"), 70.0))
        .await
        .unwrap();
    // Mesmo (astronauta, indicador, fase, missão) com OUTRO id → viola UNIQUE.
    assert!(
        repo.salvar_baseline(&baseline("B2", "EM_VOO", Some("M1"), 71.0))
            .await
            .is_err()
    );
}

// ============================================================
// AVALIAÇÕES DE SAÚDE
// ============================================================

#[tokio::test]
async fn avaliacao_salvar_e_consultar() {
    let pool = pool_com_base().await;
    let repo = SqliteEvaluationRepository::new(pool.clone());

    repo.salvar(&avaliacao("AV1", "ATENCAO")).await.unwrap();
    repo.salvar(&avaliacao("AV2", "CRITICO")).await.unwrap();

    // Busca por id.
    let por_id = repo.buscar_por_id("AV2").await.unwrap().expect("AV2");
    assert_eq!(por_id.nivel_risco, "CRITICO");

    // Anteriores em ordem DECRESCENTE com corte superior.
    let anteriores = repo
        .listar_anteriores("A1", Some("2025-01-15T08:00:01Z"), 10)
        .await
        .unwrap();
    assert_eq!(anteriores.len(), 2);
    assert_eq!(anteriores[0].id, "AV2", "mais recente primeiro");

    // Limite respeitado.
    assert_eq!(
        repo.listar_anteriores("A1", None, 1).await.unwrap().len(),
        1
    );

    // Mais recente.
    assert_eq!(
        repo.mais_recente("A1").await.unwrap().expect("recente").id,
        "AV2"
    );

    // Por missão e período.
    let da_missao = repo
        .listar_por_missao_e_periodo("M1", Some("2025-01-15T00:00:00Z"), None, 10)
        .await
        .unwrap();
    assert_eq!(da_missao.len(), 2);

    // Astronauta sem avaliações → vazio (nunca erro).
    assert!(
        repo.listar_anteriores("FANTASMA", None, 5)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn avaliacao_atualizar_orientacao_nao_altera_o_nivel() {
    let pool = pool_com_base().await;
    let repo = SqliteEvaluationRepository::new(pool.clone());

    repo.salvar(&avaliacao("AV1", "ALERTA")).await.unwrap();

    repo.atualizar_orientacao(
        "AV1",
        Some("explicação da IA"),
        Some("recomendações da IA"),
        "2025-01-15T08:05:00Z",
    )
    .await
    .unwrap();

    let depois = repo.buscar_por_id("AV1").await.unwrap().expect("AV1");
    assert_eq!(depois.nivel_risco, "ALERTA", "nível imutável pela IA");
    assert_eq!(depois.resumo, "resumo determinístico");
    assert_eq!(depois.explicacao.as_deref(), Some("explicação da IA"));
    assert_eq!(depois.orientacao.as_deref(), Some("recomendações da IA"));
    assert_eq!(depois.updated_at.as_deref(), Some("2025-01-15T08:05:00Z"));
    // Evidências continuam intactas.
    assert_eq!(depois.indicadores_analisados_json, "[]");
}

// ============================================================
// ALERTAS OPERACIONAIS (tabela `alertas`)
// ============================================================

fn entrada(avaliacao_id: &str, condicao: &str, nivel: &str) -> EntradaAlerta {
    EntradaAlerta {
        avaliacao_id: avaliacao_id.to_string(),
        astronauta_id: "A1".to_string(),
        missao_id: Some("M1".to_string()),
        condicao: condicao.to_string(),
        timestamp_utc: "2025-01-15T08:00:00Z".to_string(),
        nivel_risco: nivel.to_string(),
        resumo: "resumo determinístico".to_string(),
        explicacao: None,
        orientacao: None,
        atualizado_em: "2025-01-15T08:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn alerta_ciclo_de_vida_completo() {
    let pool = pool_com_base().await;
    let repo = SqliteAlertRepository::new(pool.clone());
    let agora = |t: &str| t.to_string();

    // Avaliações referenciadas pelos alertas (FK).
    let evaluations = SqliteEvaluationRepository::new(pool.clone());
    evaluations
        .salvar(&avaliacao("AV1", "ATENCAO"))
        .await
        .unwrap();
    evaluations
        .salvar(&avaliacao("AV2", "CRITICO"))
        .await
        .unwrap();

    // Criação.
    let (alerta, criado) = repo
        .criar_ou_atualizar_alerta(&entrada("AV1", "SPO2:Abaixo", "ATENCAO"))
        .await
        .unwrap();
    assert!(criado);
    assert!(alerta.ativo());
    let id_alerta = alerta.id.clone();

    // Deduplicação: mesma condição → ATUALIZA o mesmo registro.
    let (alerta2, criado2) = repo
        .criar_ou_atualizar_alerta(&entrada("AV2", "SPO2:Abaixo", "CRITICO"))
        .await
        .unwrap();
    assert!(!criado2, "condição persistente não cria novo alerta");
    assert_eq!(alerta2.id, id_alerta, "o MESMO alerta evolui");
    assert_eq!(alerta2.nivel_risco, "CRITICO");
    assert_eq!(
        alerta2.timestamp_utc, "2025-01-15T08:00:00Z",
        "início preservado"
    );

    // Segunda condição → novo alerta ATIVO.
    let (outro, criado3) = repo
        .criar_ou_atualizar_alerta(&entrada("AV2", "HRV:Abaixo", "ATENCAO"))
        .await
        .unwrap();
    assert!(criado3);
    let id_outro = outro.id;

    // Listagem de ativos.
    let ativos = repo
        .listar_alertas_ativos(Some("A1"), Some("M1"))
        .await
        .unwrap();
    assert_eq!(ativos.len(), 2);

    // Busca por condição específica.
    let por_condicao = repo
        .buscar_alerta_ativo("A1", "HRV:Abaixo")
        .await
        .unwrap()
        .expect("ativo por condição");
    assert_eq!(por_condicao.id, id_outro);

    // Resolução individual.
    assert!(
        repo.resolver_alerta(&id_outro, &agora("2025-01-15T09:00:00Z"))
            .await
            .unwrap(),
        "resolver altera 1 registro"
    );
    assert!(
        repo.buscar_alerta_ativo("A1", "HRV:Abaixo")
            .await
            .unwrap()
            .is_none()
    );
    // Resolver de novo não faz nada (0 linhas alteradas).
    assert!(
        !repo
            .resolver_alerta(&id_outro, &agora("2025-01-15T09:00:01Z"))
            .await
            .unwrap()
    );

    // Resolução de todos os ativos do astronauta.
    let todos = repo
        .resolver_todos("A1", Some("M1"), &agora("2025-01-15T10:00:00Z"))
        .await
        .unwrap();
    assert_eq!(todos, 1);
    assert!(
        repo.listar_alertas_ativos(Some("A1"), None)
            .await
            .unwrap()
            .is_empty()
    );

    // Histórico preservado (não soft-delete: os registros continuam lá).
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'RESOLVIDO'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn alerta_nao_duplica_condicao_ativa_no_banco() {
    let pool = pool_com_base().await;
    let repo = SqliteAlertRepository::new(pool.clone());

    let evaluations = SqliteEvaluationRepository::new(pool.clone());
    evaluations
        .salvar(&avaliacao("AV1", "ATENCAO"))
        .await
        .unwrap();

    // Apenas a primeira chamada cria; as demais atualizam o mesmo alerta.
    let mut criacoes = Vec::new();
    for _ in 0..3 {
        let (_, criado) = repo
            .criar_ou_atualizar_alerta(&entrada("AV1", "SPO2:Abaixo", "ATENCAO"))
            .await
            .unwrap();
        criacoes.push(criado);
    }
    assert_eq!(criacoes, vec![true, false, false]);

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'ATIVO'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1,
        "índice único parcial: no máximo um alerta ATIVO por condição"
    );
}

#[tokio::test]
async fn alerta_atualizar_orientacao_sem_alterar_estado() {
    let pool = pool_com_base().await;
    let repo = SqliteAlertRepository::new(pool.clone());

    let evaluations = SqliteEvaluationRepository::new(pool.clone());
    evaluations
        .salvar(&avaliacao("AV1", "ATENCAO"))
        .await
        .unwrap();

    let (alerta, _) = repo
        .criar_ou_atualizar_alerta(&entrada("AV1", "SPO2:Abaixo", "ATENCAO"))
        .await
        .unwrap();

    repo.atualizar_orientacao_alerta(
        &alerta.id,
        Some("explicação da IA"),
        Some("recomendações da IA"),
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    )
    .await
    .unwrap();

    let depois = repo
        .buscar_alerta_ativo("A1", "SPO2:Abaixo")
        .await
        .unwrap()
        .expect("segue ATIVO");
    assert_eq!(
        depois.estado, "ATIVO",
        "orientação da IA não altera o estado"
    );
    assert_eq!(depois.nivel_risco, "ATENCAO");
    assert_eq!(depois.explicacao.as_deref(), Some("explicação da IA"));
    assert_eq!(depois.orientacao.as_deref(), Some("recomendações da IA"));
}
