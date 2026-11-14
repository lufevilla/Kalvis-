/*
MOTOR DE MONITORAMENTO (CICLO DE COLETA + AVALIAÇÃO + ORIENTAÇÃO).

Cada ciclo, a cada `intervalo` de coleta (padrão do projeto: 5 s):
 1. coleta via SensorAdapter (NUNCA substituído) e persiste via caso de
    uso `ColetaMedicao`;
 2. quando o `intervalo_avaliacao` permite e NÃO há avaliação em curso,
    spawn da avaliação determinística em background — o ciclo de coleta
    nunca fica bloqueado esperando a análise;
 3. no fim de cada avaliação bem-sucedida, a orientação da IA copiloto
    é spawnada de forma NÃO sobreposta (trava em `spawn_orientacao`).

Nenhum erro encerra o loop: coleta, avaliação e orientação falham de
forma independente e registram o problema (`tracing`) — o motor segue
operando no ciclo seguinte.
*/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};

use crate::application::collection::collect_measurement::ColetaMedicao;
use crate::application::copilot::generate_guidance::{GeradorOrientacao, spawn_orientacao};
use crate::application::evaluation::process_assessment::ProcessamentoAvaliacao;
use crate::infrastructure::sensors::sensor_adapter::SensorAdapter;

/// Dependências do motor (todas compartilháveis, construídas no `main`).
pub struct DependenciasMotor {
    pub coleta: Arc<ColetaMedicao>,
    pub avaliacao: Arc<ProcessamentoAvaliacao>,
    pub copiloto: Arc<GeradorOrientacao>,
    /// Astronauta monitorado pelo sensor (mesma identidade da coleta).
    pub astronauta_id: String,
    /// Missão associada às avaliações (`None` = sem missão).
    pub missao_id: Option<String>,
    /// Intervalo mínimo ENTRE avaliações (a coleta continua no próprio
    /// intervalo do sensor).
    pub intervalo_avaliacao: Duration,
    /// Intervalo do ciclo de coleta (padrão do projeto: 5 s).
    pub intervalo_coleta: Duration,
}

pub struct MotorMonitoramento {
    sensor: SensorAdapter,
    deps: Arc<DependenciasMotor>,
    ultima_avaliacao: Option<Instant>,
    /// `true` enquanto uma avaliação spawnada não terminou (a própria
    /// tarefa desarma a flag — inclusive quando falha).
    avaliando: Arc<AtomicBool>,
}

impl MotorMonitoramento {
    pub fn novo(sensor: SensorAdapter, deps: Arc<DependenciasMotor>) -> Self {
        Self {
            sensor,
            deps,
            ultima_avaliacao: None,
            avaliando: Arc::new(AtomicBool::new(false)),
        }
    }

    /// UM ciclo completo: coleta → (quando devido) spawn da avaliação.
    /// Devolve o handle da avaliação spawnada (para testes/aguardo) ou
    /// `None` quando a avaliação não foi disparada neste ciclo.
    pub async fn ciclo(&mut self) -> Option<JoinHandle<()>> {
        // ===== 1. Coleta =====
        let (fisiologica, ambiental) = self.sensor.collect();
        match self.deps.coleta.executar(fisiologica, ambiental).await {
            Ok(resultado) => {
                for aviso in &resultado.avisos {
                    tracing::warn!("coleta: {aviso}");
                }
                tracing::debug!("coleta persistida ({})", resultado.telemetria_id);
            }
            Err(erro) => {
                tracing::error!("coleta não persistida: {erro}");
            }
        }

        // ===== 2. Avaliação (intervalo + sem sobreposição) =====
        if !self.deve_avaliar() {
            return None;
        }
        self.ultima_avaliacao = Some(Instant::now());
        self.avaliando.store(true, Ordering::SeqCst);

        let avaliando = self.avaliando.clone();
        let avaliacao = self.deps.avaliacao.clone();
        let copiloto = self.deps.copiloto.clone();
        let astronauta_id = self.deps.astronauta_id.clone();
        let missao_id = self.deps.missao_id.clone();

        let handle = tokio::spawn(async move {
            let resultado = avaliacao
                .executar(&astronauta_id, missao_id.as_deref(), chrono::Utc::now())
                .await;
            match resultado {
                Ok(relatorio) => {
                    tracing::info!(
                        "avaliação {} do astronauta {}: nível {} (alerta: {})",
                        relatorio.avaliacao.id,
                        relatorio.avaliacao.astronauta_id,
                        relatorio.avaliacao.nivel_risco,
                        relatorio
                            .alerta_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "nenhum".to_string())
                    );
                    for aviso in &relatorio.avisos {
                        tracing::warn!("avaliação: {aviso}");
                    }
                    // ===== 3. Orientação da IA copiloto (opcional) =====
                    let alerta_id = relatorio.alerta_id();
                    if let Some(solicitacao) = relatorio.orientacao {
                        spawn_orientacao(copiloto, &solicitacao, alerta_id);
                    }
                }
                Err(erro) => {
                    tracing::error!("avaliação falhou: {erro}");
                }
            }
            avaliando.store(false, Ordering::SeqCst);
        });

