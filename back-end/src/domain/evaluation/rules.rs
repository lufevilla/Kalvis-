/*
REGRAS DETERMINÍSTICAS DE AVALIAÇÃO DE SAÚDE (SEM IA, SEM DIAGNÓSTICO).

A classificação produzida aqui é um NÍVEL OPERACIONAL DE ATENÇÃO
(Normal / Atenção / Alerta / Crítico) derivado EXCLUSIVAMENTE de evidências
estatísticas já calculadas pelos módulos de `analysis`:

- variação em relação ao baseline individual (z-score);
- tendências;
- persistência das alterações;
- exposição ambiental estimada;
- qualidade e disponibilidade dos dados.

NENHUM LIMITE AQUI É CLÍNICO OU UNIVERSAL: todos os limiares chegam prontos
via configuração (protocolo/settings) e o z-score é um desvio em relação ao
PRÓPRIRO histórico do astronauta. A IA nunca determina ou altera o nível.

REGRAS FUNDAMENTAIS:
1. Dados inválidos ou insuficientes NUNCA elevam o risco (são registrados
   como justificativa e como limitação da avaliação).
2. Ausência de dados é diferente de resultado normal: uma avaliação sem
   nenhuma comparação utilizável é marcada como `inconclusivo`.
3. Cada decisão gera justificativa rastreável com os números que a geraram.
4. A agregação é determinística: mesmas evidências → mesmo nível.
*/

use serde::{Deserialize, Serialize};

// ============================================================
// NÍVEL DE RISCO
// ============================================================

/// Nível operacional de atenção. NÃO é um diagnóstico médico.
/// Os nomes são os valores aceitos pela coluna `nivel_risco`
/// da tabela `avaliacoes_saude`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Normal,
    Atencao,
    Alerta,
    Critico,
}

impl RiskLevel {
    /// Nome como gravado no banco (`avaliacoes_saude.nivel_risco`).
    pub fn nome(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Atencao => "ATENCAO",
            Self::Alerta => "ALERTA",
            Self::Critico => "CRITICO",
        }
    }

    /// Interpreta o valor gravado no banco.
    pub fn from_nome(nome: &str) -> Option<Self> {
        match nome {
            "NORMAL" => Some(Self::Normal),
            "ATENCAO" => Some(Self::Atencao),
            "ALERTA" => Some(Self::Alerta),
            "CRITICO" => Some(Self::Critico),
            _ => None,
        }
    }

    /// Severidade ordinal (0 = Normal … 3 = Crítico) para agregação.
    pub fn gravidade(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Atencao => 1,
            Self::Alerta => 2,
            Self::Critico => 3,
        }
    }

    /// O mais grave entre os dois níveis.
    pub fn mais_grave(self, outro: Self) -> Self {
        if outro.gravidade() > self.gravidade() {
            outro
        } else {
            self
        }
    }

    /// Eleva `passos` níveis, limitado a `Critico` (nunca rebaixa).
    pub fn elevar(self, passos: usize) -> Self {
        let alvo = self.gravidade().saturating_add(passos as u8).min(3);
        match alvo {
            0 => Self::Normal,
            1 => Self::Atencao,
            2 => Self::Alerta,
            _ => Self::Critico,
        }
    }
}

// ============================================================
// EVIDÊNCIAS (serializadas em `avaliacoes_saude`)
// ============================================================

/// Situação de um indicador na avaliação. Distingue explicitamente:
/// - `Analisado`: havia dados válidos e comparação utilizável;
/// - `DadosInsuficientes`: havia dados, mas em quantidade/qualidade
///   insuficiente para concluir (baseline ausente, poucas amostras…);
/// - `Indisponivel`: nenhuma medição na janela (ausência ≠ zero);
/// - `Invalido`: havia medições, mas todas rejeitadas pela validação.
///
/// A serialização usa os MESMOS nomes de `nome()` (legíveis e estáveis no
/// JSON gravado em `avaliacoes_saude.indicadores_analisados_json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoIndicador {
    #[serde(rename = "ANALISADO")]
    Analisado,
    #[serde(rename = "DADOS_INSUFICIENTES")]
    DadosInsuficientes,
    #[serde(rename = "INDISPONIVEL")]
    Indisponivel,
    #[serde(rename = "INVALIDO")]
    Invalido,
}

