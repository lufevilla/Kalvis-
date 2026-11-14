use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::fs;
use std::path::Path;
use std::str::FromStr;

pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    // Cria a pasta se ela não existir (ex: "data")

    if let Some(parent) = Path::new(db_path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
    }

    // Cria o arquivo .db automaticamente se não existir + ativa FKs e WAL
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Roda as migrations da pasta back-end/migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
