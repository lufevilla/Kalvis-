/*
MODELOS DA INTEGRAÇÃO COM OLLAMA (COPILOTO EXPLICATIVO).

A IA atua APENAS como copiloto explicativo: recebe o resultado JÁ
PRODUZIDO pelas regras determinísticas (nível de risco, evidências,
fatores contextuais) e devolve TEXTO explicativo. Ela NUNCA:
- altera o nível de risco;
- cria valores de sensores ou inventa evidências;
- faz diagnóstico;
- substitui protocolos determinísticos;
- oculta incertezas ou limitações.

NÃO há RAG, recuperação de documentos, embeddings ou busca semântica.
*/

use serde::{Deserialize, Serialize};

use crate::domain::evaluation::rules::{EvidenciaIndicador, FatoresContextuais, RiskLevel};

/// Solicitação de orientação enviada ao modelo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolicitacaoOrientacao {
    pub astronauta_id: String,
    pub avaliacao_id: String,
    pub timestamp_utc: String,
    /// Nível JÁ calculado pelas regras determinísticas (imutável pela IA).
    pub nivel_risco: String,
    pub inconclusivo: bool,
    pub resumo_deterministico: String,
    pub justificativas: Vec<String>,
    pub evidencias: Vec<EvidenciaIndicador>,
    pub fatores_contextuais: Option<FatoresContextuais>,
}

/// Contexto estruturado da avaliação, formatado no prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextoAvaliacaoIa {
    pub nivel_risco: String,
    pub gravidade: u8,
    pub inconclusivo: bool,
    pub indicadores_analisados: Vec<String>,
    pub indicadores_insuficientes: Vec<String>,
    pub indicadores_indisponiveis: Vec<String>,
    pub indicadores_invalidos: Vec<String>,
    pub tendencias: Vec<String>,
    pub persistencias: Vec<String>,
    pub exposicao_ambiental: Option<String>,
    pub qualidade_dados: String,
    pub limitacoes: Vec<String>,
}

/// Estado de disponibilidade da IA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisponibilidadeIa {
    /// Cliente configurado e serviço respondeu.
    Disponivel,
    /// Sem cliente configurado (IA desativada por configuração).
    Desativada,
    /// Cliente configurado, mas o serviço falhou (com motivo).
    Indisponivel(String),
}

/// Resposta CONTROLADA do gerador de orientação.
/// `disponivel = false` quando a IA está indisponível — o restante do
/// sistema continua funcionando normalmente nesse caso.
#[derive(Debug, Clone)]
pub struct RespostaOrientacao {
    pub disponivel: bool,
    pub disponibilidade: DisponibilidadeIa,
    /// Resumo compreensível produzido pela IA (quando disponível).
    pub resumo: Option<String>,
    /// Texto completo (evidências, fatores associados, limitações).
    pub explicacao: Option<String>,
    /// Recomendações operacionais gerais, quando aplicáveis.
    pub orientacao: Option<String>,
}

/// Corpo de pedido do endpoint `/api/generate` do Ollama.
#[derive(Debug, Serialize)]
pub struct OllamaGeneratePedido {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    /// Opções de geração (temperatura baixa: texto factual, não criativo).
    pub options: OllamaOpcoes,
}

#[derive(Debug, Serialize)]
pub struct OllamaOpcoes {
    pub temperature: f64,
}

/// Resposta do endpoint `/api/generate` (com `stream: false`).
#[derive(Debug, Deserialize)]
pub struct OllamaGenerateResposta {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub done: bool,
}