impl EstadoIndicador {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Analisado => "ANALISADO",
            Self::DadosInsuficientes => "DADOS_INSUFICIENTES",
            Self::Indisponivel => "INDISPONIVEL",
            Self::Invalido => "INVALIDO",
        }
    }

    pub fn from_nome(nome: &str) -> Option<Self> {
        match nome {
            "ANALISADO" => Some(Self::Analisado),
            "DADOS_INSUFICIENTES" => Some(Self::DadosInsuficientes),
            "INDISPONIVEL" => Some(Self::Indisponivel),
            "INVALIDO" => Some(Self::Invalido),
            _ => None,
        }
    }
}

/// Evidência de UM indicador, com os números que sustentam a classificação.
/// Serializada em `avaliacoes_saude.indicadores_analisados_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenciaIndicador {
    pub indicador: String,
    pub estado: EstadoIndicador,
    /// Nível da variação individual ("Normal"/"Atencao"/"Critico"/"Indeterminado"),
    /// presente apenas quando houve comparação com o baseline.
    pub nivel_variacao: Option<String>,
    pub z_score: Option<f64>,
    pub valor_atual: Option<f64>,
    pub valor_referencia: Option<f64>,
    /// "Acima"/"Abaixo"/"Estavel".
    pub direcao: Option<String>,
    /// "Aumentando"/"Diminuindo"/"Estavel" (inclinação em unidade/hora
    /// registrada na justificativa quando aplicável).
    pub tendencia: Option<String>,
    pub persistente: Option<bool>,
    pub baseline_desatualizado: Option<bool>,
    pub qualidade_aceitavel: bool,
    pub amostras_validas: usize,
    /// Amostras da janela sem valor (ausência — nunca convertida em zero).
    pub amostras_ausentes: usize,
    /// Amostras rejeitadas por timestamp/limite/ruído (não finitas inclusas).
    pub amostras_invalidas: usize,
    pub justificativa: String,
}

impl EvidenciaIndicador {
    /// Gravidade efetiva da variação individual desta evidência:
    /// Normal/Indeterminado = 0, Atenção = 1, Crítico = 2.
    /// Evidências não analisadas (insuficientes/inválidas/ausentes)
    /// NUNCA contribuem para o risco.
    fn gravidade_variacao(&self) -> u8 {
        if self.estado != EstadoIndicador::Analisado {
            return 0;
        }
        match self.nivel_variacao.as_deref() {
            Some("Atencao") => 1,
            Some("Critico") => 2,
            _ => 0,
        }
    }

    fn alterada(&self) -> bool {
        self.gravidade_variacao() >= 1
    }

    /// A tendência caminha NA MESMA direção do desvio (afastando-se do
    /// baseline): evidência de agravamento. Estável nunca é adversa.
    fn tendencia_adversa(&self) -> bool {
        if !self.alterada() {
            return false;
        }
        matches!(
            (self.direcao.as_deref(), self.tendencia.as_deref()),
            (Some("Acima"), Some("Aumentando")) | (Some("Abaixo"), Some("Diminuindo"))
        )
    }
}

/// Resumo da exposição ambiental estimada (integrada por `analysis::environmental_exposure`).
/// É EXPOSIÇÃO AMBIENTAL DO MÓDULO — não dose pessoal medida.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExposicaoResumo {
    pub co2_exposicao_ppm_hora: f64,
    pub radiacao_dose_msv: f64,
    pub temperatura_exposicao_graus_hora: f64,
    pub pressao_exposicao_kpa_hora: f64,
    pub co2_tempo_acima_segundos: f64,
    pub temperatura_tempo_acima_segundos: f64,
    pub pressao_tempo_fora_segundos: f64,
    pub tempo_analisado_segundos: f64,
    /// Calculado pelo chamador comparando os tempos acima aos limiares e ao
    /// tempo mínimo de evidência configurado.
    pub limites_excedidos: bool,
}

