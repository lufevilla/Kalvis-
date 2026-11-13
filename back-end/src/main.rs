use back_end::infrastructure::database::sqlite_initialization::init_db;
use tokio::time::{interval, Duration};
use back_end::infrastructure::sensors::motor::start_monitoring;
use back_end::infrastructure::sensors::sensor_adapter::SensorAdapter;

#[tokio::main]
async fn main() {
    let pool = init_db("data_base/kalvis.db")
        .await
        .expect("Falha ao inicializar o banco de dados");
    

    let sensor = SensorAdapter::new(
        "AST-001".to_string(),
        Some("MISSION-001".to_string()),
        "MODULE-001".to_string(),
    );

    start_monitoring(sensor).await;
}