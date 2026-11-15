//! Publica cada `ResultadoAnalise` em um canal de transmissão e guarda o histórico recente.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{Value, json};
use tokio::sync::broadcast;

use crate::analysis::engine::{Episodio, IndicadorAvaliado, Motivo, ResultadoAnalise};
use crate::analysis::variation::{Direcao, NivelVariacao};

const CAPACIDADE: usize = 512;

#[derive(Debug, Clone)]
pub struct EventoAlerta {
    pub seq: i64,
    pub astronauta_id: String,
    pub json: String,
}

pub struct AlertHub {
    tx: broadcast::Sender<Arc<EventoAlerta>>,
    historico: Mutex<VecDeque<Arc<EventoAlerta>>>,
}

static HUB: OnceLock<AlertHub> = OnceLock::new();

pub fn hub() -> &'static AlertHub {
    HUB.get_or_init(|| {
        let (tx, _) = broadcast::channel(CAPACIDADE);
        AlertHub { tx, historico: Mutex::new(VecDeque::with_capacity(CAPACIDADE)) }
    })
}

impl AlertHub {
    pub fn publicar(&self, r: &ResultadoAnalise) {
        let seq = r.momento.timestamp_millis();
        let evento = Arc::new(EventoAlerta {
            seq,
            astronauta_id: r.astronauta_id.clone(),
            json: para_json(r, seq).to_string(),
        });
        {
            let mut h = self.historico.lock().unwrap_or_else(|e| e.into_inner());
            if h.len() == CAPACIDADE {
                h.pop_front();
            }
            h.push_back(evento.clone());
        }
        let _ = self.tx.send(evento); // sem assinantes não é erro
    }

    pub fn assinar(&self) -> broadcast::Receiver<Arc<EventoAlerta>> {
        self.tx.subscribe()
    }

    pub fn desde(&self, seq: i64, astronauta: Option<&str>) -> Vec<Arc<EventoAlerta>> {
        self.historico
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|e| e.seq > seq && astronauta.is_none_or(|a| a == e.astronauta_id))
            .cloned()
            .collect()
    }
}

fn nivel(n: NivelVariacao) -> &'static str {
    match n {
        NivelVariacao::Indeterminado => "indeterminado",
        NivelVariacao::Normal => "normal",
        NivelVariacao::Atencao => "atencao",
        NivelVariacao::Critico => "critico",
    }
}

fn direcao(d: Direcao) -> String {
    format!("{d:?}").to_lowercase()
}

fn episodio(e: Episodio) -> &'static str {
    match e {
        Episodio::Exercicio => "exercicio",
        Episodio::Ataque => "ataque",
        Episodio::Indefinido => "indefinido",
    }
}

fn motivo(m: &Motivo) -> Value {
    let (codigo, ind) = match m {
        Motivo::DadoNaoConfiavel(i) => ("dado_nao_confiavel", Some(i.nome())),
        Motivo::SemBaseline(i) => ("sem_baseline", Some(i.nome())),
        Motivo::ExercicioEsperado => ("exercicio_esperado", None),
        Motivo::EpisodioAnsiedade => ("episodio_ansiedade", None),
        Motivo::AdaptacaoFluidosVoo => ("adaptacao_fluidos_voo", None),
        Motivo::RiscoQuedaOrtostatica => ("risco_queda_ortostatica", None),
        Motivo::Co2Modulo => ("co2_modulo", None),
        Motivo::CabineQuente => ("cabine_quente", None),
        Motivo::PossivelInfeccao => ("possivel_infeccao", None),
        Motivo::DeficitSono => ("deficit_sono", None),
        Motivo::ExercicioInsuficiente => ("exercicio_insuficiente", None),
        Motivo::AlteracaoPersistente(i) => ("alteracao_persistente", Some(i.nome())),
        Motivo::PioraProgressiva(i) => ("piora_progressiva", Some(i.nome())),
    };
    json!({ "codigo": codigo, "indicador": ind })
}

fn indicador(i: &IndicadorAvaliado) -> Value {
    json!({
        "indicador": i.variacao.indicador.nome(),
        "nivel_original": nivel(i.variacao.nivel),
        "nivel_final": nivel(i.nivel_final),
        "direcao": direcao(i.variacao.direcao),
        "z_score": i.variacao.z_score,
        "persistente": i.persistencia.as_ref().map(|p| p.is_persistent),
        "tendencia": i.tendencia.as_ref().map(|t| json!({
            "direcao": format!("{:?}", t.direction).to_lowercase(),
            "inclinacao_h": t.slope_per_hour,
        })),
    })
}

fn para_json(r: &ResultadoAnalise, seq: i64) -> Value {
    json!({
        "tipo": "analise.identificada",
        "versao": 1,
        "id": format!("{}-{}-{}", r.astronauta_id, r.missao_id, seq),
        "seq": seq,
        "astronauta_id": r.astronauta_id,
        "missao_id": r.missao_id,
        "momento": r.momento.to_rfc3339(),
        "fase": r.fase.nome(),
        "dias_na_fase": r.dias_na_fase,
        "episodio": episodio(r.episodio),
        "nivel_final": nivel(r.nivel_final),
        "indicadores": r.indicadores.iter().map(indicador).collect::<Vec<_>>(),
        "motivos": r.motivos.iter().map(motivo).collect::<Vec<_>>(),
        "exposicao_ambiental": r.exposicao_ambiental.as_ref().map(|e| json!({
            "co2_acima_s": e.co2_time_above_seconds,
            "temperatura_acima_s": e.temperature_time_above_seconds,
        })),
    })
}
