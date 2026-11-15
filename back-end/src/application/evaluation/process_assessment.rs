/*
CASO DE USO: PROCESSAMENTO DA AVALIAÇÃO DE SAÚDE.

Orquestra, em uma única execução determinística, todo o ciclo de
avaliação de UM astronauta em UM momento:

 1. valida a entrada;
 2. constrói o contexto (dados recentes + qualidade + baselines + histórico);
 3. garante/atualiza baselines individuais conforme a política de idade;
 4. avalia a variação em relação ao baseline individual (analysis::variation);
 5. avalia tendências (analysis::trend);
 6. avalia persistência das alterações (analysis::persistence);
 7. estima a exposição ambiental (analysis::environmental_exposure);
 8. agrega as evidências pelas regras determinísticas (domain::evaluation::rules);
 9. persiste a avaliação em `avaliacoes_saude`;
10. gerencia os alertas operacionais (application::alert_management);
11. devolve o relatório — inclusive a solicitação de orientação que o
    MOTOR (infrastructure::sensors::motor) pode spawnar para a IA copiloto.

A IA nunca participa deste fluxo: o nível de risco é decidido EXCLUSIVAMENTE
pelas regras determinísticas e persistido antes de qualquer chamada à IA.
Nenhum limiar aqui é clínico: tudo chega pronto via ProtocoloAvaliacao.
*/

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};

use crate::analysis::baseline::{
    BaselineConfig, BaselineErro, baseline_desatualizado, calcular_baseline_de_repositorio,
};
use crate::analysis::data_quality::{DataQualityResult, JanelaTempo};
use crate::analysis::environmental_exposure::{
    EnvironmentalThresholds, calculate_environmental_exposure_de_repositorio,
};
use crate::analysis::persistence::{CriterioAlteracao, evaluate_persistence, janelas_da_serie};
use crate::analysis::repository::{FaseMissao, Indicador};
use crate::analysis::trend::{TrendConfig, TrendDirection, analyze_trend_com_configuracao};
use crate::analysis::variation::{ContextoAtual, VariacaoErro, VariationConfig, calcular_variacao};
use crate::application::alert_management::create_alert::{
    AcaoAlerta, GerenciadorAlertas, ResultadoAlerta,
};
use crate::application::evaluation::build_context::{
    ConstrutorContexto, ContextoAvaliacao, config_qualidade,
};
use crate::domain::entities::evaluation_entity::AvaliacaoSaude;
use crate::domain::entities::measurement_entity::BaselineFisiologico;
use crate::domain::entities::protocol_entity::ProtocoloAvaliacao;
use crate::domain::evaluation::rules::{
    EstadoIndicador, EvidenciaIndicador, ExposicaoResumo, FatoresContextuais,
    RegrasAvaliacaoConfig, RiskLevel, avaliar,
};
use crate::domain::repositories::alert_repository::AlertRepository;
use crate::domain::repositories::evaluation_repository::EvaluationRepository;
use crate::domain::repositories::measurement_repository::MeasurementRepository;
use crate::infrastructure::ai::models::SolicitacaoOrientacao;
use crate::shared::errors::{AppErro, AppResultado};

pub struct ProcessamentoAvaliacao {
    measurements: Arc<dyn MeasurementRepository>,
    evaluations: Arc<dyn EvaluationRepository>,
    alertas: Arc<dyn AlertRepository>,
    protocolo: ProtocoloAvaliacao,
    regras: RegrasAvaliacaoConfig,
    janela_analise: chrono::Duration,
    limite_anteriores: i64,
    fase_missao: FaseMissao,
}

/// Relatório completo de uma execução da avaliação.
#[derive(Debug)]
pub struct RelatorioAvaliacao {
    pub avaliacao: AvaliacaoSaude,
    pub nivel: RiskLevel,
    pub inconclusivo: bool,
    pub evidencias: Vec<EvidenciaIndicador>,
    pub fatores: FatoresContextuais,
    pub justificativas: Vec<String>,
    pub resultado_alerta: ResultadoAlerta,
    /// Avisos de falhas parciais (nunca silenciados).
    pub avisos: Vec<String>,
    pub baselines_criados: usize,
    pub baselines_atualizados: usize,
    /// Solicitação para a orientação da IA copiloto. O MOTOR decide quando
    /// (ou se) spawnar a tarefa — a avaliação em si não chama a IA.
    pub orientacao: Option<SolicitacaoOrientacao>,
}

impl RelatorioAvaliacao {
    /// Id do alerta criado/atualizado nesta execução, quando houver.
    pub fn alerta_id(&self) -> Option<String> {
        match &self.resultado_alerta.acao {
            AcaoAlerta::Criado(a) | AcaoAlerta::Atualizado(a) => Some(a.id.clone()),
            _ => None,
        }
    }
}

/// Origem do baseline devolvido por `garantir_baseline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrigemBaseline {
    Mantido,
    Criado,
    Atualizado,
}

