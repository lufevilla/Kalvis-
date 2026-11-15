/*
MOTOR DE MONITORAMENTO (COLETA + AVALIAÇÃO + ORIENTAÇÃO)

A cada `intervalo_coleta`:
 1. SensorAdapter coleta, valida (data quality) e persiste;
 2. se o intervalo permitir e nenhuma avaliação estiver em curso,
    a avaliação roda em background (a coleta nunca espera por ela);
 3. ao fim da avaliação, a orientação da IA é disparada (sem sobreposição).

Proteções contra travamento/sobrecarga:
 - timeout na coleta e na avaliação: uma tarefa travada não segura recursos;
 - no máximo UMA avaliação por vez (compare_exchange + guard RAII);
 - ticks atrasados são pulados (sem rajada de ciclos acumulados).
*/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::application::copilot::generate_guidance::{GeradorOrientacao, spawn_orientacao};
use crate::application::evaluation::process_assessment::ProcessamentoAvaliacao;
use crate::domain::repositories::measurement_repository::MeasurementRepository;
use crate::infrastructure::sensors::sensor_adapter::SensorAdapter;

/// Teto de duração de uma avaliação antes de ser abortada.
const TIMEOUT_AVALIACAO: Duration = Duration::from_secs(30);

pub struct DependenciasMotor {
    pub measurements: Arc<dyn MeasurementRepository>,
    pub avaliacao: Arc<ProcessamentoAvaliacao>,
    pub copiloto: Arc<GeradorOrientacao>,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub intervalo_avaliacao: Duration,
    pub intervalo_coleta: Duration,
}

/// Libera a flag ao sair do escopo, mesmo em caso de pânico ou timeout.
struct GuardAvaliacao(Arc<AtomicBool>);

impl Drop for GuardAvaliacao {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub struct MotorMonitoramento {
    sensor: SensorAdapter,
    deps: Arc<DependenciasMotor>,
    ultima_avaliacao: Option<Instant>,
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

    async fn ciclo(&mut self) {
        // ===== 1. Coleta (validação e insert ficam no SensorAdapter) =====
        // O timeout é igual ao intervalo: uma coleta travada não invade o próximo ciclo.
        let coleta = timeout(
            self.deps.intervalo_coleta,
            self.sensor.coletar_e_salvar(self.deps.measurements.as_ref()),
        )
        .await;

        match coleta {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => tracing::warn!("coleta descartada: data quality reprovado"),
            Ok(Err(e)) => tracing::error!("coleta não persistida: {e}"),
            Err(_) => tracing::error!("coleta excedeu o timeout"),
        }

        // TODO: chamar o histórico de monitoramento.

        // ===== 2. Avaliação =====
        self.disparar_avaliacao();
    }

    fn disparar_avaliacao(&mut self) {
        let no_intervalo = self
            .ultima_avaliacao
            .is_none_or(|t| t.elapsed() >= self.deps.intervalo_avaliacao);
        if !no_intervalo {
            return;
        }
        // Reserva atômica: falha se já houver uma avaliação em curso.
        if self
            .avaliando
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        self.ultima_avaliacao = Some(Instant::now());

        let guard = GuardAvaliacao(self.avaliando.clone());
        let deps = self.deps.clone();

        tokio::spawn(async move {
            let _guard = guard;

            let resultado = timeout(
                TIMEOUT_AVALIACAO,
                deps.avaliacao
                    .executar(&deps.astronauta_id, deps.missao_id.as_deref(), chrono::Utc::now()),
            )
            .await;

            let relatorio = match resultado {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => return tracing::error!("avaliação falhou: {e}"),
                Err(_) => return tracing::error!("avaliação excedeu o timeout"),
            };

            let alerta_id = relatorio.alerta_id();
            tracing::info!(
                "avaliação {}: nível {} (alerta: {:?})",
                relatorio.avaliacao.id,
                relatorio.avaliacao.nivel_risco,
                alerta_id
            );
            for aviso in &relatorio.avisos {
                tracing::warn!("avaliação: {aviso}");
            }

            // ===== 3. Orientação da IA (não sobreposta, trava interna) =====
            if let Some(solicitacao) = relatorio.orientacao {
                spawn_orientacao(deps.copiloto.clone(), &solicitacao, alerta_id);
            }
        });
    }
}

/// Loop principal. Nenhum erro encerra o motor.
pub async fn start_monitoring(sensor: SensorAdapter, deps: Arc<DependenciasMotor>) {
    let mut timer = interval(deps.intervalo_coleta);
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut motor = MotorMonitoramento::novo(sensor, deps);

    tracing::info!("motor de monitoramento iniciado");
    loop {
        timer.tick().await;
        motor.ciclo().await;
    }
}
