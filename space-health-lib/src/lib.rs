mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs */

/// Space Health Library
/// 
/// Shared Rust library for Flutter-Rust bridge integration.
/// Provides tracking data access via FFI.

pub mod models;
pub mod database;
pub mod tracking;

// Re-export public API
pub use models::{TrackingData, TrackingVital, TrackingEnvironment};
pub use tracking::{init_database, get_tracking_data, get_crew_vitals, get_environment_metrics};

/// Initialize the library (called automatically by flutter_rust_bridge)
pub fn init() {
    // Initialize tracing for logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init();
}