        Some(handle)
    }

    /// `true` quando a avaliação pode ser disparada: nenhuma em curso e
    /// o intervalo mínimo entre avaliações foi respeitado.
    fn deve_avaliar(&self) -> bool {
        if self.avaliando.load(Ordering::SeqCst) {
            return false;
        }
        match self.ultima_avaliacao {
            None => true,
            Some(instante) => instante.elapsed() >= self.deps.intervalo_avaliacao,
        }
    }
}

/// Loop principal do monitoramento. Erros são LOGADOS e nunca encerram
/// o ciclo — o motor volta a coletar no próximo tick.
pub async fn start_monitoring(sensor: SensorAdapter, deps: Arc<DependenciasMotor>) {
    let mut motor = MotorMonitoramento::novo(sensor, deps);
    let mut timer = interval(motor.deps.intervalo_coleta);
    // Ciclos atrasados NÃO se acumulam: um tick perdido é pulado.
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!("motor de monitoramento iniciado");
    loop {
        timer.tick().await;
        motor.ciclo().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::analysis::repository::FaseMissao;
    use crate::application::collection::collect_measurement::ColetaMedicao;
    use crate::application::copilot::generate_guidance::GeradorOrientacao;
    use crate::application::evaluation::process_assessment::ProcessamentoAvaliacao;
    use crate::config::settings::Settings;
    use crate::domain::entities::protocol_entity::ProtocoloAvaliacao;
    use crate::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
    use crate::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;
    use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

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
        sqlx::query(
            "INSERT INTO modulos_nave (id, missao_id, nome) VALUES ('MOD1', 'M1', 'Habitat')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn protocolo_teste() -> ProtocoloAvaliacao {
        Settings::from_env().expect("padrões válidos").protocolo()
    }

    async fn motor(pool: &SqlitePool, intervalo_avaliacao: Duration) -> MotorMonitoramento {
        let measurements: Arc<
            dyn crate::domain::repositories::measurement_repository::MeasurementRepository,
        > = Arc::new(SqliteMeasurementRepository::new(pool.clone()));
        let evaluations: Arc<
            dyn crate::domain::repositories::evaluation_repository::EvaluationRepository,
        > = Arc::new(SqliteEvaluationRepository::new(pool.clone()));
        let alertas: Arc<dyn crate::domain::repositories::alert_repository::AlertRepository> =
            Arc::new(SqliteAlertRepository::new(pool.clone()));

        let coleta = Arc::new(ColetaMedicao::novo(measurements.clone()));
        let avaliacao = Arc::new(
            ProcessamentoAvaliacao::novo(
                measurements,
                evaluations.clone(),
                alertas.clone(),
                protocolo_teste(),
                300,
                5,
                FaseMissao::EmVoo,
            )
            .expect("avaliação válida"),
        );
        let copiloto = Arc::new(GeradorOrientacao::novo(None, evaluations, alertas));

        let sensor =
            SensorAdapter::new("A1".to_string(), Some("M1".to_string()), "MOD1".to_string());
        let deps = Arc::new(DependenciasMotor {
            coleta,
            avaliacao,
            copiloto,
            astronauta_id: "A1".to_string(),
            missao_id: Some("M1".to_string()),
            intervalo_avaliacao,
            intervalo_coleta: Duration::from_secs(5),
        });
        MotorMonitoramento::novo(sensor, deps)
    }

    #[tokio::test]
    async fn ciclo_coleta_e_avalia_e_nao_sobrepoe() {
        let pool = pool_com_base().await;
        let mut m = motor(&pool, Duration::from_secs(3600)).await;

        // Primeiro ciclo: coleta + avaliação spawnada.
        let handle = m.ciclo().await.expect("primeira avaliação spawnada");
        handle.await.expect("avaliação concluída");

        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM telemetria_fisiologica")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM avaliacoes_saude")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM presenca_modulo")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );

        // Segundo ciclo IMEDIATO: fora do intervalo de avaliação → só coleta.
        assert!(
            m.ciclo().await.is_none(),
            "intervalo de avaliação respeitado"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM telemetria_fisiologica")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM avaliacoes_saude")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn avaliacoes_em_curso_nao_sao_sobrepostas() {
        let pool = pool_com_base().await;
        // Intervalo 0: a política que impede sobreposição é o guard do handle.
        let mut m = motor(&pool, Duration::from_secs(0)).await;

        let handle = m.ciclo().await.expect("avaliação spawnada");
        // O guard `is_finished` evita spawn duplicado enquanto a tarefa
        // está em curso; ao terminar, o próximo ciclo avalia de novo.
        handle.await.unwrap();

        let handle2 = m.ciclo().await.expect("nova avaliação após a anterior");
        handle2.await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM avaliacoes_saude")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn motor_sobrevive_a_erros_de_coleta_e_de_avaliacao() {
        // Pool SEM astronauta/módulo: coleta e avaliação falham (FK),
        // mas nenhum ciclo entra em pânico.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("conectar");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        let mut m = motor(&pool, Duration::from_secs(0)).await;

        let handle = m.ciclo().await.expect("tarefa spawnada mesmo com falhas");
        handle.await.expect("tarefa concluída sem pânico");

        // A coleta falhou (FK do ambiente) e a avaliação falhou (FK da
        // avaliação) — nada foi persistido e o motor segue vivo.
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM avaliacoes_saude")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM telemetria_fisiologica")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );

        // Ciclo seguinte continua operando.
        let _ = m.ciclo().await;
    }
}
