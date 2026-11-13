use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use crate::infrastructure::sensors::sensor_adapter::SensorAdapter;
use crate::domain::entities::measurement_entity::{AmbienteNave, TelemetriaFisiologica};

pub async fn start_monitoring(sensor: SensorAdapter) {
    let mut timer = interval(Duration::from_secs(5));

    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        timer.tick().await;

        // Solicita uma nova coleta ao SensorAdapter.
        let (fisiologica, ambiental) = sensor.collect();

        println!("Nova coleta realizada!");
        println!(
            "Frequência cardíaca: {:?} BPM",
            fisiologica.frequencia_cardiaca_bpm
        );

        // Encaminha as medições para o processamento.
        processar_alteracoes(fisiologica, ambiental).await;
    }
}

async fn processar_alteracoes(
    fisiologica: TelemetriaFisiologica,
    ambiental: AmbienteNave,
) {
    // Aqui entrará a chamada para o motor de detecção de alterações.

    println!("Medições encaminhadas para análise.");
}