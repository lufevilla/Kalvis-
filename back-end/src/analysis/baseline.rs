/*
CALCULA O VALOR ATUAL COMPARANDO COM O PRÉ-VOO

1. BASELINE PRÉ-VOO (MÉDIA + DESVIO PADRÃO DAS AMOSTRAS)
2. COMPARAÇÃO DA TELEMETRIA ATUAL COM O PRÉ-VOO (DIFERENÇA, VARIAÇÃO %, Z-SCORE, DIREÇÃO)
3. COMPARAÇÃO DO SONO DIÁRIO COM O PRÉ-VOO
*/

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::measurement_entity::{
    BaselineFisiologico, RegistroDiario, TelemetriaFisiologica,
};

// ============================================================
// NOMES OFICIAIS DOS INDICADORES (USADOS NO BANCO)
// ============================================================

pub const INDICADOR_FREQUENCIA_CARDIACA: &str = "FREQUENCIA_CARDIACA";
pub const INDICADOR_SPO2: &str = "SPO2";
pub const INDICADOR_TEMPERATURA_CORPORAL: &str = "TEMPERATURA_CORPORAL";
pub const INDICADOR_TEMPERATURA_PELE: &str = "TEMPERATURA_PELE";
pub const INDICADOR_TAXA_RESPIRATORIA: &str = "TAXA_RESPIRATORIA";
pub const INDICADOR_HRV: &str = "HRV";
pub const INDICADOR_PRESSAO_SISTOLICA: &str = "PRESSAO_SISTOLICA";
pub const INDICADOR_PRESSAO_DIASTOLICA: &str = "PRESSAO_DIASTOLICA";
pub const INDICADOR_GLICOSE: &str = "GLICOSE";
pub const INDICADOR_HORAS_SONO: &str = "HORAS_SONO";

pub const FASE_PRE_VOO: &str = "PRE_VOO";

// ============================================================
// ESTRUTURAS DE SAÍDA
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DesvioDirecao {
    Acima,
    Abaixo,
    Estavel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparacaoPreVoo {
    pub indicador: String,
    pub valor_atual: f64,
    pub valor_referencia_pre_voo: f64,
    pub desvio_padrao: Option<f64>,
    pub diferenca: f64,
    pub variacao_percentual: Option<f64>,
    pub z_score: Option<f64>,
    pub direcao: DesvioDirecao,
}

// ============================================================
// CÁLCULO DO BASELINE PRÉ-VOO
// ============================================================

/// Calcula o baseline pré-voô a partir de amostras `(timestamp_utc, valor)`.
///
/// Retorna `None` se não houver amostras suficientes (`quantidade de amostras
/// válidas < min_amostras`). Valores `NaN`/`inf` são descartados.
pub fn calcular_baseline_pre_voo(
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    amostras: &[(&str, f64)],
    min_amostras: usize,
    atualizado_em: &str,
) -> Option<BaselineFisiologico> {
    let validas: Vec<(&str, f64)> = amostras
        .iter()
        .copied()
        .filter(|(_, valor)| valor.is_finite())
        .collect();

    if validas.is_empty() || validas.len() < min_amostras {
        return None;
    }

    let soma: f64 = validas.iter().map(|(_, valor)| valor).sum();
    let media = soma / validas.len() as f64;

    if !media.is_finite() {
        return None;
    }

    // Desvio padrão amostral (n - 1). Precisa de pelo menos 2 amostras.
    let desvio_padrao = if validas.len() >= 2 {
        let soma_quadrados: f64 = validas
            .iter()
            .map(|(_, valor)| (valor - media).powi(2))
            .sum();

        let variancia = soma_quadrados / (validas.len() - 1) as f64;

        if variancia.is_finite() {
            Some(variancia.sqrt())
        } else {
            None
        }
    } else {
        None
    };

    // Timestamps ISO ordenam lexicograficamente.
    let periodo_inicio = validas
        .iter()
        .map(|(timestamp, _)| *timestamp)
        .min()?
        .to_string();

    let periodo_fim = validas
        .iter()
        .map(|(timestamp, _)| *timestamp)
        .max()?
        .to_string();

    Some(BaselineFisiologico {
        id: Uuid::new_v4().to_string(),
        astronauta_id: astronauta_id.to_string(),
        indicador: indicador.to_string(),
        fase_missao: FASE_PRE_VOO.to_string(),
        missao_id: missao_id.map(str::to_string),
        valor_referencia: media,
        desvio_padrao,
        periodo_inicio: Some(periodo_inicio),
        periodo_fim: Some(periodo_fim),
        quantidade_amostras: Some(validas.len() as i64),
        atualizado_em: atualizado_em.to_string(),
        updated_at: Some(atualizado_em.to_string()),
        deleted_at: None,
    })
}

/// Extrai a série `(timestamp_utc, valor)` de um dos 9 indicadores fisiológicos.
pub fn extrair_amostras_telemetria<'a>(
    telemetrias: &'a [TelemetriaFisiologica],
    indicador: &str,
) -> Vec<(&'a str, f64)> {
    telemetrias
        .iter()
        .filter_map(|telemetria| {
            let valor = match indicador {
                INDICADOR_FREQUENCIA_CARDIACA => telemetria.frequencia_cardiaca_bpm,
                INDICADOR_SPO2 => telemetria.spo2_percentual,
                INDICADOR_TEMPERATURA_CORPORAL => telemetria.temperatura_corporal_c,
                INDICADOR_TEMPERATURA_PELE => telemetria.temperatura_pele_c,
                INDICADOR_TAXA_RESPIRATORIA => telemetria.taxa_respiratoria_rpm,
                INDICADOR_HRV => telemetria.hrv_ms,
                INDICADOR_PRESSAO_SISTOLICA => telemetria.pressao_sistolica_mmhg,
                INDICADOR_PRESSAO_DIASTOLICA => telemetria.pressao_diastolica_mmhg,
                INDICADOR_GLICOSE => telemetria.glicose_mg_dl,
                _ => None,
            }?;

            if !valor.is_finite() {
                return None;
            }

            Some((telemetria.timestamp_utc.as_str(), valor))
        })
        .collect()
}

