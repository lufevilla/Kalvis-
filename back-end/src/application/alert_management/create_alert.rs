/*
CASO DE USO: GERENCIAMENTO DE ALERTAS (DETERMINÍSTICO).

Recebe a avaliação JÁ classificada pelas regras determinísticas e suas
evidências. A geração de alertas depende EXCLUSIVAMENTE dessas regras —
a resposta da IA nunca cria, altera ou resolve alertas.

Política (configurável via protocolo):
- nível NORMAL (ou nenhuma condição justificada) → resolve os alertas
  ativos do astronauta quando `resolucao_automatica`;
- nível >= ATENCAO → cria o alerta da condição ou ATUALIZA o alerta
  ATIVO da mesma condição (deduplicação: um alerta por condição
  persistente, garantido por índice único parcial no banco);
- alertas ativos de condições que deixaram de ser justificadas são
  resolvidos (a criação/atualização e a resolução acontecem de forma
  coerente na mesma execução).
*/

use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};

use crate::domain::entities::alert_entity::{Alerta, EntradaAlerta};
use crate::domain::entities::evaluation_entity::AvaliacaoSaude;
use crate::domain::entities::protocol_entity::ProtocoloAvaliacao;
use crate::domain::evaluation::rules::{EvidenciaIndicador, RiskLevel};
use crate::domain::repositories::alert_repository::AlertRepository;
use crate::shared::errors::{AppErro, AppResultado};

pub struct GerenciadorAlertas {
    repo: Arc<dyn AlertRepository>,
}

/// O que o gerenciamento fez nesta execução.
#[derive(Debug)]
pub enum AcaoAlerta {
    /// Nada a fazer (nível normal e/ou resolução automática desativada).
    Nenhum,
    /// Condição acabou: alertas ativos resolvidos.
    Resolvidos(u64),
    /// Novo alerta criado.
    Criado(Alerta),
    /// Alerta existente da mesma condição atualizado.
    Atualizado(Alerta),
}

#[derive(Debug)]
pub struct ResultadoAlerta {
    pub acao: AcaoAlerta,
    pub condicao: Option<String>,
}

impl GerenciadorAlertas {
    pub fn novo(repo: Arc<dyn AlertRepository>) -> Self {
        Self { repo }
    }

    pub async fn executar(
        &self,
        avaliacao: &AvaliacaoSaude,
        evidencias: &[EvidenciaIndicador],
        protocolo: &ProtocoloAvaliacao,
        agora: DateTime<Utc>,
    ) -> AppResultado<ResultadoAlerta> {
        let nivel = RiskLevel::from_nome(&avaliacao.nivel_risco).ok_or_else(|| {
            AppErro::DadosInvalidos(format!(
                "nível de risco desconhecido na avaliação '{}': {}",
                avaliacao.id, avaliacao.nivel_risco
            ))
        })?;

        let agora_iso = agora.to_rfc3339_opts(SecondsFormat::Secs, true);

        // Chave estável da condição: indicadores alterados + direções.
        // Sem indicador alterado, a exposição ambiental excedida ainda
        // justifica um alerta de ambiente.
        let condicao = chave_condicao(evidencias)
            .or((nivel != RiskLevel::Normal).then(|| "EXPOSICAO_AMBIENTAL".to_string()));

        match (nivel, condicao) {
            (RiskLevel::Normal, _) | (_, None) => {
                if protocolo.alerta.resolucao_automatica {
                    let resolvidos = self
                        .repo
                        .resolver_todos(
                            &avaliacao.astronauta_id,
                            avaliacao.missao_id.as_deref(),
                            &agora_iso,
                        )
                        .await
                        .map_err(AppErro::Repositorio)?;
                    Ok(ResultadoAlerta {
                        acao: AcaoAlerta::Resolvidos(resolvidos),
                        condicao: None,
                    })
                } else {
                    Ok(ResultadoAlerta {
                        acao: AcaoAlerta::Nenhum,
                        condicao: None,
                    })
                }
            }
            (_, Some(condicao)) => {
                let entrada = EntradaAlerta {
                    avaliacao_id: avaliacao.id.clone(),
                    astronauta_id: avaliacao.astronauta_id.clone(),
                    missao_id: avaliacao.missao_id.clone(),
                    condicao: condicao.clone(),
                    timestamp_utc: avaliacao.timestamp_utc.clone(),
                    nivel_risco: avaliacao.nivel_risco.clone(),
                    resumo: avaliacao.resumo.clone(),
                    explicacao: None,
                    orientacao: None,
                    atualizado_em: agora_iso.clone(),
                };

                let (alerta, criado) = self
                    .repo
                    .criar_ou_atualizar_alerta(&entrada)
                    .await
                    .map_err(AppErro::Repositorio)?;

                // Condições que deixaram de ser justificadas → resolvidas.
                if protocolo.alerta.resolucao_automatica {
                    let ativos = self
                        .repo
                        .listar_alertas_ativos(
                            Some(&avaliacao.astronauta_id),
                            avaliacao.missao_id.as_deref(),
                        )
                        .await
                        .map_err(AppErro::Repositorio)?;
                    for ativo in ativos {
                        if ativo.id != alerta.id && ativo.condicao != condicao {
                            self.repo
                                .resolver_alerta(&ativo.id, &agora_iso)
                                .await
                                .map_err(AppErro::Repositorio)?;
                        }
                    }
                }

                Ok(ResultadoAlerta {
                    acao: if criado {
                        AcaoAlerta::Criado(alerta)
                    } else {
                        AcaoAlerta::Atualizado(alerta)
                    },
                    condicao: Some(condicao),
                })
            }
        }
    }
}

