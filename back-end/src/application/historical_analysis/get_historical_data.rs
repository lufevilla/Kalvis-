/*
VAI FAZER O CÁLCULO DE TUDO DE ANALYSIS

Orquestrador: único ponto de entrada dos módulos de analysis.
Fluxo (para o mais cedo possível):

  0. Contexto do ciclo   → missão (fase + dias na fase); PRE_VOO / fase
                           indeterminada não são analisadas.
  1. variation           → todos Normal/Indeterminado (e sono ok) → FIM.
  2. episódio            → EXERCICIO / ATAQUE / INDEFINIDO (sinais
                           fisiológicos; registro diário = só apoio).
  3. regras espaciais    → rebaixa (adaptação esperada) ou eleva (risco).
  4. persistence         → pico pontual × alteração sustentada.
  5. trend               → piorando / estabilizando.
  6. ambiente (1×)       → CO₂ / cabine como causa provável.
  7. ResultadoAnalise    → gerador de alerta (TODO).

BASELINE: apenas EM_VOO (POS_VOO compara contra o baseline em voo).
SEM DIAGNÓSTICO: os motivos são códigos de regra; decisão clínica é do alerta.
Todos os limiares vêm do repositório (`ParametrosAvaliacao`).
*/
use std::sync::Arc;

use axum::{
    extract::{
        State,
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use tokio::sync::broadcast;
use chrono::{DateTime, Duration, Utc};

use crate::analysis::data_quality::{DataQualityConfig, JanelaTempo};
use crate::analysis::environmental_exposure::{
    EnvironmentalExposureResult, EnvironmentalThresholds,
    calculate_environmental_exposure_de_repositorio,
};
use crate::analysis::persistence::{
    CriterioAlteracao, PersistenceConfig, PersistenceResult, evaluate_persistence_de_repositorio,
};
use crate::analysis::repository::{FaseMissao, Indicador, parse_momento};
use crate::analysis::trend::{TrendConfig, TrendDirection, TrendResult, analyze_trend_de_repositorio};
use crate::analysis::variation::{
    ContextoAtual, Direcao, NivelVariacao, VariacaoErro, VariacaoResultado, VariationConfig,
    calcular_variacao_de_repositorio,
};
use crate::domain::entities::measurement_entity::{BaselineFisiologico, RegistroDiario};
use crate::domain::entities::mission_entity::Missao;
use crate::domain::repositories::measurement_repository::MeasurementRepository;
use crate::domain::repositories::mission_repository::MissionRepository;

/// Janela "atual" da variation.
const JANELA_ATUAL_MIN: i64 = 5;
/// Fase cujo baseline é usado em todas as comparações.
const FASE_BASELINE: FaseMissao = FaseMissao::EmVoo;

const INDICADORES_CONTINUOS: [Indicador; 9] = [
    Indicador::FrequenciaCardiaca,
    Indicador::Spo2,
    Indicador::TemperaturaCorporal,
    Indicador::TemperaturaPele,
    Indicador::TaxaRespiratoria,
    Indicador::Hrv,
    Indicador::PressaoSistolica,
    Indicador::PressaoDiastolica,
    Indicador::Glicose,
];

// ============================================================
// PARÂMETROS (vêm do repositório — nenhum valor inventado aqui)
// ============================================================

#[derive(Debug, Clone, Default)]
pub struct ParametrosAvaliacao {
    pub variation: VariationConfig,
    pub persistence: PersistenceConfig,
    pub trend: TrendConfig,
    pub ambiente: EnvironmentalThresholds,
    pub max_gap_ambiente_s: i64,
    /// Janela de histórico para persistência/tendência (min).
    pub janela_historico_min: i64,
    /// Janela do sono (dias).
    pub janela_sono_dias: i64,
    /// Dias iniciais de EM_VOO com deslocamento de fluidos esperado.
    pub dias_adaptacao_voo: i64,
    /// Dias iniciais de POS_VOO com readaptação à gravidade.
    pub dias_readaptacao_pos_voo: i64,
    /// Meta diária de exercício (resistência + aeróbico), min.
    pub meta_exercicio_min: i64,
    /// Estresse subjetivo >= isto conta como apoio a ATAQUE.
    pub estresse_alto: i64,
    /// Inclinação da FC (bpm/h) acima da qual a subida é "súbita".
    pub fc_subida_subita_bpm_h: f64,
    /// z_resp / z_fc acima disso = hiperventilação desproporcional.
    pub razao_hiperventilacao: f64,
}

// ============================================================
// RESULTADO (entrada do gerador de alerta)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Episodio {
    Exercicio,
    Ataque,
    Indefinido,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Motivo {
    DadoNaoConfiavel(Indicador),
    SemBaseline(Indicador),
    ExercicioEsperado,
    EpisodioAnsiedade,
    AdaptacaoFluidosVoo,
    RiscoQuedaOrtostatica,
    Co2Modulo,
    CabineQuente,
    PossivelInfeccao,
    DeficitSono,
    ExercicioInsuficiente,
    AlteracaoPersistente(Indicador),
    PioraProgressiva(Indicador),
}

#[derive(Debug, Clone)]
pub struct IndicadorAvaliado {
    pub variacao: VariacaoResultado,
    pub nivel_final: NivelVariacao,
    pub persistencia: Option<PersistenceResult>,
    pub tendencia: Option<TrendResult>,
}

#[derive(Debug, Clone)]
pub struct ResultadoAnalise {
    pub astronauta_id: String,
    pub missao_id: String,
    pub momento: DateTime<Utc>,
    pub fase: FaseMissao,
    pub dias_na_fase: i64,
    pub episodio: Episodio,
    pub nivel_final: NivelVariacao,
    pub indicadores: Vec<IndicadorAvaliado>,
    pub exposicao_ambiental: Option<EnvironmentalExposureResult>,
    pub motivos: Vec<Motivo>,
}

#[derive(Debug)]
pub enum AnaliseErro {
    Repositorio(sqlx::Error),
    MissaoNaoEncontrada,
    ParametrosAusentes,
}

impl From<sqlx::Error> for AnaliseErro {
    fn from(e: sqlx::Error) -> Self {
        Self::Repositorio(e)
    }
}

// ============================================================
// AUXILIARES PUROS
// ============================================================

fn grau(n: NivelVariacao) -> u8 {
    match n {
        NivelVariacao::Indeterminado | NivelVariacao::Normal => 0,
        NivelVariacao::Atencao => 1,
        NivelVariacao::Critico => 2,
    }
}

fn alterado(n: NivelVariacao) -> bool {
    grau(n) > 0
}

/// Rebaixa Atenção → Normal. Crítico NUNCA é rebaixado.
fn rebaixar(n: NivelVariacao) -> NivelVariacao {
    if n == NivelVariacao::Atencao { NivelVariacao::Normal } else { n }
}

/// Eleva Atenção → Crítico.
fn elevar(n: NivelVariacao) -> NivelVariacao {
    if n == NivelVariacao::Atencao { NivelVariacao::Critico } else { n }
}

/// Fase e dias na fase a partir de `missoes`. `None` = indeterminada.
pub fn determinar_fase(m: &Missao, agora: DateTime<Utc>) -> Option<(FaseMissao, i64)> {
    let lanc = parse_momento(m.data_lancamento.as_deref()?)?;
    if agora < lanc {
        return Some((FaseMissao::PreVoo, (lanc - agora).num_days()));
    }
    match m.data_retorno.as_deref().and_then(parse_momento) {
        Some(ret) if agora >= ret => Some((FaseMissao::PosVoo, (agora - ret).num_days())),
        _ => Some((FaseMissao::EmVoo, (agora - lanc).num_days())),
    }
}

fn buscar<'a>(v: &'a [VariacaoResultado], i: Indicador) -> Option<&'a VariacaoResultado> {
    v.iter().find(|r| r.indicador == i)
}

