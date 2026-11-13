/*
PONTE ENTRE AS ANALISES E O REPOSITORIO DE DADOS

AS FUNCOES DE ANALISE RECEBEM UM `MeasurementRepository` E BUSCAM A SI
PROPRIAS OS DADOS DE QUE PRECISAM (SERIES TEMPORAIS, PRESENCAS, AMBIENTE).
A LOGICA DE CALCULO CONTINUA NAS FUNCOES PURAS DE CADA MODULO.
*/

use chrono::{DateTime, Utc};

use crate::domain::repositories::measurement_repository::{MeasurementRepository, PontoSerie};

/// Limite padrão de amostras buscadas por série temporal. As análises
/// operam sobre janelas completas, não paginadas.
pub const LIMITE_PADRAO_SERIE: i64 = 100_000;

/// Traduz o nome do indicador (como gravado em `baseline_fisiologico.indicador`)
/// para o nome da coluna correspondente em `telemetria_fisiologica`, que é o
/// que as queries de série temporal aceitam.
pub fn coluna_do_indicador(indicador: &str) -> Option<&'static str> {
    match indicador {
        "FREQUENCIA_CARDIACA" => Some("frequencia_cardiaca_bpm"),
        "SPO2" => Some("spo2_percentual"),
        "TEMPERATURA_CORPORAL" => Some("temperatura_corporal_c"),
        "TEMPERATURA_PELE" => Some("temperatura_pele_c"),
        "TAXA_RESPIRATORIA" => Some("taxa_respiratoria_rpm"),
        "HRV" => Some("hrv_ms"),
        "PRESSAO_SISTOLICA" => Some("pressao_sistolica_mmhg"),
        "PRESSAO_DIASTOLICA" => Some("pressao_diastolica_mmhg"),
        "GLICOSE" => Some("glicose_mg_dl"),
        _ => None,
    }
}

/// Busca a série temporal `(momento_utc, valor)` de um indicador fisiológico
/// a partir do nome oficial do indicador. Indicador desconhecido (ou sem
/// série, como `HORAS_SONO`) devolve uma série vazia.
pub async fn buscar_serie_do_indicador(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Vec<PontoSerie>, sqlx::Error> {
    let Some(coluna) = coluna_do_indicador(indicador) else {
        return Ok(Vec::new());
    };

    repo.list_indicator_time_series(
        astronauta_id,
        missao_id,
        coluna,
        inicio_utc,
        fim_utc,
        LIMITE_PADRAO_SERIE,
        0,
    )
    .await
}

/// Busca apenas os valores (na ordem temporal) de um indicador fisiológico.
pub async fn buscar_valores_do_indicador(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Vec<f64>, sqlx::Error> {
    let serie =
        buscar_serie_do_indicador(repo, astronauta_id, missao_id, indicador, inicio_utc, fim_utc)
            .await?;

    Ok(serie.into_iter().map(|ponto| ponto.valor).collect())
}

/// Converte um timestamp ISO 8601 em `DateTime<Utc>`.
pub fn parse_momento(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}