// ============================================================
// COMPARAÇÃO COM O PRÉ-VOO
// ============================================================

fn classificar_direcao(diferenca: f64) -> DesvioDirecao {
    if !diferenca.is_finite() || diferenca.abs() < f64::EPSILON {
        DesvioDirecao::Estavel
    } else if diferenca > 0.0 {
        DesvioDirecao::Acima
    } else {
        DesvioDirecao::Abaixo
    }
}

fn construir_comparacao(
    indicador: &str,
    valor_atual: f64,
    valor_referencia: f64,
    desvio_padrao: Option<f64>,
) -> ComparacaoPreVoo {
    let diferenca = valor_atual - valor_referencia;

    // Proteção contra divisão por zero / não-finito.
    let variacao_percentual = if valor_referencia != 0.0 && valor_referencia.is_finite() {
        Some((diferenca / valor_referencia) * 100.0)
    } else {
        None
    };

    let z_score = match desvio_padrao {
        Some(dp) if dp.is_finite() && dp > 0.0 => Some(diferenca / dp),
        _ => None,
    };

    ComparacaoPreVoo {
        indicador: indicador.to_string(),
        valor_atual,
        valor_referencia_pre_voo: valor_referencia,
        desvio_padrao,
        diferenca,
        variacao_percentual,
        z_score,
        direcao: classificar_direcao(diferenca),
    }
}

/// Escolhe o baseline do indicador: prioriza a missão específica e,
/// não havendo, cai para o baseline global (`missao_id == None`).
fn escolher_baseline<'a, 'b>(
    baselines: &'a [&'b BaselineFisiologico],
    indicador: &str,
    missao_id: Option<&str>,
) -> Option<&'b BaselineFisiologico> {
    match missao_id {
        Some(missao) => baselines
            .iter()
            .copied()
            .find(|b| b.indicador == indicador && b.missao_id.as_deref() == Some(missao))
            .or_else(|| {
                baselines
                    .iter()
                    .copied()
                    .find(|b| b.indicador == indicador && b.missao_id.is_none())
            }),
        None => baselines
            .iter()
            .copied()
            .find(|b| b.indicador == indicador && b.missao_id.is_none()),
    }
}