impl ProcessamentoAvaliacao {
    #[allow(clippy::too_many_arguments)]
    pub fn novo(
        measurements: Arc<dyn MeasurementRepository>,
        evaluations: Arc<dyn EvaluationRepository>,
        alertas: Arc<dyn AlertRepository>,
        protocolo: ProtocoloAvaliacao,
        janela_analise_segundos: i64,
        limite_anteriores: i64,
        fase_missao: FaseMissao,
    ) -> AppResultado<Self> {
        if janela_analise_segundos <= 0 {
            return Err(AppErro::Configuracao(
                "janela de análise deve ser > 0".to_string(),
            ));
        }
        protocolo.validar().map_err(AppErro::Configuracao)?;

        let regras = RegrasAvaliacaoConfig {
            elevar_com_persistencia: protocolo.elevar_com_persistencia,
            elevar_com_tendencia_adversa: protocolo.elevar_com_tendencia_adversa,
            elevar_com_exposicao_ambiental: protocolo.elevar_com_exposicao_ambiental,
            elevar_com_multiplos_indicadores: protocolo.elevar_com_multiplos_indicadores,
            min_indicadores_multiplos: protocolo.min_indicadores_multiplos,
        };
        regras.validar().map_err(AppErro::Configuracao)?;

        Ok(Self {
            measurements,
            evaluations,
            alertas,
            protocolo,
            regras,
            janela_analise: chrono::Duration::seconds(janela_analise_segundos),
            limite_anteriores,
            fase_missao,
        })
    }

    /// Executa o ciclo completo da avaliação para o momento `agora`.
    pub async fn executar(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        agora: DateTime<Utc>,
    ) -> AppResultado<RelatorioAvaliacao> {
        // ===== 1–2. Contexto =====
        let construtor = ConstrutorContexto::novo(
            self.measurements.clone(),
            self.evaluations.clone(),
            self.janela_analise,
            self.limite_anteriores,
            self.protocolo.qualidade.clone(),
        );
        let contexto = construtor
            .executar(astronauta_id, missao_id, self.fase_missao, agora)
            .await?;

        // ===== 3. Baselines: garantir/atualizar =====
        let mut baselines = contexto.baselines.clone();
        let mut avisos = contexto.avisos.clone();
        let mut criados = 0usize;
        let mut atualizados = 0usize;

        for indicador in Indicador::TODOS {
            match self.garantir_baseline(&contexto, indicador, agora).await {
                Ok(Some((baseline, origem))) => {
                    match origem {
                        OrigemBaseline::Mantido => {}
                        OrigemBaseline::Criado => criados += 1,
                        OrigemBaseline::Atualizado => atualizados += 1,
                    }
                    baselines.insert(indicador, baseline);
                }
                // Sem dados suficientes para o baseline NÃO é falha do
                // sistema: fica registrado e a variação fica insuficiente
                // (nunca eleva risco). O baseline existente é mantido.
                Ok(None) => {}
                Err(erro) => {
                    avisos.push(format!("baseline {}: {erro}", indicador.nome()));
                    // Baseline inservível → sai do mapa (a comparação
                    // não pode ser feita com uma referência duvidosa).
                    baselines.remove(&indicador);
                }
            }
        }

        // ===== 4–6. Variação, tendência e persistência POR INDICADOR =====
        let mut evidencias = Vec::new();
        for indicador in Indicador::TODOS {
            let qualidade = contexto
                .qualidade
                .get(&indicador)
                .cloned()
                .unwrap_or_else(qualidade_vazia);
            evidencias
                .push(self.avaliar_indicador(&contexto, indicador, &qualidade, &baselines, agora));
        }

        // ===== 7. Exposição ambiental =====
        let exposicao = self.exposicao_ambiental(astronauta_id).await;

        // ===== 8. Fatores contextuais + regras determinísticas =====
        let mut insuficientes = Vec::new();
        let mut indisponiveis = Vec::new();
        let mut invalidos = Vec::new();
        for e in &evidencias {
            match e.estado {
                EstadoIndicador::Analisado => {}
                EstadoIndicador::DadosInsuficientes => insuficientes.push(e.indicador.clone()),
                EstadoIndicador::Indisponivel => indisponiveis.push(e.indicador.clone()),
                EstadoIndicador::Invalido => invalidos.push(e.indicador.clone()),
            }
        }
        let fatores = FatoresContextuais {
            janela_analise_segundos: self.janela_analise.num_seconds(),
            exposicao_ambiental: exposicao,
            indicadores_insuficientes: insuficientes,
            indicadores_indisponiveis: indisponiveis,
            indicadores_invalidos: invalidos,
            avaliacoes_anteriores_consideradas: contexto.avaliacoes_anteriores.len(),
            observacoes: avisos.clone(),
        };

        let resultado = avaliar(&evidencias, &fatores, &self.regras);

        // ===== 9. Persistência da avaliação =====
        let resumo = resultado
            .justificativas
            .first()
            .cloned()
            .unwrap_or_else(|| "Avaliação concluída.".to_string());
        let avaliacao = AvaliacaoSaude {
            id: uuid::Uuid::new_v4().to_string(),
            astronauta_id: astronauta_id.to_string(),
            missao_id: missao_id.map(str::to_string),
            timestamp_utc: agora.to_rfc3339_opts(SecondsFormat::Secs, true),
            nivel_risco: resultado.nivel.nome().to_string(),
            resumo,
            indicadores_analisados_json: serde_json::to_string(&evidencias)
                .unwrap_or_else(|_| "[]".to_string()),
            fatores_contextuais_json: serde_json::to_string(&fatores).ok(),
            explicacao: None,
            orientacao: None,
            updated_at: None,
            deleted_at: None,
        };
        self.evaluations
            .salvar(&avaliacao)
            .await
            .map_err(AppErro::Repositorio)?;

        // ===== 10. Gerenciamento de alertas =====
        let resultado_alerta = GerenciadorAlertas::novo(self.alertas.clone())
            .executar(&avaliacao, &evidencias, &self.protocolo, agora)
            .await?;

        // ===== 11. Solicitação de orientação (IA copiloto, opcional) =====
        let orientacao = Some(SolicitacaoOrientacao {
            astronauta_id: avaliacao.astronauta_id.clone(),
            avaliacao_id: avaliacao.id.clone(),
            timestamp_utc: avaliacao.timestamp_utc.clone(),
            nivel_risco: avaliacao.nivel_risco.clone(),
            inconclusivo: resultado.inconclusivo,
            resumo_deterministico: avaliacao.resumo.clone(),
            justificativas: resultado.justificativas.clone(),
            evidencias: evidencias.clone(),
            fatores_contextuais: Some(fatores.clone()),
        });

        Ok(RelatorioAvaliacao {
            avaliacao,
            nivel: resultado.nivel,
            inconclusivo: resultado.inconclusivo,
            evidencias,
            fatores,
            justificativas: resultado.justificativas,
            resultado_alerta,
            avisos,
            baselines_criados: criados,
            baselines_atualizados: atualizados,
            orientacao,
        })
    }

