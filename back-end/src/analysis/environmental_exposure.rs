/*
VE A LOCALIZAÇÃO PRA CALCULAR A EXPOSIÇÃO A DADOS COMO CO2, RADIACAO TEMPERATURA E PRESSAO
*/

// environmental_exposure.rs

use chrono::{DateTime, Utc};

use crate::domain::entities::measurement_entity::{
    AmbienteNave,
    PresencaModulo,
};

#[derive(Debug, Clone)]
pub struct EnvironmentalThresholds {
    pub co2_ppm: f64,
    pub temperature_c: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EnvironmentalExposureResult {
    pub co2_exposure_ppm_hours: f64,
    pub radiation_dose_msv: f64,
    pub temperature_exposure_degree_hours: f64,
    pub co2_time_above_seconds: f64,
    pub temperature_time_above_seconds: f64,
    pub analyzed_seconds: f64,
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn positive_excess(value: f64, threshold: f64) -> f64 {
    (value - threshold).max(0.0)
}

pub fn calculate_environmental_exposure(
    presence: &[PresencaModulo],
    environment: &[AmbienteNave],
    thresholds: &EnvironmentalThresholds,
    max_gap_seconds: i64,
) -> EnvironmentalExposureResult {
    let mut result = EnvironmentalExposureResult::default();

    for stay in presence {
        let Some(start) = parse_timestamp(&stay.inicio_utc) else {
            continue;
        };

        let end = match &stay.fim_utc {
            Some(value) => match parse_timestamp(value) {
                Some(timestamp) => timestamp,
                None => continue,
            },
            None => Utc::now(),
        };

        if end <= start {
            continue;
        }

        let mut readings: Vec<_> = environment
            .iter()
            .filter(|reading| reading.modulo_id == stay.modulo_id)
            .filter(|reading| reading.deleted_at.is_none())
            .filter_map(|reading| {
                let timestamp = parse_timestamp(&reading.timestamp_utc)?;

                if timestamp < start || timestamp > end {
                    return None;
                }

                Some((timestamp, reading))
            })
            .collect();

        readings.sort_by_key(|reading| reading.0);

        for pair in readings.windows(2) {
            let (time_a, a) = pair[0];
            let (time_b, b) = pair[1];

            let duration_seconds =
                (time_b - time_a).num_milliseconds() as f64 / 1000.0;

            if duration_seconds <= 0.0
                || duration_seconds > max_gap_seconds as f64
            {
                continue;
            }

            let duration_hours = duration_seconds / 3600.0;

            result.analyzed_seconds += duration_seconds;

            // CO2: integral of excess concentration above threshold.
            if let (Some(co2_a), Some(co2_b)) =
                (a.co2_ppm, b.co2_ppm)
            {
                let excess_a =
                    positive_excess(co2_a, thresholds.co2_ppm);

                let excess_b =
                    positive_excess(co2_b, thresholds.co2_ppm);

                result.co2_exposure_ppm_hours +=
                    ((excess_a + excess_b) / 2.0) * duration_hours;

                if co2_a > thresholds.co2_ppm
                    && co2_b > thresholds.co2_ppm
                {
                    result.co2_time_above_seconds += duration_seconds;
                }
            }

            // Radiation: integrate environmental dose rate.
            if let (Some(rate_a), Some(rate_b)) =
                (a.radiacao_msv_hora, b.radiacao_msv_hora)
            {
                if rate_a >= 0.0 && rate_b >= 0.0 {
                    result.radiation_dose_msv +=
                        ((rate_a + rate_b) / 2.0) * duration_hours;
                }
            }

            // Temperature: integral of excess temperature.
            if let (Some(temp_a), Some(temp_b)) =
                (a.temperatura_cabine_c, b.temperatura_cabine_c)
            {
                let excess_a =
                    positive_excess(temp_a, thresholds.temperature_c);

                let excess_b =
                    positive_excess(temp_b, thresholds.temperature_c);

                result.temperature_exposure_degree_hours +=
                    ((excess_a + excess_b) / 2.0) * duration_hours;

                if temp_a > thresholds.temperature_c
                    && temp_b > thresholds.temperature_c
                {
                    result.temperature_time_above_seconds +=
                        duration_seconds;
                }
            }
        }
    }

    result
}

// ============================================================
// VARIAÇÃO COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

use std::collections::HashSet;

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::LIMITE_PADRAO_SERIE;

/// Calcula a exposição ambiental buscando do repositório as presenças do
/// astronauta e as medições de cada módulo visitado.
pub async fn calculate_environmental_exposure_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    thresholds: &EnvironmentalThresholds,
    max_gap_seconds: i64,
) -> Result<EnvironmentalExposureResult, sqlx::Error> {
    let presences = repo.list_presences(astronauta_id).await?;

    let mut ambientes: Vec<AmbienteNave> = Vec::new();

    let mut modulos: HashSet<&str> = HashSet::new();
    for presenca in &presences {
        modulos.insert(presenca.modulo_id.as_str());
    }

    for modulo_id in modulos {
        let leituras = repo
            .list_environment_by_module(modulo_id, None, None, LIMITE_PADRAO_SERIE, 0)
            .await?;

        ambientes.extend(leituras);
    }

    Ok(calculate_environmental_exposure(
        &presences,
        &ambientes,
        thresholds,
        max_gap_seconds,
    ))
}