use chrono::{SecondsFormat, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::analysis::data_quality::{DataQualityConfig, evaluate_data_quality};
use crate::analysis::repository::{AmostraBruta, Indicador};
use crate::domain::entities::measurement_entity::{AmbienteNave, TelemetriaFisiologica};
use crate::domain::repositories::measurement_repository::MeasurementRepository;

/// Número máximo de coletas antes de desistir.
const MAX_TENTATIVAS: usize = 3;

const FISIOLOGICOS: [Indicador; 9] = [
    Indicador::FrequenciaCardiaca,
    Indicador::Spo2,
    Indicador::TemperaturaCorporal,
    Indicador::TemperaturaPele,
    Indicador::TaxaRespiratoria,
    Indicador::Hrv,
    Indicador::PressaoSistolica,
    Indicador::PressaoDiastolica,
    Indicador::Glicose,
];

/// (valor inicial, mínimo, máximo, passo máximo por coleta).
fn perfil(i: Indicador) -> (f64, f64, f64, f64) {
    match i {
        Indicador::FrequenciaCardiaca => (75.0, 55.0, 110.0, 3.0),
        Indicador::Spo2 => (98.0, 94.0, 100.0, 0.5),
        Indicador::TemperaturaCorporal => (36.8, 36.0, 38.0, 0.05),
        Indicador::TemperaturaPele => (34.0, 32.0, 36.0, 0.1),
        Indicador::TaxaRespiratoria => (16.0, 12.0, 24.0, 1.0),
        Indicador::Hrv => (60.0, 25.0, 100.0, 5.0),
        Indicador::PressaoSistolica => (120.0, 100.0, 140.0, 3.0),
        Indicador::PressaoDiastolica => (80.0, 60.0, 90.0, 2.0),
        Indicador::Glicose => (100.0, 70.0, 130.0, 2.0),
        Indicador::HorasSono => unreachable!("registro diário, não vem do sensor contínuo"),
    }
}

fn valor(t: &TelemetriaFisiologica, i: Indicador) -> Option<f64> {
    match i {
        Indicador::FrequenciaCardiaca => t.frequencia_cardiaca_bpm,
        Indicador::Spo2 => t.spo2_percentual,
        Indicador::TemperaturaCorporal => t.temperatura_corporal_c,
        Indicador::TemperaturaPele => t.temperatura_pele_c,
        Indicador::TaxaRespiratoria => t.taxa_respiratoria_rpm,
        Indicador::Hrv => t.hrv_ms,
        Indicador::PressaoSistolica => t.pressao_sistolica_mmhg,
        Indicador::PressaoDiastolica => t.pressao_diastolica_mmhg,
        Indicador::Glicose => t.glicose_mg_dl,
        Indicador::HorasSono => None,
    }
}

fn set_valor(t: &mut TelemetriaFisiologica, i: Indicador, v: Option<f64>) {
    let campo = match i {
        Indicador::FrequenciaCardiaca => &mut t.frequencia_cardiaca_bpm,
        Indicador::Spo2 => &mut t.spo2_percentual,
        Indicador::TemperaturaCorporal => &mut t.temperatura_corporal_c,
        Indicador::TemperaturaPele => &mut t.temperatura_pele_c,
        Indicador::TaxaRespiratoria => &mut t.taxa_respiratoria_rpm,
        Indicador::Hrv => &mut t.hrv_ms,
        Indicador::PressaoSistolica => &mut t.pressao_sistolica_mmhg,
        Indicador::PressaoDiastolica => &mut t.pressao_diastolica_mmhg,
        Indicador::Glicose => &mut t.glicose_mg_dl,
        Indicador::HorasSono => return,
    };
    *campo = v;
}

/// Passa a leitura pelo data quality. `true` = todos os indicadores aprovados.
fn passou_data_quality(t: &TelemetriaFisiologica) -> bool {
    FISIOLOGICOS.iter().all(|&ind| {
        let mut cfg = DataQualityConfig::padrao_para(ind);
        cfg.minimum_samples = 1;
        cfg.max_staleness_seconds = None;

        let amostra = AmostraBruta {
            momento_utc: t.timestamp_utc.clone(),
            valor: valor(t, ind),
        };
        evaluate_data_quality(&[amostra], None, &cfg).acceptable_quality
    })
}

pub struct SensorAdapter {
    astronauta_id: String,
    missao_id: Option<String>,
    modulo_id: String,
    estado: [f64; 9],
    probabilidade_falha: f64,
}

impl SensorAdapter {
    pub fn new(astronauta_id: String, missao_id: Option<String>, modulo_id: String) -> Self {
        Self {
            astronauta_id,
            missao_id,
            modulo_id,
            estado: FISIOLOGICOS.map(|i| perfil(i).0),
            probabilidade_falha: 0.02,
        }
    }

    /// Sensores → data quality → insert se validado; senão, recoleta.
    /// Retorna `true` se gravou, `false` se esgotou as tentativas.
    pub async fn coletar_e_salvar(
        &mut self,
        repo: &dyn MeasurementRepository,
    ) -> Result<bool, sqlx::Error> {
        for _ in 0..MAX_TENTATIVAS {
            let (fisiologica, ambiental) = self.collect();

            if passou_data_quality(&fisiologica) {
                repo.save_telemetria(&fisiologica).await?;
                repo.save_ambiente(&ambiental).await?;
                return Ok(true);
            }
        }
        // TODO: registrar evento "VERIFICAR SENSOR".
        Ok(false)
    }

    /// Leitura bruta simulada dos sensores.
    fn collect(&mut self) -> (TelemetriaFisiologica, AmbienteNave) {
        let mut rng = rand::thread_rng();
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let updated_at = Some(timestamp.clone());

        let mut fisiologica = TelemetriaFisiologica {
            id: Uuid::new_v4().to_string(),
            astronauta_id: self.astronauta_id.clone(),
            missao_id: self.missao_id.clone(),
            timestamp_utc: timestamp.clone(),
            frequencia_cardiaca_bpm: None,
            spo2_percentual: None,
            temperatura_corporal_c: None,
            temperatura_pele_c: None,
            taxa_respiratoria_rpm: None,
            hrv_ms: None,
            pressao_sistolica_mmhg: None,
            pressao_diastolica_mmhg: None,
            glicose_mg_dl: None,
            origem_medicao: Some("SIMULADO".to_string()),
            updated_at: updated_at.clone(),
            deleted_at: None,
        };

        for (k, ind) in FISIOLOGICOS.iter().enumerate() {
            let (_, min, max, passo) = perfil(*ind);
            self.estado[k] = (self.estado[k] + rng.gen_range(-passo..=passo)).clamp(min, max);

            let lido = if rng.gen_bool(self.probabilidade_falha) {
                if rng.gen_bool(0.5) { None } else { Some(9_999.0) }
            } else {
                Some(self.estado[k])
            };
            set_valor(&mut fisiologica, *ind, lido);
        }

        let ambiental = AmbienteNave {
            id: Uuid::new_v4().to_string(),
            modulo_id: self.modulo_id.clone(),
            timestamp_utc: timestamp,
            co2_ppm: Some(rng.gen_range(400.0..2000.0)),
            radiacao_msv_hora: Some(rng.gen_range(0.0001..0.01)),
            temperatura_cabine_c: Some(rng.gen_range(19.0..29.0)),
            pressao_kpa: Some(rng.gen_range(95.0..105.0)),
            updated_at,
            deleted_at: None,
        };

        (fisiologica, ambiental)
    }
}