/// Monta o contexto estruturado a partir da solicitação.
pub fn construir_contexto(solicitacao: &SolicitacaoOrientacao) -> ContextoAvaliacaoIa {
    let nivel = RiskLevel::from_nome(&solicitacao.nivel_risco);
    let gravidade = nivel.map(|n| n.gravidade()).unwrap_or(0);

    let mut tendencias = Vec::new();
    let mut persistencias = Vec::new();
    let mut analisados = Vec::new();
    for e in &solicitacao.evidencias {
        if e.estado == crate::domain::evaluation::rules::EstadoIndicador::Analisado {
            analisados.push(e.indicador.clone());
        }
        if let Some(t) = &e.tendencia
            && t != "Estavel"
        {
            tendencias.push(format!("{}: {}", e.indicador, t));
        }
        if e.persistente == Some(true) {
            persistencias.push(e.indicador.clone());
        }
    }

    let (insuficientes, indisponiveis, invalidos, limitacoes_dados) =
        match &solicitacao.fatores_contextuais {
            Some(f) => (
                f.indicadores_insuficientes.clone(),
                f.indicadores_indisponiveis.clone(),
                f.indicadores_invalidos.clone(),
                if f.indicadores_indisponiveis.is_empty()
                    && f.indicadores_invalidos.is_empty()
                    && f.indicadores_insuficientes.is_empty()
                {
                    "dados completos na janela de análise".to_string()
                } else {
                    format!(
                        "há indicadores sem dados suficientes na janela de {} s",
                        f.janela_analise_segundos
                    )
                },
            ),
            None => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                "fatores contextuais não registrados".to_string(),
            ),
        };

    let exposicao = solicitacao
        .fatores_contextuais
        .as_ref()
        .and_then(|f| f.exposicao_ambiental.as_ref())
        .map(|ex| {
            format!(
                "CO₂ {:.0} ppm·h ({:.0} s acima do limite), radiação {:.3} mSv, temperatura \
                 {:.1} °C·h, pressão {:.2} kPa·h — EXPOSIÇÃO AMBIENTAL ESTIMADA DO MÓDULO, \
                 não dose pessoal.",
                ex.co2_exposicao_ppm_hora,
                ex.co2_tempo_acima_segundos,
                ex.radiacao_dose_msv,
                ex.temperatura_exposicao_graus_hora,
                ex.pressao_exposicao_kpa_hora
            )
        });

    let mut limitacoes = vec![limitacoes_dados];
    if solicitacao.inconclusivo {
        limitacoes.push(
            "avaliação inconclusiva: nenhum indicador pôde ser comparado ao baseline individual"
                .to_string(),
        );
    }
    for obs in solicitacao
        .fatores_contextuais
        .as_ref()
        .map(|f| f.observacoes.iter().cloned())
        .into_iter()
        .flatten()
    {
        limitacoes.push(obs);
    }

    ContextoAvaliacaoIa {
        nivel_risco: solicitacao.nivel_risco.clone(),
        gravidade,
        inconclusivo: solicitacao.inconclusivo,
        indicadores_analisados: analisados,
        indicadores_insuficientes: insuficientes,
        indicadores_indisponiveis: indisponiveis,
        indicadores_invalidos: invalidos,
        tendencias,
        persistencias,
        exposicao_ambiental: exposicao,
        qualidade_dados: String::new(),
        limitacoes,
    }
}

