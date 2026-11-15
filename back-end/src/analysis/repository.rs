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
        Self::FrequenciaCardiaca,
        Self::Spo2,
        Self::TemperaturaCorporal,
        Self::TemperaturaPele,
        Self::TaxaRespiratoria,
        Self::Hrv,
        Self::PressaoSistolica,
        Self::PressaoDiastolica,
        Self::Glicose,
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
    EmVoo,
    PosVoo,
}

impl FaseMissao {
    /// Nome como gravado no banco (`baseline_fisiologico.fase_missao`,
    /// CHECK em ('PRE_VOO', 'EM_VOO', 'POS_VOO')).
    pub fn nome(self) -> &'static str {
        match self {
            Self::PreVoo => "PRE_VOO",
            Self::EmVoo => "EM_VOO",
            Self::PosVoo => "POS_VOO",
        }
    }

    /// Interpreta o valor gravado no banco. Nome desconhecido → `None`.
    pub fn from_nome(nome: &str) -> Option<Self> {
        match nome {
            "PRE_VOO" => Some(Self::PreVoo),
            "EM_VOO" => Some(Self::EmVoo),
            "POS_VOO" => Some(Self::PosVoo),
            _ => None,
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

pub(crate) fn verificar_limite(qtd: usize) -> Result<(), sqlx::Error> {
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
                astronauta_id,
                missao_id,
                coluna,
                inicio_utc,
                fim_utc,
                limite,
                0,
            )
            .await?
            .into_iter()
            .map(|p| AmostraBruta {
                momento_utc: p.momento_utc,
                valor: Some(p.valor),
            })
            .collect(),

        // Registro diário: `data_registro` é YYYY-MM-DD. Trunca a janela para
        // data, senão "2026-11-15" < "2026-11-15T00:00:00.000Z" exclui o 1º dia.
        None => {
            let dia = |s: &str| s.get(..10).unwrap_or(s).to_string();
            let inicio = inicio_utc.map(dia);
            let fim = fim_utc.map(dia);
            repo.list_daily_logs(astronauta_id, inicio.as_deref(), fim.as_deref(), limite, 0)
                .await?
                .into_iter()
                .filter(|r| r.deleted_at.is_none())
                .map(|r| AmostraBruta {
                    momento_utc: r.data_registro,
                    valor: r.horas_sono_total,
                })
                .collect()
        }

    };

    verificar_limite(serie.len())?;
    Ok(serie)
}

/// Série temporal de um indicador identificado pelo NOME gravado no banco.
/// Nome desconhecido devolve série vazia (os módulos de análise tratam
/// série vazia como "não analisável", nunca como erro).
pub async fn buscar_serie_do_indicador(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Vec<AmostraBruta>, sqlx::Error> {
    match Indicador::from_nome(indicador) {
        Some(ind) => {
            buscar_serie_bruta(repo, astronauta_id, missao_id, ind, inicio_utc, fim_utc).await
        }
        None => Ok(Vec::new()),
    }
}

/// Pares `(momento, valor)` de um indicador, já convertidos e ordenados
/// cronologicamente. Amostras sem valor, com timestamp inválido ou com
/// valor não finito são descartadas (a validação fina continua sendo
/// responsabilidade do `data_quality`). Nome desconhecido → vazio.
pub async fn buscar_valores_do_indicador(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
) -> Result<Vec<(DateTime<Utc>, f64)>, sqlx::Error> {
    let serie = buscar_serie_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    let mut pares: Vec<(DateTime<Utc>, f64)> = serie
        .iter()
        .filter_map(|amostra| {
            let momento = parse_momento(&amostra.momento_utc)?;
            let valor = amostra.valor?;
            valor.is_finite().then_some((momento, valor))
        })
        .collect();

    pares.sort_by_key(|&(momento, _)| momento);
    Ok(pares)
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