/// Fatores de contexto da avaliação, serializados em
/// `avaliacoes_saude.fatores_contextuais_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatoresContextuais {
    pub janela_analise_segundos: i64,
    pub exposicao_ambiental: Option<ExposicaoResumo>,
    pub indicadores_insuficientes: Vec<String>,
    pub indicadores_indisponiveis: Vec<String>,
    pub indicadores_invalidos: Vec<String>,
    pub avaliacoes_anteriores_consideradas: usize,
    /// Observações gerais (erros parciais de análise, avisos de política…).
    pub observacoes: Vec<String>,
}

/// Configuração determinística das regras (política de elevação).
/// Todos os valores são políticos/estatísticos, nunca clínicos.
#[derive(Debug, Clone)]
pub struct RegrasAvaliacaoConfig {
    /// Persistência de alteração em indicador alterado eleva 1 nível.
    pub elevar_com_persistencia: bool,
    /// Tendência adversa (afastando-se do baseline) em indicador alterado
    /// eleva 1 nível.
    pub elevar_com_tendencia_adversa: bool,
    /// Exposição ambiental acima dos limites configurados eleva 1 nível
    /// (e sozinha já justifica nível de Atenção).
    pub elevar_com_exposicao_ambiental: bool,
    /// Vários indicadores alterados simultaneamente elevam 1 nível.
    pub elevar_com_multiplos_indicadores: bool,
    /// Quantidade de indicadores alterados que caracteriza "múltiplos".
    pub min_indicadores_multiplos: usize,
}

impl Default for RegrasAvaliacaoConfig {
    fn default() -> Self {
        Self {
            elevar_com_persistencia: true,
            elevar_com_tendencia_adversa: true,
            elevar_com_exposicao_ambiental: true,
            elevar_com_multiplos_indicadores: true,
            min_indicadores_multiplos: 2,
        }
    }
}

impl RegrasAvaliacaoConfig {
    /// Rejeita configurações incoerentes.
    pub fn validar(&self) -> Result<(), String> {
        if self.min_indicadores_multiplos < 2 {
            return Err(format!(
                "min_indicadores_multiplos deve ser >= 2 (recebido {})",
                self.min_indicadores_multiplos
            ));
        }
        Ok(())
    }
}

/// Resultado das regras determinísticas.
#[derive(Debug, Clone)]
pub struct ResultadoRegras {
    pub nivel: RiskLevel,
    /// `true` quando NENHUMA comparação utilizável esteve disponível
    /// (nenhum indicador analisado com baseline e nenhuma exposição
    /// excedida): a avaliação NÃO pôde ser concluída.
    pub inconclusivo: bool,
    /// Justificativas rastreáveis, em ordem de contribuição.
    pub justificativas: Vec<String>,
}