    /// Garante um baseline individual para o indicador:
    /// - ausente → calcula da janela de cálculo e insere;
    /// - presente e mais antigo que a política → recalcula e atualiza;
    /// - presente e atual (ou política desativada) → mantém.
    ///
    /// `Ok(None)` = não havia dados suficientes para calcular o baseline
    /// (não é falha do sistema; a avaliação segue sem essa comparação).
    async fn garantir_baseline(
        &self,
        contexto: &ContextoAvaliacao,
        indicador: Indicador,
        agora: DateTime<Utc>,
    ) -> AppResultado<Option<(BaselineFisiologico, OrigemBaseline)>> {
        let existente = contexto.baselines.get(&indicador);

        // Política de atualização por idade.
        let precisa_atualizar = match (existente, self.protocolo.baseline.idade_maxima_segundos) {
            (Some(b), Some(max_segundos)) => {
                baseline_desatualizado(b, agora, chrono::Duration::seconds(max_segundos))
            }
            (Some(_), None) => false,
            (None, _) => true, // criar do zero
        };

        if let Some(b) = existente
            && !precisa_atualizar
        {
            return Ok(Some((b.clone(), OrigemBaseline::Mantido)));
        }

        // Janela de cálculo do baseline (distinta da janela de análise).
        let janela = JanelaTempo {
            inicio: agora
                - chrono::Duration::seconds(self.protocolo.baseline.janela_calculo_segundos),
            fim: agora,
        };
        let config = BaselineConfig {
            k_mad: self.protocolo.baseline.k_mad,
            minimum_samples_after_filter: self.protocolo.baseline.minimo_amostras,
        };
        let qualidade = config_qualidade(indicador, &self.protocolo.qualidade);

        let calculado = calcular_baseline_de_repositorio(
            self.measurements.as_ref(),
            &contexto.astronauta_id,
            contexto.missao_id.as_deref(),
            indicador,
            self.fase_missao,
            janela,
            &qualidade,
            &config,
            agora,
        )
        .await;

        let novo = match calculado {
            Ok(b) => b,
            Err(BaselineErro::Repositorio(e)) => return Err(AppErro::Repositorio(e)),
            Err(BaselineErro::ConfigInvalida(m)) => {
                return Err(AppErro::Configuracao(format!(
                    "configuração do baseline inválida: {m}"
                )));
            }
            // Sem amostras, amostras insuficientes ou qualidade inaceitável:
            // o baseline apenas ainda não existe.
            Err(_) => return Ok(None),
        };

        match existente {
            Some(b) if precisa_atualizar => {
                // Mesma linha (chave única do banco): id preservado.
                let mut atualizado = novo;
                atualizado.id = b.id.clone();
                atualizado.deleted_at = None;
                self.measurements
                    .atualizar_baseline(&atualizado)
                    .await
                    .map_err(AppErro::Repositorio)?;
                Ok(Some((atualizado, OrigemBaseline::Atualizado)))
            }
            _ => {
                self.measurements
                    .salvar_baseline(&novo)
                    .await
                    .map_err(AppErro::Repositorio)?;
                Ok(Some((novo, OrigemBaseline::Criado)))
            }
        }
    }