fn acima(v: &[VariacaoResultado], i: Indicador) -> bool {
    buscar(v, i).is_some_and(|r| alterado(r.nivel) && r.direcao == Direcao::Acima)
}

fn abaixo(v: &[VariacaoResultado], i: Indicador) -> bool {
    buscar(v, i).is_some_and(|r| alterado(r.nivel) && r.direcao == Direcao::Abaixo)
}

/// Exercício × ataque (maioria de 3 de 4 sinais).
/// Sinais fisiológicos decidem; o registro diário é só evidência de apoio.
pub fn classificar_episodio(
    v: &[VariacaoResultado],
    fc_tendencia: Option<&TrendResult>,
    registro: Option<&RegistroDiario>,
    p: &ParametrosAvaliacao,
) -> Episodio {
    // Sem FC ou respiração alteradas para cima não há episódio a classificar.
    if !acima(v, Indicador::FrequenciaCardiaca) && !acima(v, Indicador::TaxaRespiratoria) {
        return Episodio::Indefinido;
    }

    let (mut exercicio, mut ataque) = (0u8, 0u8);

    // 1. Diastólica: sobe no ataque (vasoconstrição); estável/cai no exercício.
    if acima(v, Indicador::PressaoDiastolica) { ataque += 1 } else { exercicio += 1 }

    // 2. Subida da FC: súbita no ataque, gradual no exercício.
    if let Some(t) = fc_tendencia {
        if t.slope_per_hour > p.fc_subida_subita_bpm_h { ataque += 1 } else { exercicio += 1 }
    }

    // 3. Hiperventilação: respiração desproporcional à FC.
    let z = |i| buscar(v, i).and_then(|r| r.z_score);
    if let (Some(zr), Some(zf)) = (z(Indicador::TaxaRespiratoria), z(Indicador::FrequenciaCardiaca))
        && zf > 0.0
    {
        if zr / zf > p.razao_hiperventilacao { ataque += 1 } else { exercicio += 1 }
    }

    // 4. Apoio: registro diário (exercício do dia × estresse subjetivo).
    if let Some(r) = registro {
        let min_ex = r.exercicio_resistencia_min.unwrap_or(0) + r.exercicio_aerobico_min.unwrap_or(0);
        let estresse = r.nivel_estresse_subjetivo.unwrap_or(0);
        if estresse >= p.estresse_alto { ataque += 1 } else if min_ex > 0 { exercicio += 1 }
    }

    // TODO: 5º sinal — recuperação lenta após o pico (exige janela maior).
    match (exercicio >= 3, ataque >= 3) {
        (true, false) => Episodio::Exercicio,
        (false, true) => Episodio::Ataque,
        _ => Episodio::Indefinido,
    }
}

