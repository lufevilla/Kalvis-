use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

use crate::domain::entities::measurement_entity::{AmbienteNave, TelemetriaFisiologica};

pub struct SensorAdapter {
    astronauta_id: String,
    missao_id: Option<String>,
    modulo_id: String,
}

impl SensorAdapter {
    pub fn new(astronauta_id: String, missao_id: Option<String>, modulo_id: String) -> Self {
        Self {
            astronauta_id,
            missao_id,
            modulo_id,
        }
    }

    /// Simula uma coleta dos sensores fisiológicos e ambientais.
    /// Retorna os dados utilizando as entidades já existentes no domínio.
    pub fn collect(&self) -> (TelemetriaFisiologica, AmbienteNave) {
        let mut rng = rand::thread_rng();

        let timestamp = Utc::now().to_rfc3339();
        let updated_at = Some(timestamp.clone());

        let fisiologica = TelemetriaFisiologica {
            id: Uuid::new_v4().to_string(),
            astronauta_id: self.astronauta_id.clone(),
            missao_id: self.missao_id.clone(),
            timestamp_utc: timestamp.clone(),

            frequencia_cardiaca_bpm: Some(rng.gen_range(55.0..110.0)),

            spo2_percentual: Some(rng.gen_range(94.0..100.0)),

            temperatura_corporal_c: Some(rng.gen_range(36.0..38.0)),

            temperatura_pele_c: Some(rng.gen_range(32.0..36.0)),

            taxa_respiratoria_rpm: Some(rng.gen_range(12.0..24.0)),

            hrv_ms: Some(rng.gen_range(25.0..100.0)),

            pressao_sistolica_mmhg: Some(rng.gen_range(100.0..140.0)),

            pressao_diastolica_mmhg: Some(rng.gen_range(60.0..90.0)),

            glicose_mg_dl: Some(rng.gen_range(70.0..130.0)),

            origem_medicao: Some("SIMULADO".to_string()),
            updated_at: updated_at.clone(),
            deleted_at: None,
        };

        let ambiental = AmbienteNave {
            id: Uuid::new_v4().to_string(),
            modulo_id: self.modulo_id.clone(),
            timestamp_utc: timestamp,

            co2_ppm: Some(rng.gen_range(400.0..2000.0)),

            // Valor sintético de taxa de dose ambiental.
            // Não representa a saída real do CANS.
            radiacao_msv_hora: Some(rng.gen_range(0.0001..0.01)),

            temperatura_cabine_c: Some(rng.gen_range(19.0..29.0)),

            pressao_kpa: Some(rng.gen_range(95.0..105.0)),

            updated_at,
            deleted_at: None,
        };

        (fisiologica, ambiental)
    }
}
