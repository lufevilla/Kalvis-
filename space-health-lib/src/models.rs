use serde::{Deserialize, Serialize};
use flutter_rust_bridge::frb;

/// Vital signs data for crew member
#[frb(opaque)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingVital {
    pub title: String,
    pub value: String,
    pub unit: String,
    pub status: String,           // "normal" | "attention" | "alert"
    pub status_label: String,
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub normal_min: Option<f64>,
    pub normal_max: Option<f64>,
    pub description: Option<String>,
    pub action_text: Option<String>,
}

/// Environmental metrics for module
#[frb(opaque)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingEnvironment {
    pub title: String,
    pub value: String,
    pub unit: String,
    pub status: String,           // "normal" | "attention" | "alert"
    pub status_label: String,
    pub description: Option<String>,
    pub action_text: Option<String>,
}

/// Complete tracking data response
#[frb(opaque)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingData {
    pub crew_vitals: Vec<TrackingVital>,
    pub environment_metrics: Vec<TrackingEnvironment>,
    pub last_update: String,
    pub astronaut_id: String,
    pub mission_name: Option<String>,
    pub current_module: Option<String>,
}

impl TrackingVital {
    pub fn new(
        title: String,
        value: String,
        unit: String,
        status: String,
        status_label: String,
        range_min: Option<f64>,
        range_max: Option<f64>,
        normal_min: Option<f64>,
        normal_max: Option<f64>,
        description: Option<String>,
        action_text: Option<String>,
    ) -> Self {
        Self {
            title,
            value,
            unit,
            status,
            status_label,
            range_min,
            range_max,
            normal_min,
            normal_max,
            description,
            action_text,
        }
    }
}

impl TrackingEnvironment {
    pub fn new(
        title: String,
        value: String,
        unit: String,
        status: String,
        status_label: String,
        description: Option<String>,
        action_text: Option<String>,
    ) -> Self {
        Self {
            title,
            value,
            unit,
            status,
            status_label,
            description,
            action_text,
        }
    }
}

impl TrackingData {
    pub fn new(
        crew_vitals: Vec<TrackingVital>,
        environment_metrics: Vec<TrackingEnvironment>,
        last_update: String,
        astronaut_id: String,
        mission_name: Option<String>,
        current_module: Option<String>,
    ) -> Self {
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