/// Faixa individual: referência ± z_atencao·dp (usada na persistência).
fn criterio_do_baseline(b: &BaselineFisiologico, z_atencao: f64) -> Option<CriterioAlteracao> {
    let dp = b.desvio_padrao.filter(|d| *d > 0.0)?;
    let c = CriterioAlteracao::ForaDaFaixa {
        minimo: b.valor_referencia - z_atencao * dp,
        maximo: b.valor_referencia + z_atencao * dp,
    };
    c.validar().ok().map(|_| c)
}

// ============================================================
// FLUXO PRINCIPAL
// ============================================================

/// `Ok(None)` = nada a alertar (ou fase não analisável).
pub async fn analisar_ciclo(
    repo: &dyn MeasurementRepository,
    mission_repo: &dyn MissionRepository,
    astronauta_id: &str,
    missao_id: &str,
    agora: DateTime<Utc>,
) -> Result<Option<ResultadoAnalise>, AnaliseErro> {
    // ---------- 0. Contexto do ciclo ----------
    let missao = mission_repo.find_by_id(missao_id).await?.ok_or(AnaliseErro::MissaoNaoEncontrada)?;
    let Some((fase, dias_na_fase)) = determinar_fase(&missao, agora) else {
        return Ok(None); // fase indeterminada: nenhuma regra aplicada
    };
    if fase == FaseMissao::PreVoo {
        return Ok(None); // não existe baseline em voo ainda
    }
    let p: ParametrosAvaliacao = {
        let opt: Option<ParametrosAvaliacao> = repo
            .get_parametros_avaliacao(missao_id)
            .await?;
        opt.ok_or(AnaliseErro::ParametrosAusentes)?
    };

    let contexto = ContextoAtual {
        astronauta_id,
        missao_id: Some(missao_id),
        fase_missao: Some(FASE_BASELINE.nome()),
        agora,
    };
    let janela_atual = JanelaTempo { inicio: agora - Duration::minutes(JANELA_ATUAL_MIN), fim: agora };

    // ---------- 1. Variation ----------
    let mut motivos = Vec::new();
    let mut variacoes = Vec::new();
    let mut baselines = Vec::new();

    for ind in INDICADORES_CONTINUOS {
        let baselines_list = repo
            .list_baselines(astronauta_id, Some(missao_id), Some(FASE_BASELINE.nome()))
            .await?;
        let Some(b) = baselines_list.into_iter().find(|b| b.indicador == ind.nome()) else {
            motivos.push(Motivo::SemBaseline(ind));
            continue;
        };
        match calcular_variacao_de_repositorio(
            repo, contexto, Some(missao_id), ind, &b, janela_atual,
            &DataQualityConfig::padrao_para(ind), &p.variation,
        )

        .await
        {
            Ok(v) => {
                variacoes.push(v);
                baselines.push(b);
            }
            Err(VariacaoErro::Repositorio(e)) => return Err(e.into()),
            // Dado ruim nunca vira alarme fisiológico: só "verificar sensor".
            Err(VariacaoErro::QualidadeInaceitavel(_)) => motivos.push(Motivo::DadoNaoConfiavel(ind)),
            Err(_) => motivos.push(Motivo::SemBaseline(ind)),
        }
    }

    // Sono (diário, janela própria).
    let deficit_sono = avaliar_sono(repo, contexto, &p, agora).await?;
    if deficit_sono {
        motivos.push(Motivo::DeficitSono);
    }

    // Exercício insuficiente em voo (perda óssea/muscular).
    let registro = repo
        .get_daily_log(astronauta_id, &agora.format("%Y-%m-%d").to_string())
        .await?
        .and_then(|r| if r.deleted_at.is_none() { Some(r) } else { None });
    if fase == FaseMissao::EmVoo
        && let Some(r) = &registro
    {
        let total = r.exercicio_resistencia_min.unwrap_or(0) + r.exercicio_aerobico_min.unwrap_or(0);
        if total < p.meta_exercicio_min {
            motivos.push(Motivo::ExercicioInsuficiente);
        }
    }

    let algum_alterado = variacoes.iter().any(|v| alterado(v.nivel));
    let so_sensor = motivos.iter().any(|m| matches!(m, Motivo::DadoNaoConfiavel(_)));
    if !algum_alterado && !deficit_sono && !so_sensor && !motivos.contains(&Motivo::ExercicioInsuficiente) {
        return Ok(None); // caminho barato
    }

    // ---------- 2. Episódio ----------
    let inicio_hist = (agora - Duration::minutes(p.janela_historico_min)).to_rfc3339();
    let fim_hist = agora.to_rfc3339();

    let fc_tendencia = if acima(&variacoes, Indicador::FrequenciaCardiaca) {
        analyze_trend_de_repositorio(
            repo, astronauta_id, Some(missao_id), Indicador::FrequenciaCardiaca.nome(),
            Some(&janela_atual.inicio.to_rfc3339()), Some(&fim_hist), &p.trend,
        )
        .await?
    } else {
        None
    };
    let episodio = classificar_episodio(&variacoes, fc_tendencia.as_ref(), registro.as_ref(), &p);
    match episodio {
        Episodio::Exercicio => motivos.push(Motivo::ExercicioEsperado),
        Episodio::Ataque => motivos.push(Motivo::EpisodioAnsiedade),
        Episodio::Indefinido => {}
    }

    // ---------- 6. Ambiente (1×, só se algo alterou) ----------
    let exposicao = if algum_alterado {
        Some(
            calculate_environmental_exposure_de_repositorio(
                repo, astronauta_id, Some(&janela_atual.inicio.to_rfc3339()), Some(&fim_hist),
                &p.ambiente, p.max_gap_ambiente_s, agora,
            )
            .await?,
        )
    } else {
        None
    };
    let co2_alto = exposicao.as_ref().is_some_and(|e| e.co2_time_above_seconds > 0.0);
    let cabine_quente = exposicao.as_ref().is_some_and(|e| e.temperature_time_above_seconds > 0.0);

    let ortostatica = fase == FaseMissao::PosVoo
        && dias_na_fase <= p.dias_readaptacao_pos_voo
        && abaixo(&variacoes, Indicador::PressaoSistolica)
        && acima(&variacoes, Indicador::FrequenciaCardiaca);
    if ortostatica {
        motivos.push(Motivo::RiscoQuedaOrtostatica);
    }
    let resp_fc_altas =
        acima(&variacoes, Indicador::TaxaRespiratoria) || acima(&variacoes, Indicador::FrequenciaCardiaca);
    if co2_alto && resp_fc_altas {
        motivos.push(Motivo::Co2Modulo);
    }
    if cabine_quente && acima(&variacoes, Indicador::TemperaturaPele) {
        motivos.push(Motivo::CabineQuente);
    }

    // ---------- 3–5. Regras, persistência e tendência por indicador ----------
    let mut indicadores = Vec::new();
    for (v, b) in variacoes.into_iter().zip(baselines) {
        if !alterado(v.nivel) {
            continue;
        }
        let ind = v.indicador;
        let mut nivel = v.nivel;

        // 3. Regras espaciais.
        let por_exercicio = episodio == Episodio::Exercicio
            && matches!(
                (ind, v.direcao),
                (Indicador::FrequenciaCardiaca | Indicador::TaxaRespiratoria
                    | Indicador::PressaoSistolica | Indicador::TemperaturaPele, Direcao::Acima)
                    | (Indicador::Hrv, Direcao::Abaixo)
            );
        let por_fluidos = fase == FaseMissao::EmVoo
            && dias_na_fase <= p.dias_adaptacao_voo
            && matches!(
                ind,
                Indicador::FrequenciaCardiaca | Indicador::PressaoSistolica | Indicador::PressaoDiastolica
            );
        if por_exercicio {
            nivel = rebaixar(nivel);
        } else if por_fluidos && !ortostatica {
            nivel = rebaixar(nivel);
            if !motivos.contains(&Motivo::AdaptacaoFluidosVoo) {
                motivos.push(Motivo::AdaptacaoFluidosVoo);
            }
        }
        if co2_alto && matches!(ind, Indicador::TaxaRespiratoria | Indicador::FrequenciaCardiaca) {
            nivel = elevar(nivel); // causa corrigível: prioriza
        }

        // 4. Persistência (só se ainda alterado — economiza consulta).
        let persistencia = match (alterado(nivel), criterio_do_baseline(&b, p.variation.z_atencao)) {
            (true, Some(c)) => {
                evaluate_persistence_de_repositorio(
                    repo, astronauta_id, Some(missao_id), ind.nome(), Some(&inicio_hist),
                    Some(&fim_hist), &c, &p.persistence, Some(&DataQualityConfig::padrao_para(ind)),
                )
                .await?
            }
            _ => None,
        };
        let persistente = persistencia.as_ref().is_some_and(|r| r.is_persistent);

        // Pico pontual em Atenção não vira alerta (Crítico sempre passa).
        if nivel == NivelVariacao::Atencao && !persistente {
            nivel = NivelVariacao::Normal;
        }
        if persistente {
            motivos.push(Motivo::AlteracaoPersistente(ind));
            if ind == Indicador::TemperaturaCorporal && v.direcao == Direcao::Acima {
                motivos.push(Motivo::PossivelInfeccao);
                nivel = elevar(nivel);
            }
        }

        // 5. Tendência (só para o que restou alterado).
        let tendencia = if alterado(nivel) {
            analyze_trend_de_repositorio(
                repo, astronauta_id, Some(missao_id), ind.nome(), Some(&inicio_hist),
                Some(&fim_hist), &p.trend,
            )
            .await?
        } else {
            None
        };
        let piorando = tendencia.as_ref().is_some_and(|t| {
            matches!(
                (&t.direction, v.direcao),
                (TrendDirection::Increasing, Direcao::Acima) | (TrendDirection::Decreasing, Direcao::Abaixo)
            )
        });
        if piorando {
            motivos.push(Motivo::PioraProgressiva(ind));
        }

        indicadores.push(IndicadorAvaliado { variacao: v, nivel_final: nivel, persistencia, tendencia });
    }

    // ---------- 7. Resultado ----------
    let mut nivel_final = indicadores
        .iter()
        .map(|i| i.nivel_final)
        .max_by_key(|n| grau(*n))
        .unwrap_or(NivelVariacao::Normal);
    if nivel_final == NivelVariacao::Normal
        && (deficit_sono || episodio == Episodio::Ataque || ortostatica || so_sensor)
    {
        nivel_final = NivelVariacao::Atencao;
    }
    if nivel_final == NivelVariacao::Normal && !motivos.contains(&Motivo::ExercicioInsuficiente) {
        return Ok(None); // tudo explicado (exercício/adaptação): sem alarme falso
    }

    let resultado = ResultadoAnalise {
        astronauta_id: astronauta_id.to_string(),
        missao_id: missao_id.to_string(),
        momento: agora,
        fase,
        dias_na_fase,
        episodio,
        nivel_final,
        indicadores,
        exposicao_ambiental: exposicao,
        motivos,
    };

    crate::analysis::alert_stream::hub().publicar(&resultado);
    Ok(Some(resultado))
}