    /// Constrói a evidência de UM indicador combinando qualidade, baseline,
    /// variação, tendência e persistência. Dados insuficientes/inválidos/
    /// ausentes NUNCA produzem nível elevado.
    fn avaliar_indicador(
        &self,
        contexto: &ContextoAvaliacao,
        indicador: Indicador,
        qualidade: &DataQualityResult,
        baselines: &HashMap<Indicador, BaselineFisiologico>,
        agora: DateTime<Utc>,
    ) -> EvidenciaIndicador {
        let invalidas = qualidade.invalid_timestamps
            + qualidade.duplicated_timestamps
            + qualidade.out_of_range_samples
            + qualidade.noisy_samples;

        let mut evidencia = EvidenciaIndicador {
            indicador: indicador.nome().to_string(),
            estado: EstadoIndicador::Indisponivel,
            nivel_variacao: None,
            z_score: None,
            valor_atual: None,
            valor_referencia: None,
            direcao: None,
            tendencia: None,
            persistente: None,
            baseline_desatualizado: None,
            qualidade_aceitavel: qualidade.acceptable_quality,
            amostras_validas: qualidade.valid_samples,
            amostras_ausentes: qualidade.missing_samples,
            amostras_invalidas: invalidas,
            justificativa: String::new(),
        };

        // ---- Ausência ≠ resultado normal ----
        if qualidade.total_samples == 0 {
            evidencia.justificativa = "nenhuma medição na janela de análise".to_string();
            return evidencia;
        }

        // ---- Tudo rejeitado pela validação ----
        if qualidade.valid_samples == 0 {
            evidencia.estado = EstadoIndicador::Invalido;
            evidencia.justificativa = format!("{invalidas} amostras rejeitadas pela validação");
            return evidencia;
        }

        // ---- Qualidade inaceitável → insuficiente (não eleva risco) ----
        if !qualidade.acceptable_quality {
            evidencia.estado = EstadoIndicador::DadosInsuficientes;
            evidencia.justificativa = qualidade
                .motivos
                .iter()
                .map(|m| format!("{m:?}"))
                .collect::<Vec<_>>()
                .join("; ");
            return evidencia;
        }

        let Some(baseline) = baselines.get(&indicador) else {
            evidencia.estado = EstadoIndicador::DadosInsuficientes;
            evidencia.justificativa =
                "sem baseline individual comparável para este indicador".to_string();
            return evidencia;
        };

        let Some(amostras) = contexto.amostras(indicador).filter(|a| !a.is_empty()) else {
            evidencia.estado = EstadoIndicador::DadosInsuficientes;
            evidencia.justificativa =
                "amostras válidas indisponíveis para a comparação".to_string();
            return evidencia;
        };

        // ---- Variação individual (pura, sobre amostras já validadas) ----
        let config = VariationConfig {
            z_atencao: self.protocolo.variacao.z_atencao,
            z_critico: self.protocolo.variacao.z_critico,
            max_idade_baseline: self
                .protocolo
                .baseline
                .idade_maxima_segundos
                .map(chrono::Duration::seconds),
        };
        let variacao = match calcular_variacao(
            ContextoAtual {
                astronauta_id: &contexto.astronauta_id,
                missao_id: contexto.missao_id.as_deref(),
                fase_missao: Some(self.fase_missao.nome()),
                agora,
            },
            indicador,
            baseline,
            amostras,
            &config,
        ) {
            Ok(v) => v,
            Err(VariacaoErro::ConfigInvalida(m)) => {
                evidencia.estado = EstadoIndicador::DadosInsuficientes;
                evidencia.justificativa = format!("configuração inválida: {m}");
                return evidencia;
            }
            Err(erro) => {
                // Identidade divergente/sem amostras: comparação não
                // utilizável → insuficiente, com motivo registrado.
                evidencia.estado = EstadoIndicador::DadosInsuficientes;
                evidencia.justificativa = format!("comparação não utilizável: {erro:?}");
                return evidencia;
            }
        };

        evidencia.estado = EstadoIndicador::Analisado;
        evidencia.nivel_variacao = Some(
            match variacao.nivel {
                crate::analysis::variation::NivelVariacao::Normal => "Normal",
                crate::analysis::variation::NivelVariacao::Atencao => "Atencao",
                crate::analysis::variation::NivelVariacao::Critico => "Critico",
                crate::analysis::variation::NivelVariacao::Indeterminado => "Indeterminado",
            }
            .to_string(),
        );
        evidencia.z_score = variacao.z_score;
        evidencia.valor_atual = Some(variacao.valor_atual);
        evidencia.valor_referencia = Some(variacao.valor_referencia);
        evidencia.direcao = Some(
            match variacao.direcao {
                crate::analysis::variation::Direcao::Acima => "Acima",
                crate::analysis::variation::Direcao::Abaixo => "Abaixo",
                crate::analysis::variation::Direcao::Estavel => "Estavel",
            }
            .to_string(),
        );
        evidencia.baseline_desatualizado = variacao.baseline_desatualizado;

        // ---- Tendência (pura) ----
        evidencia.tendencia = self.tendencia_do_indicador(baseline, amostras);

        // ---- Persistência (pura; exige desvio-padrão no baseline) ----
        evidencia.persistente = self.persistencia_do_indicador(baseline, amostras);

        evidencia.justificativa = format!(
            "média atual {:.1} vs referência individual {:.1} ({} amostras)",
            variacao.valor_atual, variacao.valor_referencia, variacao.quantidade_amostras
        );

        evidencia
    }