/// Prompt enviado ao modelo. Reforça o papel de copiloto explicativo e as
/// proibições (sem diagnóstico, sem alterar o nível, sem inventar dados).
pub fn construir_prompt(solicitacao: &SolicitacaoOrientacao) -> String {
    let contexto = construir_contexto(solicitacao);
    let evidencias: Vec<String> = solicitacao
        .evidencias
        .iter()
        .map(|e| {
            format!(
                "- {}: estado {}, nível de variação {}, z-score {}, direção {}, \
                 tendência {}, persistência {}, baseline desatualizado {}, \
                 amostras válidas/ausentes/inválidas {}/{}/{}",
                e.indicador,
                e.estado.nome(),
                e.nivel_variacao.as_deref().unwrap_or("-"),
                e.z_score.map(|z| format!("{z:.2}")).unwrap_or("-".into()),
                e.direcao.as_deref().unwrap_or("-"),
                e.tendencia.as_deref().unwrap_or("-"),
                e.persistente.map(|p| p.to_string()).unwrap_or("-".into()),
                e.baseline_desatualizado
                    .map(|p| p.to_string())
                    .unwrap_or("-".into()),
                e.amostras_validas,
                e.amostras_ausentes,
                e.amostras_invalidas
            )
        })
        .collect();

    let justificativas: Vec<String> = solicitacao
        .justificativas
        .iter()
        .map(|j| format!("- {j}"))
        .collect();

    format!(
        r#"Você é o copiloto EXPLICATIVO de um sistema de monitoramento de saúde para \
astronautas. O nível de risco, as evidências e a classificação JÁ foram produzidos por \
regras determinísticas. Sua única função é EXPLICAR o resultado ao time de operações.

REGRAS INEGOCIÁVEIS:
- NÃO altere, conteste ou reavalie o nível de risco informado: ele é "{}" e está FINALIZADO.
- NÃO faça diagnóstico médico e NÃO use linguagem de diagnóstico.
- NÃO crie valores de sensores, evidências ou dados que não estejam abaixo.
- NÃO substitua os protocolos determinísticos nem sugira outro nível.
- DESTAQUE as incertezas e limitações dos dados; não oculte nada.
- Os "limites" mencionados são limites OPERACIONAIS/ESTATÍSTICOS configuráveis, \
não limites clínicos universais.

CONTEXTO DA AVALIAÇÃO (astronauta {}, avaliação {}, {}):
- Nível operacional final: {} (inconclusivo: {})
- Resumo determinístico: {}
- Indicadores analisados: {}
- Indicadores com dados insuficientes: {}
- Indicadores indisponíveis (sem medição): {}
- Indicadores com dados inválidos: {}
- Tendências relevantes: {}
- Alterações persistentes: {}
- Exposição ambiental: {}
- Limitações da análise: {}

JUSTIFICATIVAS DETERMINÍSTICAS:
{}

EVIDÊNCIAS POR INDICADOR:
{}

Responda EXATAMENTE neste formato, em português, cada seção separada por uma linha:

RESUMO: <resumo compreensível da avaliação em 2 a 4 frases>
FATORES: <possíveis fatores associados às evidências, com as devidas incertezas>
LIMITACOES: <limitações da análise e o que os dados não permitem concluir>
RECOMENDACOES: <recomendações operacionais gerais (monitoramento, confirmação com \
outros dados, comunicação ao time de saúde); se nada for aplicável, escreva \
"Nenhuma recomendação operacional adicional">"#,
        solicitacao.nivel_risco,
        solicitacao.astronauta_id,
        solicitacao.avaliacao_id,
        solicitacao.timestamp_utc,
        contexto.nivel_risco,
        contexto.inconclusivo,
        solicitacao.resumo_deterministico,
        if contexto.indicadores_analisados.is_empty() {
            "nenhum".to_string()
        } else {
            contexto.indicadores_analisados.join(", ")
        },
        if contexto.indicadores_insuficientes.is_empty() {
            "nenhum".to_string()
        } else {
            contexto.indicadores_insuficientes.join(", ")
        },
        if contexto.indicadores_indisponiveis.is_empty() {
            "nenhum".to_string()
        } else {
            contexto.indicadores_indisponiveis.join(", ")
        },
        if contexto.indicadores_invalidos.is_empty() {
            "nenhum".to_string()
        } else {
            contexto.indicadores_invalidos.join(", ")
        },
        if contexto.tendencias.is_empty() {
            "nenhuma".to_string()
        } else {
            contexto.tendencias.join("; ")
        },
        if contexto.persistencias.is_empty() {
            "nenhuma".to_string()
        } else {
            contexto.persistencias.join(", ")
        },
        contexto
            .exposicao_ambiental
            .as_deref()
            .unwrap_or("nenhuma registrada"),
        contexto
            .limitacoes
            .iter()
            .map(|l| format!("• {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
        if justificativas.is_empty() {
            "- (nenhuma justificativa registrada)".to_string()
        } else {
            justificativas.join("\n")
        },
        if evidencias.is_empty() {
            "- (nenhuma evidência registrada)".to_string()
        } else {
            evidencias.join("\n")
        },
    )
}

/// Extrai as seções `RESUMO:` / `FATORES:` / `LIMITACOES:` / `RECOMENDACOES:`
/// da resposta do modelo. Interpretação tolerante: seções ausentes ficam
/// `None` e o texto bruto não é descartado. O conteúdo de cada seção vai
/// ATÉ o rótulo da próxima (nunca o inclui).
pub fn interpretar_resposta(texto: &str) -> (Option<String>, Option<String>, Option<String>) {
    let secoes = ["RESUMO", "FATORES", "LIMITACOES", "RECOMENDACOES"];

    // (índice, início do RÓTULO, início do CONTEÚDO) de cada seção achada.
    let mut marcas: Vec<(usize, usize, usize)> = Vec::new();
    for (indice, nome) in secoes.iter().enumerate() {
        let padrao = format!("{nome}:");
        if let Some(pos) = texto.find(&padrao) {
            marcas.push((indice, pos, pos + padrao.len()));
        }
    }
    marcas.sort_by_key(|&(_, inicio, _)| inicio);

    let extrair = |indice: usize| -> Option<String> {
        let &(_, _, conteudo) = marcas.iter().find(|(i, _, _)| *i == indice)?;
        let fim = marcas
            .iter()
            .map(|&(_, inicio_rotulo, _)| inicio_rotulo)
            .find(|&pos| pos > conteudo)
            .unwrap_or(texto.len());
        let texto_secao = texto[conteudo..fim].trim();
        (!texto_secao.is_empty()).then(|| texto_secao.to_string())
    };

    (extrair(0), extrair(2), extrair(3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::evaluation::rules::{EstadoIndicador, ExposicaoResumo, FatoresContextuais};

    fn solicitacao() -> SolicitacaoOrientacao {
        SolicitacaoOrientacao {
            astronauta_id: "A1".into(),
            avaliacao_id: "AV1".into(),
            timestamp_utc: "2025-01-15T08:00:00Z".into(),
            nivel_risco: "ATENCAO".into(),
            inconclusivo: false,
            resumo_deterministico: "1 indicador alterado".into(),
            justificativas: vec!["SPO2: variação nível Atencao (z=-2.20).".into()],
            evidencias: vec![EvidenciaIndicador {
                indicador: "SPO2".into(),
                estado: EstadoIndicador::Analisado,
                nivel_variacao: Some("Atencao".into()),
                z_score: Some(-2.2),
                valor_atual: Some(91.0),
                valor_referencia: Some(98.0),
                direcao: Some("Abaixo".into()),
                tendencia: Some("Diminuindo".into()),
                persistente: Some(true),
                baseline_desatualizado: None,
                qualidade_aceitavel: true,
                amostras_validas: 12,
                amostras_ausentes: 0,
                amostras_invalidas: 1,
                justificativa: String::new(),
            }],
            fatores_contextuais: Some(FatoresContextuais {
                janela_analise_segundos: 300,
                exposicao_ambiental: Some(ExposicaoResumo {
                    co2_exposicao_ppm_hora: 350.0,
                    ..ExposicaoResumo::default()
                }),
                indicadores_insuficientes: vec!["HRV".into()],
                indicadores_indisponiveis: vec!["GLICOSE".into()],
                indicadores_invalidos: Vec::new(),
                avaliacoes_anteriores_consideradas: 1,
                observacoes: vec!["aviso de teste".into()],
            }),
        }
    }

    #[test]
    fn prompt_reforca_o_papel_de_copiloto_explicativo() {
        let prompt = construir_prompt(&solicitacao());
        assert!(prompt.contains("NÃO altere, conteste ou reavalie o nível de risco"));
        assert!(prompt.contains("NÃO faça diagnóstico"));
        assert!(prompt.contains("não dose pessoal"));
        assert!(prompt.contains("ATENCAO"));
        assert!(prompt.contains("z=-2.20") || prompt.contains("z-score"));
        assert!(prompt.contains("aviso de teste"));
    }

    #[test]
    fn contexto_reflete_inconclusividade_e_limitacoes() {
        let mut s = solicitacao();
        s.inconclusivo = true;
        let c = construir_contexto(&s);
        assert!(c.inconclusivo);
        assert_eq!(c.indicadores_insuficientes, vec!["HRV"]);
        assert!(c.limitacoes.iter().any(|l| l.contains("inconclusiva")));
        assert!(c.exposicao_ambiental.is_some());
        // `qualidade_dados` é preenchido em uso; aqui é campo reservado.
        assert!(c.qualidade_dados.is_empty());
    }

    #[test]
    fn resposta_do_modelo_e_seccionada_de_forma_tolerante() {
        let texto = "RESUMO: SpO2 abaixo do padrão individual.\n\
                     FATORES: possível descondicionamento.\n\
                     LIMITACOES: janela curta.\n\
                     RECOMENDACOES: monitorar próximas coletas.";
        let (resumo, limitacoes, recomendacoes) = interpretar_resposta(texto);
        assert_eq!(resumo.as_deref(), Some("SpO2 abaixo do padrão individual."));
        assert_eq!(limitacoes.as_deref(), Some("janela curta."));
        assert_eq!(
            recomendacoes.as_deref(),
            Some("monitorar próximas coletas.")
        );

        // Seções ausentes → None, sem pânico.
        let (r, l, o) = interpretar_resposta("texto sem seções");
        assert!(r.is_none() && l.is_none() && o.is_none());
    }

    #[test]
    fn secoes_parciais_sao_extraidas() {
        let texto = "RESUMO: algo\nFATORES: outra coisa";
        let (resumo, limitacoes, _) = interpretar_resposta(texto);
        assert_eq!(resumo.as_deref(), Some("algo"));
        assert_eq!(limitacoes, None);
    }
}