/// Agregação determinística das evidências em um nível operacional de risco.
///
/// Regras:
/// - nível base = indicador alterado mais grave (Atenção→1, Crítico→2);
/// - modificadores (cada +1 nível, teto Crítico): persistência, tendência
///   adversa, exposição ambiental excedida, múltiplos indicadores;
/// - dados insuficientes/inválidos/ausentes não elevam nada, mas são
///   registrados nas justificativas;
/// - sem nenhuma comparação utilizável → `inconclusivo` com nível Normal.
pub fn avaliar(
    evidencias: &[EvidenciaIndicador],
    fatores: &FatoresContextuais,
    config: &RegrasAvaliacaoConfig,
) -> ResultadoRegras {
    config
        .validar()
        .expect("config de regras validada antes da avaliação");

    let mut justificativas: Vec<String> = Vec::new();

    // ---- Nível base: indicador alterado mais grave ----
    let mut base = 0u8;
    let mut alteradas: Vec<&EvidenciaIndicador> = Vec::new();
    for e in evidencias {
        let g = e.gravidade_variacao();
        if g > base {
            base = g;
        }
        if e.alterada() {
            alteradas.push(e);
        }
    }

    for e in &alteradas {
        let z = e
            .z_score
            .map(|v| format!("z={v:.2}"))
            .unwrap_or_else(|| "z=indeterminado".to_string());
        let valores = match (e.valor_atual, e.valor_referencia) {
            (Some(atual), Some(refe)) => {
                format!("média atual {atual:.1} vs referência individual {refe:.1}")
            }
            _ => "valores indisponíveis".to_string(),
        };
        let persist = match e.persistente {
            Some(true) => "alteração persistente".to_string(),
            Some(false) => "sem persistência".to_string(),
            None => "persistência não avaliada".to_string(),
        };
        let tendencia = e.tendencia.clone().unwrap_or_else(|| "não avaliada".into());
        justificativas.push(format!(
            "{}: variação nível {} ({}) — {}; direção {:?}; {} — tendência {}.",
            e.indicador,
            e.nivel_variacao.clone().unwrap_or_default(),
            z,
            valores,
            e.direcao.clone().unwrap_or_default(),
            persist,
            tendencia
        ));
        if e.baseline_desatualizado == Some(true) {
            justificativas.push(format!(
                "{}: baseline individual desatualizado — interpretar a comparação com cautela.",
                e.indicador
            ));
        }
    }

    // ---- Modificadores ----
    let mut passos = 0usize;

    let persistente =
        config.elevar_com_persistencia && alteradas.iter().any(|e| e.persistente == Some(true));
    if persistente {
        passos += 1;
        justificativas.push(
            "Modificador: alteração persistente em indicador alterado (eleva 1 nível).".to_string(),
        );
    }

    let tendencia_adversa =
        config.elevar_com_tendencia_adversa && alteradas.iter().any(|e| e.tendencia_adversa());
    if tendencia_adversa {
        passos += 1;
        justificativas.push(
            "Modificador: tendência se afastando do baseline individual em indicador alterado \
             (eleva 1 nível)."
                .to_string(),
        );
    }

    let exposicao = fatores
        .exposicao_ambiental
        .as_ref()
        .filter(|ex| ex.limites_excedidos);
    if let Some(ex) = exposicao {
        justificativas.push(format!(
            "Exposição ambiental estimada acima dos limites configurados \
             (CO₂ {:.0} s acima, temperatura {:.0} s acima, pressão {:.0} s fora da faixa \
             em {:.0} s analisados) — exposição do módulo, não dose pessoal.",
            ex.co2_tempo_acima_segundos,
            ex.temperatura_tempo_acima_segundos,
            ex.pressao_tempo_fora_segundos,
            ex.tempo_analisado_segundos
        ));
        if config.elevar_com_exposicao_ambiental {
            passos += 1;
            justificativas
                .push("Modificador: exposição ambiental acima dos limites (eleva 1 nível).".into());
        }
    }

    let multiplos = config.elevar_com_multiplos_indicadores
        && alteradas.len() >= config.min_indicadores_multiplos;
    if multiplos {
        passos += 1;
        justificativas.push(format!(
            "Modificador: {} indicadores alterados simultaneamente (eleva 1 nível).",
            alteradas.len()
        ));
    }

    let nivel = RiskLevel::from_gravidade(base).elevar(passos);

    // ---- Justificativas dos indicadores NÃO analisados ----
    for e in evidencias {
        match e.estado {
            EstadoIndicador::Analisado => {}
            EstadoIndicador::DadosInsuficientes => {
                justificativas.push(format!(
                    "{}: dados insuficientes ({} amostras válidas, {} ausentes, {} inválidas) — \
                     NÃO contribui para o risco.",
                    e.indicador, e.amostras_validas, e.amostras_ausentes, e.amostras_invalidas
                ));
            }
            EstadoIndicador::Indisponivel => {
                justificativas.push(format!(
                    "{}: nenhuma medição disponível na janela — ausência de dados não é \
                     resultado normal e não contribui para o risco.",
                    e.indicador
                ));
            }
            EstadoIndicador::Invalido => {
                justificativas.push(format!(
                    "{}: {} amostras rejeitadas pela validação de qualidade — dados inválidos \
                     NÃO elevam o risco.",
                    e.indicador, e.amostras_invalidas
                ));
            }
        }
    }

    // ---- Inconclusividade ----
    let comparacoes_usaveis = evidencias
        .iter()
        .any(|e| e.estado == EstadoIndicador::Analisado && e.nivel_variacao.is_some());
    let exposicao_util = exposicao.is_some();
    let inconclusivo = !comparacoes_usaveis && !exposicao_util;

    if inconclusivo {
        justificativas.push(
            "Avaliação inconclusiva: nenhum indicador pôde ser comparado ao baseline individual \
             e não há exposição ambiental excedida. Isso NÃO significa resultado normal."
                .to_string(),
        );
    } else if nivel == RiskLevel::Normal {
        if alteradas.is_empty() {
            justificativas
                .push("Nenhuma alteração identificada nos indicadores analisados.".into());
        }
        justificativas
            .push("Classificação operacional de atenção NORMAL — não é diagnóstico médico.".into());
    }

    ResultadoRegras {
        nivel,
        inconclusivo,
        justificativas,
    }
}