fn baselines_pre_voo_validos<'a>(
    baselines: &'a [BaselineFisiologico],
    astronauta_id: &'a str,
) -> impl Iterator<Item = &'a BaselineFisiologico> + 'a {
    baselines
        .iter()
        .filter(move |b| {
            b.fase_missao == FASE_PRE_VOO
                && b.astronauta_id == astronauta_id
                && b.deleted_at.is_none()
        })
}

/// Compara cada indicador não-nulo da telemetria atual com o baseline
/// pré-voô correspondente do mesmo astronauta.
pub fn comparar_telemetria_com_pre_voo(
    atual: &TelemetriaFisiologica,
    baselines: &[BaselineFisiologico],
) -> Vec<ComparacaoPreVoo> {
    let pre_voo: Vec<&BaselineFisiologico> =
        baselines_pre_voo_validos(baselines, &atual.astronauta_id).collect();

    let missao_atual = atual.missao_id.as_deref();

    let pares: [(&str, Option<f64>); 9] = [
        (
            INDICADOR_FREQUENCIA_CARDIACA,
            atual.frequencia_cardiaca_bpm,
        ),
        (INDICADOR_SPO2, atual.spo2_percentual),
        (
            INDICADOR_TEMPERATURA_CORPORAL,
            atual.temperatura_corporal_c,
        ),
        (INDICADOR_TEMPERATURA_PELE, atual.temperatura_pele_c),
        (
            INDICADOR_TAXA_RESPIRATORIA,
            atual.taxa_respiratoria_rpm,
        ),
        (INDICADOR_HRV, atual.hrv_ms),
        (
            INDICADOR_PRESSAO_SISTOLICA,
            atual.pressao_sistolica_mmhg,
        ),
        (
            INDICADOR_PRESSAO_DIASTOLICA,
            atual.pressao_diastolica_mmhg,
        ),
        (INDICADOR_GLICOSE, atual.glicose_mg_dl),
    ];

    pares
        .into_iter()
        .filter_map(|(indicador, valor)| {
            let valor = valor?;

            if !valor.is_finite() {
                return None;
            }

            let baseline = escolher_baseline(&pre_voo, indicador, missao_atual)?;

            Some(construir_comparacao(
                indicador,
                valor,
                baseline.valor_referencia,
                baseline.desvio_padrao,
            ))
        })
        .collect()
}

/// Compara as horas de sono do registro diário com o baseline pré-voô
/// do indicador `HORAS_SONO`.
pub fn comparar_sono_com_pre_voo(
    registro: &RegistroDiario,
    baselines: &[BaselineFisiologico],
) -> Option<ComparacaoPreVoo> {
    let horas_sono = registro.horas_sono_total?;

    if !horas_sono.is_finite() {
        return None;
    }

    let pre_voo: Vec<&BaselineFisiologico> =
        baselines_pre_voo_validos(baselines, &registro.astronauta_id).collect();

    // Registro diário não tem missão associada: usa o baseline global.
    let baseline = escolher_baseline(&pre_voo, INDICADOR_HORAS_SONO, None)?;

    Some(construir_comparacao(
        INDICADOR_HORAS_SONO,
        horas_sono,
        baseline.valor_referencia,
        baseline.desvio_padrao,
    ))
}

// ============================================================
// VARIAÇÕES COM REPOSITÓRIO (BUSCAM OS PRÓPRIOS DADOS)
// ============================================================

use crate::domain::repositories::measurement_repository::MeasurementRepository;

use super::repository::buscar_serie_do_indicador;

/// Calcula o baseline pré-voô buscando a série temporal do indicador
/// direto do repositório (janela `[inicio_utc, fim_utc]` opcional).
pub async fn calcular_baseline_pre_voo_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    missao_id: Option<&str>,
    indicador: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    min_amostras: usize,
    atualizado_em: &str,
) -> Result<Option<BaselineFisiologico>, sqlx::Error> {
    let serie = buscar_serie_do_indicador(
        repo,
        astronauta_id,
        missao_id,
        indicador,
        inicio_utc,
        fim_utc,
    )
    .await?;

    let amostras: Vec<(&str, f64)> = serie
        .iter()
        .map(|ponto| (ponto.momento_utc.as_str(), ponto.valor))
        .collect();

    Ok(calcular_baseline_pre_voo(
        astronauta_id,
        missao_id,
        indicador,
        &amostras,
        min_amostras,
        atualizado_em,
    ))
}