    /// Tendência da série, quando ela cobre o período e a quantidade
    /// mínimas configuradas. Limiar de inclinação derivado do desvio-
    /// padrão individual (fallback: inclinação mínima padrão).
    fn tendencia_do_indicador(
        &self,
        baseline: &BaselineFisiologico,
        amostras: &[(DateTime<Utc>, f64)],
    ) -> Option<String> {
        let limiar = match baseline.desvio_padrao {
            Some(dp) if dp.is_finite() && dp > 0.0 => dp,
            _ => self.protocolo.tendencia.inclinacao_minima_padrao,
        };
        let config = TrendConfig {
            minimum_samples: self.protocolo.tendencia.minimo_amostras,
            minimum_slope: limiar,
            minimum_period_seconds: self.protocolo.tendencia.periodo_minimo_segundos,
        };
        analyze_trend_com_configuracao(amostras, &config).map(|t| match t.direction {
            TrendDirection::Increasing => "Aumentando".to_string(),
            TrendDirection::Decreasing => "Diminuindo".to_string(),
            TrendDirection::Stable => "Estavel".to_string(),
        })
    }

    /// Persistência da alteração: janelas com TODO valor fora da faixa
    /// individual (referência ± z_atencao·σ). Sem desvio-padrão no baseline
    /// a persistência NÃO é avaliada (nunca inventa limiar).
    fn persistencia_do_indicador(
        &self,
        baseline: &BaselineFisiologico,
        amostras: &[(DateTime<Utc>, f64)],
    ) -> Option<bool> {
        let sigma = baseline.desvio_padrao?;
        if !sigma.is_finite() || sigma <= 0.0 {
            return None;
        }
        let criterio = CriterioAlteracao::ForaDaFaixa {
            minimo: baseline.valor_referencia - self.protocolo.variacao.z_atencao * sigma,
            maximo: baseline.valor_referencia + self.protocolo.variacao.z_atencao * sigma,
        };
        let janelas = janelas_da_serie(
            amostras,
            &criterio,
            self.protocolo.persistencia.janela_segundos,
        );
        evaluate_persistence(
            &janelas,
            self.protocolo.persistencia.minimo_percentual,
            self.protocolo.persistencia.minimo_janelas_consecutivas,
        )
        .map(|r| r.is_persistent)
    }

    /// Exposição ambiental estimada do(s) módulo(s) visitado(s).
    /// Falha é devolvida como `None` — a avaliação continua sem esse fator
    /// (a indisponibilidade fica registrada como ausência de exposição).
    async fn exposicao_ambiental(&self, astronauta_id: &str) -> Option<ExposicaoResumo> {
        let limiares = EnvironmentalThresholds {
            co2_ppm: self.protocolo.exposicao.co2_ppm,
            temperature_c: self.protocolo.exposicao.temperatura_c,
            pressure_kpa_min: self.protocolo.exposicao.pressao_kpa_min,
            pressure_kpa_max: self.protocolo.exposicao.pressao_kpa_max,
        };
        limiares.validar().ok()?;

        let resultado = calculate_environmental_exposure_de_repositorio(
            self.measurements.as_ref(),
            astronauta_id,
            None, // inicio_utc
            None, // fim_utc
            &limiares,
            self.protocolo.exposicao.lacuna_maxima_segundos,
            Utc::now(),
        )
        .await
        .ok()?;

        let minimo_evidencia = self.protocolo.exposicao.tempo_minimo_evidencia_segundos;
        let limites_excedidos = resultado.co2_time_above_seconds >= minimo_evidencia
            || resultado.temperature_time_above_seconds >= minimo_evidencia
            || resultado.pressure_time_outside_range_seconds >= minimo_evidencia;

        Some(ExposicaoResumo {
            co2_exposicao_ppm_hora: resultado.co2_exposure_ppm_hours,
            radiacao_dose_msv: resultado.radiation_dose_msv,
            temperatura_exposicao_graus_hora: resultado.temperature_exposure_degree_hours,
            pressao_exposicao_kpa_hora: resultado.pressure_exposure_kpa_hours,
            co2_tempo_acima_segundos: resultado.co2_time_above_seconds,
            temperatura_tempo_acima_segundos: resultado.temperature_time_above_seconds,
            pressao_tempo_fora_segundos: resultado.pressure_time_outside_range_seconds,
            tempo_analisado_segundos: resultado.analyzed_seconds,
            limites_excedidos,
        })
    }
}

