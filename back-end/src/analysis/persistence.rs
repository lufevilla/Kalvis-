/*
DIFERENCIA UM PICO PONTUAL (EX: EXERCICIO) DE UMA AÇÃO QUE PODE SER MAIS GERAL
*/

#[derive(Debug, Clone)]
pub struct PersistenceResult {
    pub total_windows: usize,
    pub altered_windows: usize,
    pub persistence_percentage: f64,
    pub max_consecutive_windows: usize,
    pub is_persistent: bool,
}

pub fn evaluate_persistence(
    windows: &[bool],
    min_persistence_percentage: f64,
    min_consecutive_windows: usize,
) -> Option<PersistenceResult> {
    if windows.is_empty() {
        return None;
    }

    let total = windows.len();

    let altered = windows.iter().filter(|&&v| v).count();

    let percentage = (altered as f64 / total as f64) * 100.0;

    let mut current_streak = 0;
    let mut max_streak = 0;

    for &altered in windows {
        if altered {
            current_streak += 1;
            max_streak = max_streak.max(current_streak);
        } else {
            current_streak = 0;
        }
    }

    let is_persistent =
        percentage >= min_persistence_percentage
        && max_streak >= min_consecutive_windows;

    Some(PersistenceResult {
        total_windows: total,
        altered_windows: altered,
        persistence_percentage: percentage,
        max_consecutive_windows: max_streak,
        is_persistent,
    })
}

// ============================================================
// VARIAÇÃO COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::buscar_valores_do_indicador;

/// Avalia a persistência de um indicador fisiológico buscando a série
/// temporal direto do repositório. Cada amostra é uma janela: considera-se
/// "alterada" quando `valor >= limiar`. Série vazia devolve `None`.
pub async fn evaluate_persistence_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    limiar: f64,
    min_persistence_percentage: f64,
    min_consecutive_windows: usize,
) -> Result<Option<PersistenceResult>, sqlx::Error> {
    let valores = buscar_valores_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    if valores.is_empty() {
        return Ok(None);
    }

    let windows: Vec<bool> = valores.iter().map(|&valor| valor >= limiar).collect();

    Ok(evaluate_persistence(
        &windows,
        min_persistence_percentage,
        min_consecutive_windows,
    ))
}