/// Chave estável e determinística da condição: indicadores alterados
/// com sua direção, ordenados (a mesma condição produz a mesma chave
/// independentemente da ordem das evidências).
pub fn chave_condicao(evidencias: &[EvidenciaIndicador]) -> Option<String> {
    let mut partes: Vec<String> = evidencias
        .iter()
        .filter(|e| {
            e.estado == crate::domain::evaluation::rules::EstadoIndicador::Analisado
                && matches!(
                    e.nivel_variacao.as_deref(),
                    Some("Atencao") | Some("Critico")
                )
        })
        .map(|e| {
            format!(
                "{}:{}",
                e.indicador,
                e.direcao.clone().unwrap_or_else(|| "Indefinido".into())
            )
        })
        .collect();
    partes.sort();
    partes.dedup();
    (!partes.is_empty()).then(|| partes.join("|"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::domain::entities::evaluation_entity::AvaliacaoSaude;
    use crate::domain::entities::protocol_entity::ParametrosAlerta;
    use crate::domain::evaluation::rules::EstadoIndicador;
    use crate::infrastructure::database::repositories::alert_repository::SqliteAlertRepository;

    fn protocolo(resolucao: bool) -> ProtocoloAvaliacao {
        let p = ProtocoloAvaliacao {
            id: "P".into(),
            nome: "P".into(),
            versao: 1,
            baseline: crate::domain::entities::protocol_entity::ParametrosBaseline {
                k_mad: 3.0,
                minimo_amostras: 20,
                janela_calculo_segundos: 3600,
                idade_maxima_segundos: Some(21_600),
            },
            variacao: crate::domain::entities::protocol_entity::ParametrosVariacao {
                z_atencao: 2.0,
                z_critico: 3.0,
            },
            qualidade: crate::domain::entities::protocol_entity::ParametrosQualidade {
                minimo_amostras: 2,
                minimo_percentual_valido: 80.0,
                maximo_percentual_invalido: 5.0,
                maximo_percentual_ruido: 5.0,
                lacuna_maxima_segundos: 300,
            },
            tendencia: crate::domain::entities::protocol_entity::ParametrosTendencia {
                minimo_amostras: 3,
                periodo_minimo_segundos: 600,
                inclinacao_minima_padrao: 0.5,
            },
            persistencia: crate::domain::entities::protocol_entity::ParametrosPersistencia {
                janela_segundos: 60,
                minimo_percentual: 50.0,
                minimo_janelas_consecutivas: 2,
            },
            exposicao: crate::domain::entities::protocol_entity::ParametrosExposicao {
                co2_ppm: 1000.0,
                temperatura_c: 24.0,
                pressao_kpa_min: 95.0,
                pressao_kpa_max: 105.0,
                lacuna_maxima_segundos: 300,
                tempo_minimo_evidencia_segundos: 60.0,
            },
            alerta: ParametrosAlerta {
                resolucao_automatica: resolucao,
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

    fn agora() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2025-01-15T08:30:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn avaliacao(nivel: &str) -> AvaliacaoSaude {
        AvaliacaoSaude {
            id: "AV1".into(),
            astronauta_id: "A1".into(),
            missao_id: Some("M1".into()),
            timestamp_utc: "2025-01-15T08:00:00Z".into(),
            nivel_risco: nivel.into(),
            resumo: "resumo determinístico".into(),
            indicadores_analisados_json: "[]".into(),
            fatores_contextuais_json: None,
            explicacao: None,
            orientacao: None,
            updated_at: None,
            deleted_at: None,
        }
    }

    fn evid(indicador: &str, nivel: &str, direcao: &str) -> EvidenciaIndicador {
        EvidenciaIndicador {
            indicador: indicador.into(),
            estado: EstadoIndicador::Analisado,
            nivel_variacao: Some(nivel.into()),
            z_score: Some(2.5),
            valor_atual: Some(91.0),
            valor_referencia: Some(98.0),
            direcao: Some(direcao.into()),
            tendencia: Some("Estavel".into()),
            persistente: Some(true),
            baseline_desatualizado: None,
            qualidade_aceitavel: true,
            amostras_validas: 5,
            amostras_ausentes: 0,
            amostras_invalidas: 0,
            justificativa: String::new(),
        }
    }

    async fn gerenciador() -> (GerenciadorAlertas, SqlitePool) {
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
        sqlx::query("INSERT INTO missoes (id, nome_missao) VALUES ('M1', 'Artemis III')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO avaliacoes_saude (id, astronauta_id, missao_id, timestamp_utc, nivel_risco, resumo, indicadores_analisados_json) VALUES ('AV1', 'A1', 'M1', '2025-01-15T08:00:00Z', 'ATENCAO', 'resumo determinístico', '[]')")
            .execute(&pool).await.unwrap();

        (
            GerenciadorAlertas::novo(Arc::new(SqliteAlertRepository::new(pool.clone()))),
            pool,
        )
    }

    #[test]
    fn chave_de_condicao_e_deterministica_e_ordenada() {
        let evids = vec![
            evid("SPO2", "Atencao", "Abaixo"),
            evid("FREQUENCIA_CARDIACA", "Critico", "Acima"),
        ];
        let reverso = {
            let mut v = evids.clone();
            v.reverse();
            v
        };
        assert_eq!(
            chave_condicao(&evids),
            chave_condicao(&reverso),
            "mesma condição → mesma chave"
        );
        assert_eq!(
            chave_condicao(&evids).as_deref(),
            Some("FREQUENCIA_CARDIACA:Acima|SPO2:Abaixo")
        );

        // Sem indicador alterado → sem condição.
        assert_eq!(chave_condicao(&[evid("SPO2", "Normal", "Estavel")]), None);
        assert_eq!(chave_condicao(&[]), None);
    }

    #[tokio::test]
    async fn nivel_normal_resolve_alertas_e_nao_cria_nada() {
        let (gerenciador, pool) = gerenciador().await;
        let p = protocolo(true);

        // Primeiro cria um alerta.
        let r = gerenciador
            .executar(
                &avaliacao("ATENCAO"),
                &[evid("SPO2", "Atencao", "Abaixo")],
                &p,
                agora(),
            )
            .await
            .unwrap();
        assert!(matches!(r.acao, AcaoAlerta::Criado(_)));

        // Avaliação normal → resolução automática.
        let r = gerenciador
            .executar(&avaliacao("NORMAL"), &[], &p, agora())
            .await
            .unwrap();
        assert!(matches!(r.acao, AcaoAlerta::Resolvidos(1)));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'ATIVO'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn alerta_e_deduplicado_por_condicao() {
        let (gerenciador, pool) = gerenciador().await;
        let p = protocolo(true);
        let evids = [evid("SPO2", "Atencao", "Abaixo")];

        let primeiro = gerenciador
            .executar(&avaliacao("ATENCAO"), &evids, &p, agora())
            .await
            .unwrap();
        let alerta1 = match primeiro.acao {
            AcaoAlerta::Criado(a) => a,
            _ => panic!("deveria criar"),
        };

        // Mesma condição persistente → atualiza o MESMO alerta.
        let segundo = gerenciador
            .executar(&avaliacao("ALERTA"), &evids, &p, agora())
            .await
            .unwrap();
        match segundo.acao {
            AcaoAlerta::Atualizado(a) => {
                assert_eq!(a.id, alerta1.id, "mesmo alerta evolui");
                assert_eq!(a.nivel_risco, "ALERTA", "nível evolui no registro");
                assert_eq!(a.timestamp_utc, alerta1.timestamp_utc, "início preservado");
                assert_eq!(a.avaliacao_id, "AV1");
            }
            _ => panic!("deveria atualizar"),
        }

        // Apenas um alerta ativo no banco.
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'ATIVO'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn condicao_que_deixa_de_existir_e_resolvida_e_nova_e_criada() {
        let (gerenciador, pool) = gerenciador().await;
        let p = protocolo(true);

        gerenciador
            .executar(
                &avaliacao("ATENCAO"),
                &[evid("SPO2", "Atencao", "Abaixo")],
                &p,
                agora(),
            )
            .await
            .unwrap();

        // Condição muda: agora é FC acima → a antiga é resolvida.
        let r = gerenciador
            .executar(
                &avaliacao("ATENCAO"),
                &[evid("FREQUENCIA_CARDIACA", "Atencao", "Acima")],
                &p,
                agora(),
            )
            .await
            .unwrap();
        assert!(matches!(r.acao, AcaoAlerta::Criado(_)));

        let ativos =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'ATIVO'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(ativos, 1, "um alerta ativo por condição vigente");

        let resolvidos =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alertas WHERE estado = 'RESOLVIDO'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(resolvidos, 1, "condição antiga resolvida");
    }

    #[tokio::test]
    async fn resolucao_automatica_desativada_nao_resolve() {
        let (gerenciador, _pool) = gerenciador().await;
        let p = protocolo(false);

        gerenciador
            .executar(
                &avaliacao("ATENCAO"),
                &[evid("SPO2", "Atencao", "Abaixo")],
                &p,
                agora(),
            )
            .await
            .unwrap();

        let r = gerenciador
            .executar(&avaliacao("NORMAL"), &[], &p, agora())
            .await
            .unwrap();
        assert!(matches!(r.acao, AcaoAlerta::Nenhum));
    }

    #[tokio::test]
    async fn alerta_de_exposicao_sem_indicador_alterado() {
        let (gerenciador, _pool) = gerenciador().await;
        let p = protocolo(true);

        // Nível elevado SEM indicador alterado (exposição ambiental) →
        // condição de ambiente.
        let r = gerenciador
            .executar(&avaliacao("ATENCAO"), &[], &p, agora())
            .await
            .unwrap();
        assert_eq!(r.condicao.as_deref(), Some("EXPOSICAO_AMBIENTAL"));
        assert!(matches!(r.acao, AcaoAlerta::Criado(_)));
    }

    #[tokio::test]
    async fn nivel_desconhecido_e_erro_explicito() {
        let (gerenciador, _pool) = gerenciador().await;
        let p = protocolo(true);
        let erro = gerenciador
            .executar(&avaliacao("FOO"), &[], &p, agora())
            .await
            .unwrap_err();
        assert!(matches!(erro, AppErro::DadosInvalidos(_)));
    }
}