/// Resultado de qualidade "sem nada" (indicador ausente do contexto).
fn qualidade_vazia() -> DataQualityResult {
    DataQualityResult {
        total_samples: 0,
        valid_samples: 0,
        missing_samples: 0,
        invalid_timestamps: 0,
        duplicated_timestamps: 0,
        out_of_window: 0,
        out_of_range_samples: 0,
        noisy_samples: 0,
        temporal_gaps: 0,
        largest_gap_seconds: 0,
        valid_percentage: 0.0,
        motivos: Vec::new(),
        acceptable_quality: false,
        amostras_validas: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::domain::entities::protocol_entity::{
        ParametrosAlerta, ParametrosBaseline, ParametrosExposicao, ParametrosPersistencia,
        ParametrosQualidade, ParametrosTendencia, ParametrosVariacao,
    };
    use crate::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;
    use crate::infrastructure::database::repositories::evaluation_repository::SqliteEvaluationRepository;
    use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

    // 1_700_000_000 = 2023-11-14T22:13:20Z.
    const EPOCA: i64 = 1_700_000_000;

    fn ts(segundos: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(EPOCA + segundos, 0).unwrap()
    }

    fn ts_iso(segundos: i64) -> String {
        ts(segundos).to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    fn protocolo() -> ProtocoloAvaliacao {
        let p = ProtocoloAvaliacao {
            id: "P".into(),
            nome: "P".into(),
            versao: 1,
            baseline: ParametrosBaseline {
                k_mad: 3.0,
                minimo_amostras: 2,
                janela_calculo_segundos: 36_000,
                idade_maxima_segundos: Some(21_600),
            },
            variacao: ParametrosVariacao {
                z_atencao: 2.0,
                z_critico: 3.0,
            },
            qualidade: ParametrosQualidade {
                minimo_amostras: 2,
                minimo_percentual_valido: 80.0,
                maximo_percentual_invalido: 5.0,
                maximo_percentual_ruido: 5.0,
                lacuna_maxima_segundos: 300,
            },
            tendencia: ParametrosTendencia {
                minimo_amostras: 3,
                periodo_minimo_segundos: 0,
                inclinacao_minima_padrao: 0.5,
            },
            persistencia: ParametrosPersistencia {
                janela_segundos: 10,
                minimo_percentual: 50.0,
                minimo_janelas_consecutivas: 2,
            },
            exposicao: ParametrosExposicao {
                co2_ppm: 1000.0,
                temperatura_c: 24.0,
                pressao_kpa_min: 95.0,
                pressao_kpa_max: 105.0,
                lacuna_maxima_segundos: 300,
                tempo_minimo_evidencia_segundos: 60.0,
            },
            alerta: ParametrosAlerta {
                resolucao_automatica: true,
            },
            elevar_com_persistencia: true,
            elevar_com_tendencia_adversa: true,
            elevar_com_exposicao_ambiental: true,
            elevar_com_multiplos_indicadores: true,
            min_indicadores_multiplos: 2,
        };
        p.validar().expect("protocolo de teste válido");
        p
    }

    async fn setup() -> (
        Arc<dyn MeasurementRepository>,
        Arc<dyn EvaluationRepository>,
        Arc<dyn AlertRepository>,
        SqlitePool,
    ) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("conectar");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        sqlx::query(
            "INSERT INTO astronautas (id, nome, data_nascimento) VALUES ('A1', 'K', '1990-05-01')",
        )
        .execute(&pool)
        .await
        .unwrap();

        (
            Arc::new(SqliteMeasurementRepository::new(pool.clone())),
            Arc::new(SqliteEvaluationRepository::new(pool.clone())),
            Arc::new(SqliteAlertRepository::new(pool.clone())),
            pool,
        )
    }

    /// Baseline individual pré-voo de SpO₂ (referência 98, σ 1),
    /// atualizado 1 min antes do `agora` padrão (ts(60)).
    async fn semear_baseline(pool: &SqlitePool) {
        sqlx::query(
            "INSERT INTO baseline_fisiologico
                (id, astronauta_id, indicador, fase_missao, valor_referencia,
                 desvio_padrao, quantidade_amostras, atualizado_em)
             VALUES ('B1', 'A1', 'SPO2', 'PRE_VOO', 98.0, 1.0, 50, ?)",
        )
        .bind(ts_iso(0))
        .execute(pool)
        .await
        .unwrap();
    }

    /// Telemetria com SpO₂ em 91 (z = -7 → Crítico) em duas janelas de
    /// persistência de 10 s (25–40 s depois do baseline).
    async fn semear_spo2_baixo(pool: &SqlitePool) {
        sqlx::query(
            "INSERT INTO telemetria_fisiologica
                (id, astronauta_id, timestamp_utc, spo2_percentual)
             VALUES
                ('T1', 'A1', ?, 91.0),
                ('T2', 'A1', ?, 91.0),
                ('T3', 'A1', ?, 91.0),
                ('T4', 'A1', ?, 91.0)",
        )
        .bind(ts_iso(25))
        .bind(ts_iso(30))
        .bind(ts_iso(35))
        .bind(ts_iso(40))
        .execute(pool)
        .await
        .unwrap();
    }

    fn processamento(
        measurements: Arc<dyn MeasurementRepository>,
        evaluations: Arc<dyn EvaluationRepository>,
        alertas: Arc<dyn AlertRepository>,
    ) -> ProcessamentoAvaliacao {
        ProcessamentoAvaliacao::novo(
            measurements,
            evaluations,
            alertas,
            protocolo(),
            300, // janela de análise: cobre as amostras semeadas
            5,
            FaseMissao::PreVoo,
        )
        .expect("processamento válido")
    }

    #[tokio::test]
    async fn fluxo_completo_classifica_e_persiste_alerta() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        semear_baseline(&pool).await;
        semear_spo2_baixo(&pool).await;
        let caso = processamento(measurements, evaluations, alertas);

        let relatorio = caso.executar("A1", None, ts(60)).await.expect("avaliação");

        // Base Crítico (|z| = 7 > 3) + persistência (+1) = CRITICO.
        assert_eq!(relatorio.nivel, RiskLevel::Critico);
        assert!(!relatorio.inconclusivo);
        assert_eq!(relatorio.avaliacao.nivel_risco, "CRITICO");

        // SpO₂ analisado: variação crítica e persistente.
        let spo2 = relatorio
            .evidencias
            .iter()
            .find(|e| e.indicador == "SPO2")
            .expect("evidência de SpO₂");
        assert_eq!(spo2.estado, EstadoIndicador::Analisado);
        assert_eq!(spo2.nivel_variacao.as_deref(), Some("Critico"));
        assert_eq!(spo2.persistente, Some(true));
        assert_eq!(spo2.direcao.as_deref(), Some("Abaixo"));

        // Alerta criado com a condição do indicador alterado.
        assert!(matches!(
            relatorio.resultado_alerta.acao,
            AcaoAlerta::Criado(_)
        ));
        assert_eq!(
            relatorio.resultado_alerta.condicao.as_deref(),
            Some("SPO2:Abaixo")
        );

        // Avaliação gravada com as evidências serializadas.
        let gravada: (String, String) =
            sqlx::query_as("SELECT nivel_risco, indicadores_analisados_json FROM avaliacoes_saude")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(gravada.0, "CRITICO");
        assert!(gravada.1.contains("SPO2"));
    }

    #[tokio::test]
    async fn deduplicacao_de_alerta_em_duas_execucoes() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        semear_baseline(&pool).await;
        semear_spo2_baixo(&pool).await;
        let caso = processamento(measurements, evaluations, alertas);

        let primeiro = caso.executar("A1", None, ts(60)).await.unwrap();
        let alerta1 = primeiro.alerta_id().expect("alerta criado");

        // Segunda execução, 1 min depois: mesma condição → mesmo alerta.
        let segundo = caso.executar("A1", None, ts(120)).await.unwrap();
        assert_eq!(
            segundo.alerta_id().as_deref(),
            Some(alerta1.as_str()),
            "condição persistente evolui o MESMO alerta"
        );
        assert!(matches!(
            segundo.resultado_alerta.acao,
            AcaoAlerta::Atualizado(_)
        ));

        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'ATIVO'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        // Duas avaliações persistidas (histórico completo).
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM avaliacoes_saude")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn sem_dados_produz_avaliacao_inconclusiva_sem_alerta() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        let caso = processamento(measurements, evaluations, alertas);

        let relatorio = caso.executar("A1", None, ts(60)).await.unwrap();

        assert_eq!(relatorio.nivel, RiskLevel::Normal);
        assert!(
            relatorio.inconclusivo,
            "ausência de dados nunca é resultado normal"
        );
        assert!(relatorio.avisos.is_empty());
        assert!(matches!(
            relatorio.resultado_alerta.acao,
            AcaoAlerta::Resolvidos(0) | AcaoAlerta::Nenhum
        ));
        // A avaliação é persistida mesmo inconclusiva (histórico completo).
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM avaliacoes_saude WHERE astronauta_id = 'A1'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn dados_invalidos_nao_geram_baseline_nem_elevam_o_risco() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        // Apenas UM valor de HRV fora da plausibilidade (-5.0):
        // sem baseline possível e sem amostras válidas.
        sqlx::query(
            "INSERT INTO telemetria_fisiologica (id, astronauta_id, timestamp_utc, hrv_ms)
             VALUES ('TX', 'A1', ?, -5.0)",
        )
        .bind(ts_iso(30))
        .execute(&pool)
        .await
        .unwrap();
        let caso = processamento(measurements, evaluations, alertas);

        let relatorio = caso.executar("A1", None, ts(60)).await.unwrap();

        let hrv = relatorio
            .evidencias
            .iter()
            .find(|e| e.indicador == "HRV")
            .unwrap();
        assert_eq!(hrv.estado, EstadoIndicador::Invalido);
        assert_eq!(
            relatorio.baselines_criados, 0,
            "dados ruins não geram baseline"
        );
        assert_eq!(relatorio.nivel, RiskLevel::Normal);
        assert!(relatorio.inconclusivo);
        assert!(matches!(
            relatorio.resultado_alerta.acao,
            AcaoAlerta::Resolvidos(0) | AcaoAlerta::Nenhum
        ));
    }

    #[tokio::test]
    async fn baseline_e_criado_na_primeira_execucao_com_dados() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        semear_spo2_baixo(&pool).await;
        let caso = processamento(measurements, evaluations, alertas);

        let relatorio = caso.executar("A1", None, ts(60)).await.unwrap();
        assert_eq!(relatorio.baselines_criados, 1, "SpO₂ ganha baseline");
        assert_eq!(relatorio.baselines_atualizados, 0);

        let (referencia, desvio): (f64, Option<f64>) =
            sqlx::query_as("SELECT valor_referencia, desvio_padrao FROM baseline_fisiologico")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!((referencia - 91.0).abs() < 1e-6);

        // Com baseline recém-criado (ref 91, σ 0) a variação fica
        // INDETERMINADA (σ = 0) — gravidade 0, nunca eleva o risco.
        let spo2 = relatorio
            .evidencias
            .iter()
            .find(|e| e.indicador == "SPO2")
            .unwrap();
        assert_eq!(spo2.nivel_variacao.as_deref(), Some("Indeterminado"));
        assert_eq!(desvio.map(|d| d.abs() < 1e-9), Some(true));
    }

    #[tokio::test]
    async fn baseline_desatualizado_e_recalculado_pela_politica_de_idade() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        semear_baseline(&pool).await;
        semear_spo2_baixo(&pool).await;
        let caso = processamento(measurements, evaluations, alertas);

        // Baseline semeado 1 min antes do `agora`; a política (6 h)
        // considera atual → nenhuma atualização.
        let primeiro = caso.executar("A1", None, ts(60)).await.unwrap();
        assert_eq!(primeiro.baselines_atualizados, 0);

        // Execução 7 h depois → baseline desatualizado → recalculado.
        let depois = caso
            .executar("A1", None, ts(60) + chrono::Duration::hours(7))
            .await
            .unwrap();
        assert_eq!(
            depois.baselines_atualizados, 1,
            "política de idade aplicada"
        );

        let linha: (f64, Option<f64>) = sqlx::query_as(
            "SELECT valor_referencia, desvio_padrao FROM baseline_fisiologico WHERE id = 'B1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            (linha.0 - 91.0).abs() < 1e-6,
            "baseline atualizado com a média das amostras da janela de cálculo ({})",
            linha.0
        );
    }

    #[tokio::test]
    async fn astronauta_invalido_e_rejeitado() {
        let (measurements, evaluations, alertas, _pool) = setup().await;
        let caso = processamento(measurements, evaluations, alertas);
        let erro = caso.executar(" ", None, ts(60)).await.unwrap_err();
        assert!(matches!(erro, AppErro::DadosInvalidos(_)));
    }

    #[tokio::test]
    async fn janela_invalida_e_rejeitada_na_construcao() {
        let (measurements, evaluations, alertas, _pool) = setup().await;
        let resultado = ProcessamentoAvaliacao::novo(
            measurements,
            evaluations,
            alertas,
            protocolo(),
            0,
            5,
            FaseMissao::PreVoo,
        );
        assert!(matches!(resultado, Err(AppErro::Configuracao(_))));
    }

    #[tokio::test]
    async fn orientacao_solicita_o_resultado_deterministico() {
        let (measurements, evaluations, alertas, pool) = setup().await;
        semear_baseline(&pool).await;
        semear_spo2_baixo(&pool).await;
        let caso = processamento(measurements, evaluations, alertas);

        let relatorio = caso.executar("A1", None, ts(60)).await.unwrap();
        let solicitacao = relatorio.orientacao.expect("solicitação gerada");
        assert_eq!(solicitacao.nivel_risco, "CRITICO");
        assert_eq!(solicitacao.avaliacao_id, relatorio.avaliacao.id);
        assert!(!solicitacao.justificativas.is_empty());
        // A orientação da IA não altera o que foi persistido.
        let explicacao: Option<String> =
            sqlx::query_scalar("SELECT explicacao FROM avaliacoes_saude WHERE id = ?")
                .bind(&relatorio.avaliacao.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(explicacao, None, "IA ainda não participou");
    }
}