/// Sono abaixo do baseline EM_VOO de forma persistente (ritmo circadiano).
async fn avaliar_sono(
    repo: &dyn MeasurementRepository,
    contexto: ContextoAtual<'_>,
    p: &ParametrosAvaliacao,
    agora: DateTime<Utc>,
) -> Result<bool, AnaliseErro> {
    let ind = Indicador::HorasSono;
    let baselines = repo
        .list_baselines(contexto.astronauta_id, contexto.missao_id, Some(FASE_BASELINE.nome()))
        .await?;
    let Some(b) = baselines.into_iter().find(|b| b.indicador == ind.nome()) else {
        return Ok(false);
    };
    let janela = JanelaTempo { inicio: agora - Duration::days(p.janela_sono_dias), fim: agora };
    let v = match calcular_variacao_de_repositorio(
        repo, contexto, contexto.missao_id, ind, &b, janela,
        &DataQualityConfig::padrao_para(ind), &p.variation,
    )

    .await
    {
        Ok(v) => v,
        Err(VariacaoErro::Repositorio(e)) => return Err(e.into()),
        Err(_) => return Ok(false),
    };
    if !(alterado(v.nivel) && v.direcao == Direcao::Abaixo) {
        return Ok(false);
    }
    let Some(dp) = b.desvio_padrao.filter(|d| *d > 0.0) else {
        return Ok(false);
    };
    let criterio = CriterioAlteracao::AbaixoDe(b.valor_referencia - p.variation.z_atencao * dp);
    let r = evaluate_persistence_de_repositorio(
        repo, contexto.astronauta_id, contexto.missao_id, ind.nome(),
        Some(&janela.inicio.to_rfc3339()), Some(&agora.to_rfc3339()), &criterio,
        &PersistenceConfig { janela_segundos: 0, ..p.persistence.clone() }, None,
    )
    .await?;
    Ok(r.is_some_and(|r| r.is_persistent))
}