/// Compara a telemetria atual com os baselines pré-voô buscados do
/// repositório (todos do astronauta; a prioridade missão → global fica
/// por conta da função pura).
pub async fn comparar_telemetria_com_pre_voo_de_repositorio(
    repo: &dyn MeasurementRepository,
    atual: &TelemetriaFisiologica,
) -> Result<Vec<ComparacaoPreVoo>, sqlx::Error> {
    let baselines = repo
        .list_baselines(&atual.astronauta_id, None, Some(FASE_PRE_VOO))
        .await?;

    Ok(comparar_telemetria_com_pre_voo(atual, &baselines))
}

/// Compara as horas de sono do registro diário (buscado por `id`) com o
/// baseline pré-voô do indicador `HORAS_SONO`.
pub async fn comparar_sono_com_pre_voo_de_repositorio(
    repo: &dyn MeasurementRepository,
    registro_diario_id: &str,
) -> Result<Option<ComparacaoPreVoo>, sqlx::Error> {
    let Some(registro) = repo.find_daily_log_by_id(registro_diario_id).await? else {
        return Ok(None);
    };

    let baselines = repo
        .list_baselines(&registro.astronauta_id, None, Some(FASE_PRE_VOO))
        .await?;

    Ok(comparar_sono_com_pre_voo(&registro, &baselines))
}

