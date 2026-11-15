use serde::{Deserialize, Serialize};
use crate::domain::repositories::measurement_repository::{PainelAtual, ResumoAmbientalModulo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingVitalDto {
    pub title: String,
    pub value: String,
    pub unit: String,
    pub status: String,
    pub status_label: String,
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub normal_min: Option<f64>,
    pub normal_max: Option<f64>,
    pub description: Option<String>,
    pub action_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingEnvironmentDto {
    pub title: String,
    pub value: String,
    pub unit: String,
    pub status: String,
    pub status_label: String,
    pub description: Option<String>,
    pub action_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingDataResponse {
    pub crew_vitals: Vec<TrackingVitalDto>,
    pub environment_metrics: Vec<TrackingEnvironmentDto>,
    pub last_update: String,
    pub astronaut_id: String,
    pub mission_name: Option<String>,
    pub current_module: Option<String>,
}

impl TrackingDataResponse {
    pub fn from_panel_and_env(
        panel: PainelAtual,
        env_summaries: Vec<ResumoAmbientalModulo>,
        astronaut_id: String,
    ) -> Self {
        let mission_name = panel.nome_missao.clone();
        let current_module = panel.nome_modulo_atual.clone();
        let last_update = panel.ultima_telemetria_utc.clone();

        // Crew vitals from panel
        let mut crew_vitals = vec![];

        // Heart Rate
        if let Some(hr) = panel.frequencia_cardiaca_bpm {
            crew_vitals.push(TrackingVitalDto {
                title: "Frequência Cardíaca".to_string(),
                value: hr.round().to_string(),
                unit: "bpm".to_string(),
                status: if hr >= 60.0 && hr <= 100.0 { "normal" } else { "attention" }.to_string(),
                status_label: if hr >= 60.0 && hr <= 100.0 { "Normal" } else { "Atenção" }.to_string(),
                range_min: Some(40.0),
                range_max: Some(140.0),
                normal_min: Some(60.0),
                normal_max: Some(100.0),
                description: None,
                action_text: None,
            });
        }

        // SpO2
        if let Some(spo2) = panel.spo2_percentual {
            crew_vitals.push(TrackingVitalDto {
                title: "Oxigenação (SpO2)".to_string(),
                value: spo2.round().to_string(),
                unit: "%".to_string(),
                status: if spo2 >= 95.0 { "normal" } else { "alert" }.to_string(),
                status_label: if spo2 >= 95.0 { "Normal" } else { "Alerta" }.to_string(),
                range_min: Some(85.0),
                range_max: Some(100.0),
                normal_min: Some(95.0),
                normal_max: Some(100.0),
                description: None,
                action_text: None,
            });
        }

        // Temperature
        if let Some(temp) = panel.temperatura_corporal_c {
            crew_vitals.push(TrackingVitalDto {
                title: "Temperatura Corporal".to_string(),
                value: format!("{:.1}", temp),
                unit: "°C".to_string(),
                status: if temp >= 36.1 && temp <= 37.2 { "normal" } else { "attention" }.to_string(),
                status_label: if temp >= 36.1 && temp <= 37.2 { "Normal" } else { "Atenção" }.to_string(),
                range_min: Some(35.0),
                range_max: Some(40.0),
                normal_min: Some(36.1),
                normal_max: Some(37.2),
                description: None,
                action_text: None,
            });
        }

        // Fatigue/Stress from daily log
        let fatigue_score = panel.fadiga_score.unwrap_or(0);
        let stress_score = panel.nivel_estresse_subjetivo.unwrap_or(0);
        let fatigue_pct = ((fatigue_score + stress_score) as f64 / 20.0 * 100.0).min(100.0).round() as i64;
        
        crew_vitals.push(TrackingVitalDto {
            title: "Nível de Fadiga / Estresse".to_string(),
            value: fatigue_pct.to_string(),
            unit: "%".to_string(),
            status: if fatigue_pct > 70 { "alert" } else if fatigue_pct > 40 { "attention" } else { "normal" }.to_string(),
            status_label: if fatigue_pct > 70 { "Alerta" } else if fatigue_pct > 40 { "Atenção" } else { "Normal" }.to_string(),
            range_min: Some(0.0),
            range_max: Some(100.0),
            normal_min: Some(0.0),
            normal_max: Some(40.0),
            description: None,
            action_text: None,
        });

        // Environment metrics from summaries
        let mut environment_metrics = vec![];

        // Use the first module's data or aggregate
        if let Some(env) = env_summaries.first() {
            // Radiation
            if let Some(rad) = env.media_radiacao_msv_hora {
                environment_metrics.push(TrackingEnvironmentDto {
                    title: "Radiação Cósmica".to_string(),
                    value: format!("{:.2}", rad),
                    unit: "mSv/h".to_string(),
                    status: if rad > 1.0 { "alert" } else { "normal" }.to_string(),
                    status_label: if rad > 1.0 { "Alerta: Alta" } else { "Normal" }.to_string(),
                    description: if rad > 1.0 { Some("acima do limiar de 1.0 mSv/h".to_string()) } else { None },
                    action_text: if rad > 1.0 { Some("Reduzir exposição externa".to_string()) } else { None },
                });
            }

            // Cabin Pressure
            if let Some(press) = env.media_pressao_kpa {
                environment_metrics.push(TrackingEnvironmentDto {
                    title: "Pressão da Cabine".to_string(),
                    value: format!("{:.1}", press),
                    unit: "kPa".to_string(),
                    status: if press >= 95.0 && press <= 105.0 { "normal" } else { "attention" }.to_string(),
                    status_label: if press >= 95.0 && press <= 105.0 { "Estável" } else { "Variação" }.to_string(),
                    description: None,
                    action_text: None,
                });
            }

            // CO2
            if let Some(co2) = env.media_co2_ppm {
                let co2_pct = co2 / 10000.0; // convert ppm to %
                environment_metrics.push(TrackingEnvironmentDto {
                    title: "Nível de CO2".to_string(),
                    value: format!("{:.2}", co2_pct),
                    unit: "%".to_string(),
                    status: if co2_pct <= 0.5 { "normal" } else { "attention" }.to_string(),
                    status_label: if co2_pct <= 0.5 { "Seguro" } else { "Elevado" }.to_string(),
                    description: None,
                    action_text: None,
                });
            }

            // O2 - estimate from CO2 or use a default
            environment_metrics.push(TrackingEnvironmentDto {
                title: "Oxigênio da Cabine (O2)".to_string(),
                value: "21.1".to_string(),
                unit: "%".to_string(),
                status: "normal".to_string(),
                status_label: "Normal".to_string(),
                description: None,
                action_text: None,
            });
        }

        Self {
            crew_vitals,
            environment_metrics,
            last_update,
            astronaut_id,
            mission_name,
            current_module,
        }
    }
}