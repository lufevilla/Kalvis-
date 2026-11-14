/*
PONTE ENTRE AS ANÁLISES E O REPOSITÓRIO DE DADOS.
ÚNICO DONO DOS NOMES DE INDICADORES/FASES E DE TODAS AS COLETAS.
*/

use chrono::{DateTime, Utc};

use crate::domain::repositories::measurement_repository::MeasurementRepository;

/// Limite por coleta. Ao ser ultrapassado, a coleta FALHA (nunca trunca em silêncio).
pub const LIMITE_PADRAO_SERIE: i64 = 100_000;

// ============================================================
// INDICADORES E FASES (nomes como gravados no banco)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Indicador {
    FrequenciaCardiaca,
    Spo2,
    TemperaturaCorporal,
    TemperaturaPele,
    TaxaRespiratoria,
    Hrv,
    PressaoSistolica,
    PressaoDiastolica,
    Glicose,
    HorasSono,
}

impl Indicador {
    pub const TODOS: [Indicador; 10] = [
        Self::FrequenciaCardiaca, Self::Spo2, Self::TemperaturaCorporal,
        Self::TemperaturaPele, Self::TaxaRespiratoria, Self::Hrv,
        Self::PressaoSistolica, Self::PressaoDiastolica, Self::Glicose,
        Self::HorasSono,
    ];

    pub fn nome(self) -> &'static str {
        match self {
            Self::FrequenciaCardiaca => "FREQUENCIA_CARDIACA",
            Self::Spo2 => "SPO2",
            Self::TemperaturaCorporal => "TEMPERATURA_CORPORAL",
            Self::TemperaturaPele => "TEMPERATURA_PELE",
            Self::TaxaRespiratoria => "TAXA_RESPIRATORIA",
            Self::Hrv => "HRV",
            Self::PressaoSistolica => "PRESSAO_SISTOLICA",
            Self::PressaoDiastolica => "PRESSAO_DIASTOLICA",
            Self::Glicose => "GLICOSE",
            Self::HorasSono => "HORAS_SONO",
        }
    }

    pub fn from_nome(nome: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|i| i.nome() == nome)
    }

    /// Coluna em `telemetria_fisiologica`. `None` = vem de outra tabela.
    fn coluna_telemetria(self) -> Option<&'static str> {
        match self {
            Self::FrequenciaCardiaca => Some("frequencia_cardiaca_bpm"),
            Self::Spo2 => Some("spo2_percentual"),
            Self::TemperaturaCorporal => Some("temperatura_corporal_c"),
            Self::TemperaturaPele => Some("temperatura_pele_c"),
            Self::TaxaRespiratoria => Some("taxa_respiratoria_rpm"),
            Self::Hrv => Some("hrv_ms"),
            Self::PressaoSistolica => Some("pressao_sistolica_mmhg"),
            Self::PressaoDiastolica => Some("pressao_diastolica_mmhg"),
            Self::Glicose => Some("glicose_mg_dl"),
            Self::HorasSono => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaseMissao {
    PreVoo,
}

impl FaseMissao {
    pub fn nome(self) -> &'static str {
        match self {
            Self::PreVoo => "PRE_VOO",
        }
    }
}

// ============================================================
// COLETAS
// ============================================================

/// Amostra bruta, sem nenhum filtro: o `data_quality` decide o que é válido.
#[derive(Debug, Clone)]
pub struct AmostraBruta {
    pub momento_utc: String,
    pub valor: Option<f64>,
}

fn verificar_limite(qtd: usize) -> Result<(), sqlx::Error> {
    if qtd as i64 > LIMITE_PADRAO_SERIE {
        return Err(sqlx::Error::Protocol(format!(
            "série excede o limite de {LIMITE_PADRAO_SERIE} amostras"
        )));
    }
    Ok(())
}

/// Série bruta de qualquer indicador (telemetria ou registro diário).
pub async fn buscar_serie_bruta(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: Indicador,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Vec<AmostraBruta>, sqlx::Error> {
    // Busca limite + 1 para detectar truncamento.
    let limite = LIMITE_PADRAO_SERIE + 1;

    let serie: Vec<AmostraBruta> = match indicador.coluna_telemetria() {
        Some(coluna) => repo
            .list_indicator_time_series(
                astronauta_id, missao_id, coluna, inicio_utc, fim_utc, limite, 0,
            )
            .await?
            .into_iter()
            .map(|p| AmostraBruta { momento_utc: p.momento_utc, valor: p.valor })
            .collect(),

        // Registro diário não tem missão associada.
        None => repo
            .list_daily_logs(astronauta_id, inicio_utc, fim_utc, limite, 0)
            .await?
            .into_iter()
            .filter(|r| r.deleted_at.is_none())
            .map(|r| AmostraBruta { momento_utc: r.data_registro, valor: r.horas_sono_total })
            .collect(),
    };

    verificar_limite(serie.len())?;
    Ok(serie)
}

/// Converte um timestamp ISO 8601 (ou uma data `YYYY-MM-DD`, à meia-noite UTC).
pub fn parse_momento(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc())
}