impl RiskLevel {
    fn from_gravidade(g: u8) -> Self {
        match g {
            0 => Self::Normal,
            1 => Self::Atencao,
            2 => Self::Alerta,
            _ => Self::Critico,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evid(
        indicador: &str,
        estado: EstadoIndicador,
        nivel: Option<&str>,
        z: Option<f64>,
        direcao: Option<&str>,
        tendencia: Option<&str>,
        persistente: Option<bool>,
    ) -> EvidenciaIndicador {
        EvidenciaIndicador {
            indicador: indicador.to_string(),
            estado,
            nivel_variacao: nivel.map(str::to_string),
            z_score: z,
            valor_atual: Some(80.0),
            valor_referencia: Some(70.0),
            direcao: direcao.map(str::to_string),
            tendencia: tendencia.map(str::to_string),
            persistente,
            baseline_desatualizado: None,
            qualidade_aceitavel: estado == EstadoIndicador::Analisado,
            amostras_validas: 10,
            amostras_ausentes: 0,
            amostras_invalidas: 0,
            justificativa: String::new(),
        }
    }

    fn fatores(exposicao: Option<ExposicaoResumo>) -> FatoresContextuais {
        FatoresContextuais {
            janela_analise_segundos: 300,
            exposicao_ambiental: exposicao,
            indicadores_insuficientes: Vec::new(),
            indicadores_indisponiveis: Vec::new(),
            indicadores_invalidos: Vec::new(),
            avaliacoes_anteriores_consideradas: 0,
            observacoes: Vec::new(),
        }
    }

    #[test]
    fn niveis_de_risco_têm_nomes_compatíveis_com_o_banco() {
        for nome in ["NORMAL", "ATENCAO", "ALERTA", "CRITICO"] {
            let nivel = RiskLevel::from_nome(nome).expect("nome válido");
            assert_eq!(nivel.nome(), nome);
        }
        assert_eq!(RiskLevel::from_nome("FOO"), None);
        // Ordenação por gravidade.
        assert!(RiskLevel::Critico > RiskLevel::Alerta);
        assert!(RiskLevel::Alerta > RiskLevel::Atencao);
        assert!(RiskLevel::Atencao > RiskLevel::Normal);
    }

    #[test]
    fn elevacao_e_limitada_ao_critico() {
        assert_eq!(RiskLevel::Normal.elevar(1), RiskLevel::Atencao);
        assert_eq!(RiskLevel::Alerta.elevar(5), RiskLevel::Critico);
        assert_eq!(RiskLevel::Critico.elevar(2), RiskLevel::Critico);
    }

    #[test]
    fn variacao_normal_produz_nivel_normal_conclusivo() {
        let evids = vec![evid(
            "SPO2",
            EstadoIndicador::Analisado,
            Some("Normal"),
            Some(0.4),
            Some("Estavel"),
            Some("Estavel"),
            Some(false),
        )];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        assert_eq!(r.nivel, RiskLevel::Normal);
        assert!(!r.inconclusivo);
        assert!(
            r.justificativas
                .iter()
                .any(|j| j.contains("Classificação operacional de atenção NORMAL"))
        );
    }

    #[test]
    fn variacao_em_atencao_eleva_o_nivel() {
        let evids = vec![evid(
            "FREQUENCIA_CARDIACA",
            EstadoIndicador::Analisado,
            Some("Atencao"),
            Some(2.3),
            Some("Acima"),
            Some("Estavel"),
            Some(false),
        )];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        assert_eq!(r.nivel, RiskLevel::Atencao);
        assert!(!r.inconclusivo);
        assert!(
            r.justificativas
                .iter()
                .any(|j| j.contains("FREQUENCIA_CARDIACA") && j.contains("z=2.30"))
        );
    }

    #[test]
    fn variacao_critica_e_persistente_alcancam_critico() {
        let evids = vec![evid(
            "SPO2",
            EstadoIndicador::Analisado,
            Some("Critico"),
            Some(-3.5),
            Some("Abaixo"),
            Some("Diminuindo"),
            Some(true),
        )];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        // Base 2 (Crítico) + persistência (+1) + tendência adversa (+1), teto 3.
        assert_eq!(r.nivel, RiskLevel::Critico);
    }

    #[test]
    fn dados_insuficientes_sao_inconclusivos_e_nao_elevam_risco() {
        let evids = vec![
            evid(
                "SPO2",
                EstadoIndicador::DadosInsuficientes,
                None,
                None,
                None,
                None,
                None,
            ),
            evid(
                "HRV",
                EstadoIndicador::Indisponivel,
                None,
                None,
                None,
                None,
                None,
            ),
            evid(
                "GLICOSE",
                EstadoIndicador::Invalido,
                None,
                None,
                None,
                None,
                None,
            ),
        ];
        let mut f = fatores(None);
        f.indicadores_insuficientes = vec!["SPO2".into()];
        f.indicadores_indisponiveis = vec!["HRV".into()];
        f.indicadores_invalidos = vec!["GLICOSE".into()];
        let r = avaliar(&evids, &f, &RegrasAvaliacaoConfig::default());

        assert_eq!(r.nivel, RiskLevel::Normal);
        assert!(r.inconclusivo, "sem nenhuma comparação utilizável");
        assert!(
            r.justificativas
                .iter()
                .any(|j| j.contains("ausência de dados não é resultado normal"))
        );
        assert!(
            r.justificativas
                .iter()
                .any(|j| j.contains("NÃO elevam o risco"))
        );
        assert!(r.justificativas.iter().any(|j| j.contains("inconclusiva")));
    }

    #[test]
    fn dados_invalidos_nunca_elevam_o_risco() {
        // Indicador alterado é INVALIDO (dados ruins); não pode elevar nada.
        let evids = vec![evid(
            "FREQUENCIA_CARDIACA",
            EstadoIndicador::Invalido,
            Some("Critico"), // classificação espúria não deve contar
            Some(9.0),
            None,
            None,
            None,
        )];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        assert_eq!(r.nivel, RiskLevel::Normal);
        assert!(r.inconclusivo);
    }

    #[test]
    fn evidencias_conflitantes_usam_o_mais_grave_e_registram_as_duas() {
        let evids = vec![
            evid(
                "FREQUENCIA_CARDIACA",
                EstadoIndicador::Analisado,
                Some("Atencao"),
                Some(2.1),
                Some("Acima"),
                Some("Estavel"),
                Some(false),
            ),
            evid(
                "SPO2",
                EstadoIndicador::Analisado,
                Some("Critico"),
                Some(-3.2),
                Some("Abaixo"),
                Some("Estavel"),
                Some(false),
            ),
        ];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        // Crítico → base 2 = ALERTA, com 2 indicadores alterados o
        // modificador de múltiplos eleva 1 → CRITICO.
        assert_eq!(r.nivel, RiskLevel::Critico);
        assert!(
            r.justificativas
                .iter()
                .any(|j| j.contains("FREQUENCIA_CARDIACA"))
        );
        assert!(r.justificativas.iter().any(|j| j.contains("SPO2")));
    }

    #[test]
    fn multiplos_indicadores_alterados_elevam_um_nivel() {
        let evids = vec![
            evid(
                "FREQUENCIA_CARDIACA",
                EstadoIndicador::Analisado,
                Some("Atencao"),
                Some(2.0),
                Some("Acima"),
                None,
                Some(false),
            ),
            evid(
                "TAXA_RESPIRATORIA",
                EstadoIndicador::Analisado,
                Some("Atencao"),
                Some(2.2),
                Some("Acima"),
                None,
                Some(false),
            ),
        ];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        assert_eq!(r.nivel, RiskLevel::Alerta, "2 × Atenção + modificador");
        assert!(
            r.justificativas
                .iter()
                .any(|j| j.contains("indicadores alterados simultaneamente"))
        );
    }

    #[test]
    fn exposicao_ambiental_excedida_justifica_atencao_mesmo_sem_variacao() {
        let mut ex = ExposicaoResumo::default();
        ex.co2_tempo_acima_segundos = 300.0;
        ex.limites_excedidos = true;
        let evids = vec![evid(
            "SPO2",
            EstadoIndicador::Indisponivel,
            None,
            None,
            None,
            None,
            None,
        )];
        let r = avaliar(
            &evids,
            &fatores(Some(ex)),
            &RegrasAvaliacaoConfig::default(),
        );

        assert_eq!(r.nivel, RiskLevel::Atencao);
        assert!(!r.inconclusivo, "exposição é evidência utilizável");
    }

    #[test]
    fn baseline_desatualizado_e_registrado_sem_elevar_o_nivel() {
        let mut e = evid(
            "SPO2",
            EstadoIndicador::Analisado,
            Some("Atencao"),
            Some(2.4),
            Some("Abaixo"),
            Some("Estavel"),
            Some(false),
        );
        e.baseline_desatualizado = Some(true);
        let evids = vec![e];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        assert_eq!(r.nivel, RiskLevel::Atencao, "desatualização não eleva");
        assert!(r.justificativas.iter().any(|j| j.contains("desatualizado")));
    }

    #[test]
    fn indicador_normal_ao_lado_de_indicador_alterado_mantem_o_mais_grave() {
        let evids = vec![
            evid(
                "SPO2",
                EstadoIndicador::Analisado,
                Some("Normal"),
                Some(0.1),
                Some("Estavel"),
                None,
                Some(false),
            ),
            evid(
                "HRV",
                EstadoIndicador::Analisado,
                Some("Atencao"),
                Some(2.5),
                Some("Abaixo"),
                None,
                Some(true),
            ),
        ];
        let r = avaliar(&evids, &fatores(None), &RegrasAvaliacaoConfig::default());

        // Atenção (base) + persistência (+1) = ALERTA.
        assert_eq!(r.nivel, RiskLevel::Alerta);
    }

    #[test]
    fn justificativas_nunca_ficam_vazias() {
        let r = avaliar(&[], &fatores(None), &RegrasAvaliacaoConfig::default());
        assert!(r.inconclusivo);
        assert!(!r.justificativas.is_empty());

        let r = avaliar(
            &[evid(
                "HRV",
                EstadoIndicador::Analisado,
                Some("Normal"),
                Some(0.0),
                Some("Estavel"),
                None,
                None,
            )],
            &fatores(None),
            &RegrasAvaliacaoConfig::default(),
        );
        assert!(!r.justificativas.is_empty());
    }

    #[test]
    fn configuracao_desativa_modificadores() {
        let evids = vec![
            evid(
                "SPO2",
                EstadoIndicador::Analisado,
                Some("Atencao"),
                Some(2.0),
                Some("Abaixo"),
                Some("Diminuindo"),
                Some(true),
            ),
            evid(
                "HRV",
                EstadoIndicador::Analisado,
                Some("Atencao"),
                Some(2.1),
                Some("Abaixo"),
                None,
                None,
            ),
        ];
        let cfg = RegrasAvaliacaoConfig {
            elevar_com_persistencia: false,
            elevar_com_tendencia_adversa: false,
            elevar_com_exposicao_ambiental: false,
            elevar_com_multiplos_indicadores: false,
            min_indicadores_multiplos: 2,
        };
        let r = avaliar(&evids, &fatores(None), &cfg);
        assert_eq!(r.nivel, RiskLevel::Atencao, "sem modificadores");
        assert!(cfg.validar().is_ok());
        assert!(
            RegrasAvaliacaoConfig {
                min_indicadores_multiplos: 1,
                ..cfg
            }
            .validar()
            .is_err()
        );
    }
}
