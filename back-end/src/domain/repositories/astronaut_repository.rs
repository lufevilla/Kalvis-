use async_trait::async_trait;

use crate::domain::entities::astronaut_entity::Astronaut;

#[async_trait]
pub trait AstronautRepository: Send + Sync {
    /// Query 1.1 — busca um astronauta pelo `id`, ignorando registros removidos logicamente.
    async fn find_by_id(&self, id: &str) -> Result<Option<Astronaut>, sqlx::Error>;

    /// Query 1.2 — lista astronautas com paginação (`limite` / `deslocamento`).
    async fn list(&self, limite: i64, deslocamento: i64) -> Result<Vec<Astronaut>, sqlx::Error>;
}