// ============================================================
// TESTES
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn telemetria(astronauta_id: &str, missao_id: Option<&str>, fc: Option<f64>) -> TelemetriaFisiologica {
        TelemetriaFisiologica {
            id: "t1".to_string(),
            astronauta_id: astronauta_id.to_string(),
            missao_id: missao_id.map(str::to_string),
            timestamp_utc: "2026-06-01T12:00:00Z".to_string(),
            frequencia_cardiaca_bpm: fc,
            spo2_percentual: None,
            temperatura_corporal_c: None,
            temperatura_pele_c: None,
            taxa_respiratoria_rpm: None,
            hrv_ms: None,
            pressao_sistolica_mmhg: None,
            pressao_diastolica_mmhg: None,
            glicose_mg_dl: None,
            origem_medicao: None,
            updated_at: None,
            deleted_at: None,
        }
    }

    fn baseline(
        astronauta_id: &str,
        missao_id: Option<&str>,
        indicador: &str,
        valor_referencia: f64,
        desvio_padrao: Option<f64>,
    ) -> BaselineFisiologico {
        BaselineFisiologico {
            id: format!("b-{indicador}-{missao_id:?}"),
            astronauta_id: astronauta_id.to_string(),
            indicador: indicador.to_string(),
            fase_missao: FASE_PRE_VOO.to_string(),
            missao_id: missao_id.map(str::to_string),
            valor_referencia,
            desvio_padrao,
            periodo_inicio: None,
            periodo_fim: None,
            quantidade_amostras: None,
            atualizado_em: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn baseline_pre_voo_com_amostras_suficientes() {
        let amostras = vec![
            ("2026-05-01T10:00:00Z", 70.0),
            ("2026-05-02T10:00:00Z", 72.0),
            ("2026-05-03T10:00:00Z", 74.0),
            ("2026-05-04T10:00:00Z", f64::NAN), // inválida: ignorada
        ];

        let resultado = calcular_baseline_pre_voo(
            "astro-1",
            Some("missao-1"),
            INDICADOR_FREQUENCIA_CARDIACA,
            &amostras,
            3,
            "2026-05-05T00:00:00Z",
        )
        .expect("deveria calcular o baseline");

        assert_eq!(resultado.astronauta_id, "astro-1");
        assert_eq!(resultado.missao_id.as_deref(), Some("missao-1"));
        assert_eq!(resultado.indicador, INDICADOR_FREQUENCIA_CARDIACA);
        assert_eq!(resultado.fase_missao, "PRE_VOO");
        assert_eq!(resultado.valor_referencia, 72.0);
        assert_eq!(resultado.desvio_padrao, Some(2.0));
        assert_eq!(
            resultado.periodo_inicio.as_deref(),
            Some("2026-05-01T10:00:00Z")
        );
        assert_eq!(
            resultado.periodo_fim.as_deref(),
            Some("2026-05-03T10:00:00Z") // amostra NaN de 05-04 é descartada
        );
        assert_eq!(resultado.quantidade_amostras, Some(3));
    }

    #[test]
    fn baseline_pre_voo_com_amostras_insuficientes() {
        let amostras = vec![
            ("2026-05-01T10:00:00Z", 70.0),
            ("2026-05-02T10:00:00Z", 72.0),
        ];

        let resultado = calcular_baseline_pre_voo(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            &amostras,
            3,
            "2026-05-05T00:00:00Z",
        );

        assert!(resultado.is_none());
    }

    #[test]
    fn baseline_pre_voo_vazio() {
        let resultado = calcular_baseline_pre_voo(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            &[],
            1,
            "2026-05-05T00:00:00Z",
        );

        assert!(resultado.is_none());
    }

    #[test]
    fn extrair_amostras_de_telemetria() {
        let telemetrias = vec![
            telemetria("astro-1", None, Some(70.0)),
            telemetria("astro-1", None, None),
            telemetria("astro-1", None, Some(f64::INFINITY)),
            telemetria("astro-1", None, Some(74.0)),
        ];

        let amostras =
            extrair_amostras_telemetria(&telemetrias, INDICADOR_FREQUENCIA_CARDIACA);

        assert_eq!(amostras.len(), 2);
        assert_eq!(amostras[0].1, 70.0);
        assert_eq!(amostras[1].1, 74.0);

        // Indicador desconhecido não extrai nada.
        assert!(extrair_amostras_telemetria(&telemetrias, "FOO").is_empty());
    }

    #[test]
    fn comparacao_acima_abaixo_e_estavel() {
        let pre_voo = vec![baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            72.0,
            Some(2.0),
        )];

        // Acima do baseline.
        let acima = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(76.0)),
            &pre_voo,
        );
        assert_eq!(acima.len(), 1);
        assert_eq!(acima[0].diferenca, 4.0);
        assert!((acima[0].variacao_percentual.unwrap() - (4.0 / 72.0) * 100.0).abs() < 1e-9);
        assert_eq!(acima[0].z_score, Some(2.0));
        assert_eq!(acima[0].direcao, DesvioDirecao::Acima);

        // Abaixo do baseline.
        let abaixo = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(69.0)),
            &pre_voo,
        );
        assert_eq!(abaixo[0].diferenca, -3.0);
        assert_eq!(abaixo[0].z_score, Some(-1.5));
        assert_eq!(abaixo[0].direcao, DesvioDirecao::Abaixo);

        // Estável (dentro da tolerância de zero).
        let estavel = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(72.0)),
            &pre_voo,
        );
        assert_eq!(estavel[0].diferenca, 0.0);
        assert_eq!(estavel[0].variacao_percentual, Some(0.0));
        assert_eq!(estavel[0].z_score, Some(0.0));
        assert_eq!(estavel[0].direcao, DesvioDirecao::Estavel);
    }

    #[test]
    fn comparacao_sem_valor_atual_ou_sem_baseline() {
        let pre_voo = vec![baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            72.0,
            Some(2.0),
        )];

        // Nenhum indicador preenchido: nada a comparar.
        assert!(comparar_telemetria_com_pre_voo(&telemetria("astro-1", None, None), &pre_voo).is_empty());

        // Baseline de outro astronauta: ignorado.
        let outro = vec![baseline(
            "astro-2",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            72.0,
            Some(2.0),
        )];
        assert!(comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(76.0)),
            &outro
        )
        .is_empty());
    }

    #[test]
    fn protecao_contra_divisao_por_zero() {
        // valor_referencia == 0.0: variacao_percentual deve ser None.
        let sem_referencia = vec![baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            0.0,
            Some(2.0),
        )];

        let resultado = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(5.0)),
            &sem_referencia,
        );
        assert_eq!(resultado[0].variacao_percentual, None);
        assert_eq!(resultado[0].z_score, Some(2.5));
        assert_eq!(resultado[0].direcao, DesvioDirecao::Acima);

        // desvio_padrao == Some(0.0): z_score deve ser None.
        let sem_desvio = vec![baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            72.0,
            Some(0.0),
        )];

        let resultado = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(76.0)),
            &sem_desvio,
        );
        assert_eq!(resultado[0].z_score, None);
        assert_eq!(resultado[0].variacao_percentual, Some((4.0 / 72.0) * 100.0));
    }

    #[test]
    fn baseline_da_missao_tem_prioridade_sobre_o_global() {
        let da_missao = baseline(
            "astro-1",
            Some("missao-1"),
            INDICADOR_FREQUENCIA_CARDIACA,
            60.0,
            Some(1.0),
        );
        let global = baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            50.0,
            Some(1.0),
        );

        let baselines = vec![global, da_missao];

        // Com missão: usa o baseline específico da missão.
        let com_missao = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", Some("missao-1"), Some(66.0)),
            &baselines,
        );
        assert_eq!(com_missao[0].valor_referencia_pre_voo, 60.0);

        // Sem missão: usa apenas o baseline global.
        let sem_missao = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(55.0)),
            &baselines,
        );
        assert_eq!(sem_missao[0].valor_referencia_pre_voo, 50.0);
    }

    #[test]
    fn baselines_fora_do_pre_voo_ou_deletados_sao_ignorados() {
        let mut durante_voo = baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            100.0,
            Some(1.0),
        );
        durante_voo.fase_missao = "DURANTE_VOO".to_string();
        let mut deletado = baseline(
            "astro-1",
            None,
            INDICADOR_FREQUENCIA_CARDIACA,
            200.0,
            Some(1.0),
        );
        deletado.deleted_at = Some("2026-06-02T00:00:00Z".to_string());

        let baselines = vec![durante_voo, deletado];

        assert!(comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(76.0)),
            &baselines
        )
        .is_empty());
    }

    #[test]
    fn comparacao_sono_com_pre_voo() {
        let mut registro = RegistroDiario {
            id: "r1".to_string(),
            astronauta_id: "astro-1".to_string(),
            data_registro: "2026-06-01".to_string(),
            horas_sono_total: Some(5.0),
            qualidade_sono_score: None,
            exercicio_resistencia_min: None,
            exercicio_aerobico_min: None,
            ingestao_agua_ml: None,
            calorias_consumidas: None,
            score_teste_cognitivo: None,
            nivel_estresse_subjetivo: None,
            humor_score: None,
            fadiga_score: None,
            updated_at: None,
            deleted_at: None,
        };

        let baselines = vec![baseline(
            "astro-1",
            None,
            INDICADOR_HORAS_SONO,
            8.0,
            Some(1.0),
        )];

        let comparacao = comparar_sono_com_pre_voo(&registro, &baselines).expect("deveria comparar");
        assert_eq!(comparacao.indicador, INDICADOR_HORAS_SONO);
        assert_eq!(comparacao.valor_atual, 5.0);
        assert_eq!(comparacao.valor_referencia_pre_voo, 8.0);
        assert_eq!(comparacao.diferenca, -3.0);
        assert_eq!(comparacao.variacao_percentual, Some(-37.5));
        assert_eq!(comparacao.z_score, Some(-3.0));
        assert_eq!(comparacao.direcao, DesvioDirecao::Abaixo);

        // Sem horas de sono: sem comparação.
        registro.horas_sono_total = None;
        assert!(comparar_sono_com_pre_voo(&registro, &baselines).is_none());
    }

    #[test]
    fn comparacao_serializa_e_desserializa() {
        let pre_voo = vec![baseline(
            "astro-1",
            None,
            INDICADOR_SPO2,
            98.0,
            Some(1.0),
        )];

        let resultado = comparar_telemetria_com_pre_voo(
            &telemetria("astro-1", None, Some(95.0)),
            &pre_voo,
        );

        let json = serde_json::to_string(&resultado).expect("deveria serializar");
        let de_volta: Vec<ComparacaoPreVoo> =
            serde_json::from_str(&json).expect("deveria desserializar");

        assert_eq!(de_volta, resultado);
    }
}
