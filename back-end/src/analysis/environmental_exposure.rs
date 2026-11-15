/*
VE A LOCALIZAÇÃO PRA CALCULAR A EXPOSIÇÃO A DADOS COMO CO2, RADIACAO TEMPERATURA E PRESSAO

A integração temporal usa o MÉTODO TRAPESOIDAL. O cruzamento de limites é
estimado por INTERPOLAÇÃO LINEAR entre as duas leituras do intervalo, então
o tempo acima/fora da faixa não exige que as DUAS leituras estejam fora.

UNIDADES E LIMITAÇÃO (importante):
- `radiacao_msv_hora` é uma TAXA em mSv/h; integrada por horas resulta em mSv.
- A integração de uma taxa ambiental estima EXPOSIÇÃO AMBIENTAL ACUMULADA
  no módulo — NÃO é necessariamente a dose individual recebida pelo
  astronauta (posição no módulo, blindagem e tempo real de permanência
  não são modelados aqui).
- Os limiares (`EnvironmentalThresholds`) são CONFIGURÁVEIS pela camada de
  aplicação; nenhuma faixa clínica/operacional é inventada neste módulo.
*/
use std::collections::HashSet;
use chrono::{DateTime, Utc};
use crate::domain::repositories::measurement_repository::MeasurementRepository;
use crate::domain::entities::measurement_entity::{AmbienteNave, PresencaModulo};
use super::repository::{LIMITE_PADRAO_SERIE, verificar_limite};
use super::statistics::trapezoid;

#[derive(Debug, Clone, Default)]
pub struct EnvironmentalThresholds {
    pub co2_ppm: f64,
    pub temperature_c: f64,
    /// Faixa aceitável de pressão (kPa). Fora da faixa conta como exposição.
    pub pressure_kpa_min: f64,
    pub pressure_kpa_max: f64,
}

impl EnvironmentalThresholds {
    pub fn validar(&self) -> Result<(), String> {
        if !self.co2_ppm.is_finite() || !self.temperature_c.is_finite() {
            return Err("limiares de CO₂ e temperatura devem ser finitos".to_string());
        }
        if !self.pressure_kpa_min.is_finite() || !self.pressure_kpa_max.is_finite() {
            return Err("limites de pressão devem ser finitos".to_string());
        }
        if self.pressure_kpa_min >= self.pressure_kpa_max {
            return Err(format!(
                "faixa de pressão invertida: min ({}) >= max ({})",
                self.pressure_kpa_min, self.pressure_kpa_max
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnvironmentalExposureResult {
    pub co2_exposure_ppm_hours: f64,
    pub radiation_dose_msv: f64,
    pub temperature_exposure_degree_hours: f64,
    /// Integração trapezoidal do EXCESSO fora da faixa de pressão (kPa·h).
    pub pressure_exposure_kpa_hours: f64,
    pub co2_time_above_seconds: f64,
    pub temperature_time_above_seconds: f64,
    /// Tempo fora da faixa de pressão (abaixo do mínimo OU acima do máximo).
    pub pressure_time_outside_range_seconds: f64,
    /// Tempo total integrado par a par (qualquer indicador disponível).
    /// Para disponibilidade POR INDICADOR use os campos específicos abaixo:
    /// cada um só cresce quando as DUAS leituras necessárias para integrar
    /// aquele intervalo estiverem presentes e válidas.
    pub analyzed_seconds: f64,
    pub co2_analyzed_seconds: f64,
    pub radiation_analyzed_seconds: f64,
    pub temperature_analyzed_seconds: f64,
    pub pressure_analyzed_seconds: f64,
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn positive_excess(value: f64, threshold: f64) -> f64 {
    (value - threshold).max(0.0)
}

/// Excesso fora de uma faixa: distância abaixo do mínimo OU acima do máximo.
fn excesso_fora_da_faixa(value: f64, minimo: f64, maximo: f64) -> f64 {
    (minimo - value).max(0.0) + (value - maximo).max(0.0)
}

/// Tempo (em segundos, dentro do intervalo `[t_a, t_b]`) em que o valor
/// interpolado linearmente está ESTRITAMENTE acima do limiar.
///
/// Em vez de exigir que as duas leituras estejam acima do limite, estima o
/// instante de cruzamento `t_c` por interpolação e integra só o trecho
/// acima — um pico entre duas leituras normais e uma subida lenta passando
/// do limite deixam de ser ignoradas ou supercontadas.
fn tempo_acima(t_a: f64, v_a: f64, t_b: f64, v_b: f64, limiar: f64) -> f64 {
    let dt = t_b - t_a;
    if dt <= 0.0 || !v_a.is_finite() || !v_b.is_finite() || !limiar.is_finite() {
        return 0.0;
    }
    match (v_a > limiar, v_b > limiar) {
        (true, true) => dt,
        (false, false) => 0.0,
        (acima_a, _) => {
            // Instante do cruzamento (relativo a t_a). Se um lado está acima
            // e o outro não, os valores são distintos e t_c é bem definido.
            let t_c = (limiar - v_a) * dt / (v_b - v_a);
            let acima_no_fim = acima_a;
            if acima_no_fim {
                t_c.clamp(0.0, dt) // acima do limite do início até o cruzamento
            } else {
                (dt - t_c).clamp(0.0, dt) // acima do cruzamento até o fim
            }
        }
    }
}

/// Intervalo de presença já convertido (com fim `now` quando aberto).
struct IntervaloPresenca {
    modulo_id: String,
    inicio: DateTime<Utc>,
    fim: DateTime<Utc>,
}

/// Converte as presenças em intervalos temporais e MERGE sobreposições do
/// mesmo módulo, evitando contabilizar duas vezes o mesmo intervalo.
/// Presenças removidas (`deleted_at`) e timestamps inválidos são ignoradas.
fn intervalos_de_presenca(
    presence: &[PresencaModulo],
    agora: DateTime<Utc>,
) -> Vec<IntervaloPresenca> {
    let mut intervalos: Vec<IntervaloPresenca> = presence
        .iter()
        .filter(|stay| stay.deleted_at.is_none())
        .filter_map(|stay| {
            let inicio = parse_timestamp(&stay.inicio_utc)?;
            let fim = match &stay.fim_utc {
                Some(value) => parse_timestamp(value)?,
                None => agora,
            };
            // Fim anterior ao início / intervalo negativo → inválido.
            if fim <= inicio {
                return None;
            }
            Some(IntervaloPresenca {
                modulo_id: stay.modulo_id.clone(),
                inicio,
                fim,
            })
        })
        .collect();

    // Ordena por módulo e início para agrupar sobreposições.
    intervalos.sort_by(|a, b| {
        a.modulo_id
            .cmp(&b.modulo_id)
            .then_with(|| a.inicio.cmp(&b.inicio))
    });

    // Merge: quando o próximo intervalo do mesmo módulo começa antes do fim
    // do acumulado, os dois cobrem o mesmo período — mantém só a união.
    let mut mesclados: Vec<IntervaloPresenca> = Vec::with_capacity(intervalos.len());
    for intervalo in intervalos {
        if let Some(ultimo) = mesclados.last_mut()
            && ultimo.modulo_id == intervalo.modulo_id
            && intervalo.inicio <= ultimo.fim
        {
            ultimo.fim = ultimo.fim.max(intervalo.fim);
            continue;
        }
        mesclados.push(intervalo);
    }
    mesclados
}

pub fn calculate_environmental_exposure(
    presence: &[PresencaModulo],
    environment: &[AmbienteNave],
    thresholds: &EnvironmentalThresholds,
    max_gap_seconds: i64,
    agora: DateTime<Utc>,
) -> EnvironmentalExposureResult {
    let mut result = EnvironmentalExposureResult::default();
    let pressao_valida = thresholds.validar().is_ok();

    for stay in intervalos_de_presenca(presence, agora) {
        // Medições do módulo correspondente à presença, dentro do intervalo.
        let mut readings: Vec<_> = environment
            .iter()
            .filter(|reading| reading.modulo_id == stay.modulo_id)
            .filter(|reading| reading.deleted_at.is_none())
            .filter_map(|reading| {
                let timestamp = parse_timestamp(&reading.timestamp_utc)?;
                if timestamp < stay.inicio || timestamp > stay.fim {
                    return None;
                }
                Some((timestamp, reading))
            })
            .collect();

        readings.sort_by_key(|reading| reading.0);

        for pair in readings.windows(2) {
            let (time_a, a) = pair[0];
            let (time_b, b) = pair[1];

            let duration_seconds = (time_b - time_a).num_milliseconds() as f64 / 1000.0;

            // Intervalo negativo/nulo ou lacuna superior ao máximo não integra.
            if duration_seconds <= 0.0 || duration_seconds > max_gap_seconds as f64 {
                continue;
            }

            let duration_hours = duration_seconds / 3600.0;

            result.analyzed_seconds += duration_seconds;

            // CO2: integral do excesso acima do limiar + tempo acima com
            // interpolação do cruzamento.
            if let (Some(co2_a), Some(co2_b)) = (a.co2_ppm, b.co2_ppm)
                && co2_a.is_finite()
                && co2_b.is_finite()
            {
                result.co2_analyzed_seconds += duration_seconds;
                result.co2_exposure_ppm_hours += trapezoid(
                    positive_excess(co2_a, thresholds.co2_ppm),
                    positive_excess(co2_b, thresholds.co2_ppm),
                    duration_hours,
                );
                result.co2_time_above_seconds +=
                    tempo_acima(0.0, co2_a, duration_seconds, co2_b, thresholds.co2_ppm);
            }

            // Radiação: integra a TAXA ambiental (mSv/h × h = mSv).
            if let (Some(rate_a), Some(rate_b)) = (a.radiacao_msv_hora, b.radiacao_msv_hora)
                && rate_a.is_finite()
                && rate_b.is_finite()
                && rate_a >= 0.0
                && rate_b >= 0.0
            {
                result.radiation_analyzed_seconds += duration_seconds;
                result.radiation_dose_msv += trapezoid(rate_a, rate_b, duration_hours);
            }

            // Temperatura: integral do excesso + tempo acima com interpolação.
            if let (Some(temp_a), Some(temp_b)) = (a.temperatura_cabine_c, b.temperatura_cabine_c)
                && temp_a.is_finite()
                && temp_b.is_finite()
            {
                result.temperature_analyzed_seconds += duration_seconds;
                result.temperature_exposure_degree_hours += trapezoid(
                    positive_excess(temp_a, thresholds.temperature_c),
                    positive_excess(temp_b, thresholds.temperature_c),
                    duration_hours,
                );
                result.temperature_time_above_seconds += tempo_acima(
                    0.0,
                    temp_a,
                    duration_seconds,
                    temp_b,
                    thresholds.temperature_c,
                );
            }

            // Pressão: integral do excesso fora da faixa + tempo fora da faixa.
            if pressao_valida
                && let (Some(p_a), Some(p_b)) = (a.pressao_kpa, b.pressao_kpa)
                && p_a.is_finite()
                && p_b.is_finite()
            {
                result.pressure_analyzed_seconds += duration_seconds;
                result.pressure_exposure_kpa_hours += trapezoid(
                    excesso_fora_da_faixa(
                        p_a,
                        thresholds.pressure_kpa_min,
                        thresholds.pressure_kpa_max,
                    ),
                    excesso_fora_da_faixa(
                        p_b,
                        thresholds.pressure_kpa_min,
                        thresholds.pressure_kpa_max,
                    ),
                    duration_hours,
                );
                let abaixo_min = tempo_acima(
                    0.0,
                    -p_a,
                    duration_seconds,
                    -p_b,
                    -thresholds.pressure_kpa_min,
                );
                let acima_max =
                    tempo_acima(0.0, p_a, duration_seconds, p_b, thresholds.pressure_kpa_max);
                result.pressure_time_outside_range_seconds += abaixo_min + acima_max;
            }
        }
    }

    result
}

// ============================================================
// EXPOSIÇÃO COM REPOSITÓRIO (BUSCA OS PRÓPRIOS DADOS)
// ============================================================

pub async fn calculate_environmental_exposure_de_repositorio(
    repo: &dyn MeasurementRepository,
    astronauta_id: &str,
    inicio_utc: Option<&str>,
    fim_utc: Option<&str>,
    thresholds: &EnvironmentalThresholds,
    max_gap_seconds: i64,
    agora: DateTime<Utc>,
) -> Result<EnvironmentalExposureResult, sqlx::Error> {
    let presences = repo.list_presences(astronauta_id).await?;

    let modulos: HashSet<&str> = presences
        .iter()
        .filter(|p| p.deleted_at.is_none())
        .map(|p| p.modulo_id.as_str())
        .collect();

    let mut ambientes = Vec::new();
    for modulo_id in modulos {
        // limite + 1 para detectar truncamento: falha em vez de truncar.
        let leituras = repo
            .list_environment_by_module(modulo_id, inicio_utc, fim_utc, LIMITE_PADRAO_SERIE + 1, 0)
            .await?;
        verificar_limite(leituras.len())?;
        ambientes.extend(leituras);
    }

    Ok(calculate_environmental_exposure(
        &presences, &ambientes, thresholds, max_gap_seconds, agora,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn ts(valor: &str) -> String {
        valor.to_string()
    }

    fn ambiente(
        id: &str,
        co2: Option<f64>,
        radiacao: Option<f64>,
        temperatura: Option<f64>,
        pressao: Option<f64>,
        timestamp: &str,
    ) -> AmbienteNave {
        AmbienteNave {
            id: id.to_string(),
            modulo_id: "MOD1".to_string(),
            timestamp_utc: ts(timestamp),
            co2_ppm: co2,
            radiacao_msv_hora: radiacao,
            temperatura_cabine_c: temperatura,
            pressao_kpa: pressao,
            updated_at: None,
            deleted_at: None,
        }
    }

    fn presenca(id: &str, inicio: &str, fim: Option<&str>) -> PresencaModulo {
        PresencaModulo {
            id: id.to_string(),
            astronauta_id: "A1".to_string(),
            modulo_id: "MOD1".to_string(),
            inicio_utc: ts(inicio),
            fim_utc: fim.map(ts),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn limiares() -> EnvironmentalThresholds {
        EnvironmentalThresholds {
            co2_ppm: 1000.0,
            temperature_c: 21.0,
            pressure_kpa_min: 95.0,
            pressure_kpa_max: 105.0,
        }
    }

    #[test]
    fn exposicao_normal_abaixo_dos_limites() {
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T10:00:00Z"),
        )];
        let env = vec![
            ambiente(
                "E1",
                Some(800.0),
                Some(0.10),
                Some(20.0),
                Some(101.3),
                "2025-01-15T08:00:00Z",
            ),
            ambiente(
                "E2",
                Some(900.0),
                Some(0.10),
                Some(20.0),
                Some(101.3),
                "2025-01-15T09:00:00Z",
            ),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        assert!((r.co2_exposure_ppm_hours - 0.0).abs() < 1e-9);
        assert_eq!(r.co2_time_above_seconds, 0.0);
        assert!((r.radiation_dose_msv - 0.10).abs() < 1e-9); // 0.10 mSv/h × 1 h
        assert!(r.temperature_exposure_degree_hours.abs() < 1e-9);
        assert!(r.pressure_exposure_kpa_hours.abs() < 1e-9);
        assert_eq!(r.pressure_time_outside_range_seconds, 0.0);
        assert_eq!(r.analyzed_seconds, 3600.0);
        assert_eq!(r.co2_analyzed_seconds, 3600.0);
        assert_eq!(r.radiation_analyzed_seconds, 3600.0);
        assert_eq!(r.temperature_analyzed_seconds, 3600.0);
        assert_eq!(r.pressure_analyzed_seconds, 3600.0);
    }

    #[test]
    fn exposicao_acima_do_limite_com_duas_leituras_altas() {
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T09:00:00Z"),
        )];
        let env = vec![
            ambiente("E1", Some(1200.0), None, None, None, "2025-01-15T08:00:00Z"),
            ambiente("E2", Some(1500.0), None, None, None, "2025-01-15T09:00:00Z"),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        // Excesso médio (200 + 500)/2 = 350 ppm × 1 h.
        assert!((r.co2_exposure_ppm_hours - 350.0).abs() < 1e-9);
        assert_eq!(r.co2_time_above_seconds, 3600.0);
        assert_eq!(r.co2_analyzed_seconds, 3600.0);
    }

    #[test]
    fn cruzamento_de_limite_e_estimado_por_interpolacao() {
        // Sobe de 900 para 1100 ppm em 1 h: cruza 1000 em 50 min.
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T09:00:00Z"),
        )];
        let env = vec![
            ambiente("E1", Some(900.0), None, None, None, "2025-01-15T08:00:00Z"),
            ambiente("E2", Some(1100.0), None, None, None, "2025-01-15T09:00:00Z"),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        // Nenhuma leitura está acima, mas o trecho interpolado sim:
        // (1100-1000)/(1100-900) × 3600 = 1800 s.
        assert!((r.co2_time_above_seconds - 1800.0).abs() < 1e-6);
        // Pico interpolado de 100 ppm no fim: trapézio = 50 ppm·h.
        assert!((r.co2_exposure_ppm_hours - 50.0).abs() < 1e-9);
    }

    #[test]
    fn leituras_ausentes_nao_integram_nem_zeram() {
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T09:00:00Z"),
        )];
        let env = vec![
            ambiente(
                "E1",
                None,
                Some(0.10),
                None,
                Some(101.0),
                "2025-01-15T08:00:00Z",
            ),
            ambiente(
                "E2",
                Some(1200.0),
                None,
                Some(25.0),
                None,
                "2025-01-15T09:00:00Z",
            ),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        // Nenhum indicador tem as DUAS leituras → nada integrado.
        assert_eq!(r.co2_exposure_ppm_hours, 0.0);
        assert_eq!(r.radiation_dose_msv, 0.0);
        assert_eq!(r.temperature_exposure_degree_hours, 0.0);
        assert_eq!(r.pressure_exposure_kpa_hours, 0.0);
        // Tempo por indicador reflete a disponibilidade real de cada um.
        assert_eq!(r.co2_analyzed_seconds, 0.0);
        assert_eq!(r.radiation_analyzed_seconds, 0.0);
        assert_eq!(r.temperature_analyzed_seconds, 0.0);
        assert_eq!(r.pressure_analyzed_seconds, 0.0);
        // Mas o intervalo foi analisado no total (legado).
        assert_eq!(r.analyzed_seconds, 3600.0);
    }

    #[test]
    fn lacunas_temporais_nao_sao_integradas() {
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T12:00:00Z"),
        )];
        let env = vec![
            ambiente("E1", Some(800.0), None, None, None, "2025-01-15T08:00:00Z"),
            ambiente("E2", Some(1600.0), None, None, None, "2025-01-15T11:00:00Z"), // gap de 3 h
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());
        assert_eq!(
            r.analyzed_seconds, 0.0,
            "lacuna maior que max_gap não integra"
        );
        assert_eq!(r.co2_exposure_ppm_hours, 0.0);
    }

    #[test]
    fn presencas_sobrepostas_nao_duplicam_o_intervalo() {
        // Duas presenças sobrepostas (08:00–10:00 e 09:00–11:00).
        let presences = vec![
            presenca("P1", "2025-01-15T08:00:00Z", Some("2025-01-15T10:00:00Z")),
            presenca("P2", "2025-01-15T09:00:00Z", Some("2025-01-15T11:00:00Z")),
        ];
        let env = vec![
            ambiente(
                "E1",
                Some(800.0),
                Some(0.10),
                Some(20.0),
                Some(101.0),
                "2025-01-15T08:00:00Z",
            ),
            ambiente(
                "E2",
                Some(800.0),
                Some(0.10),
                Some(20.0),
                Some(101.0),
                "2025-01-15T09:00:00Z",
            ),
            ambiente(
                "E3",
                Some(800.0),
                Some(0.10),
                Some(20.0),
                Some(101.0),
                "2025-01-15T10:00:00Z",
            ),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        // Sem o merge, o par 09:00→10:00 seria contado nas duas presenças
        // (10800 s). Mescladas: apenas os pares 08→09 e 09→10 (7200 s).
        assert_eq!(r.analyzed_seconds, 7200.0);
        assert!((r.radiation_dose_msv - 0.20).abs() < 1e-9);
    }

    #[test]
    fn presencas_de_modulos_diferentes_nao_sao_mescladas() {
        let mut p2 = presenca("P2", "2025-01-15T09:00:00Z", Some("2025-01-15T10:00:00Z"));
        p2.modulo_id = "MOD2".to_string();
        let presences = vec![
            presenca("P1", "2025-01-15T08:00:00Z", Some("2025-01-15T10:00:00Z")),
            p2,
        ];
        // Só MOD1 tem leituras.
        let env = vec![
            ambiente("E1", Some(800.0), None, None, None, "2025-01-15T08:00:00Z"),
            ambiente("E2", Some(800.0), None, None, None, "2025-01-15T09:00:00Z"),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());
        // MOD1 tem apenas o par 08:00→09:00 (3600 s); MOD2 não tem leituras
        // e NÃO herda o intervalo de MOD1 (módulos não são mesclados).
        assert_eq!(r.analyzed_seconds, 3600.0);
    }

    #[test]
    fn integracao_da_taxa_de_radiacao() {
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T10:00:00Z"),
        )];
        let env = vec![
            ambiente("E1", None, Some(0.10), None, None, "2025-01-15T08:00:00Z"),
            ambiente("E2", None, Some(0.30), None, None, "2025-01-15T09:00:00Z"),
            ambiente("E3", None, Some(0.30), None, None, "2025-01-15T10:00:00Z"),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        // mSv/h × h: (0.10+0.30)/2 × 1 + (0.30+0.30)/2 × 1 = 0.50 mSv.
        assert!((r.radiation_dose_msv - 0.50).abs() < 1e-9);
        assert_eq!(r.radiation_analyzed_seconds, 7200.0);
    }

    #[test]
    fn pressao_fora_da_faixa_e_contabilizada() {
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T09:00:00Z"),
        )];
        let env = vec![
            ambiente("E1", None, None, None, Some(94.0), "2025-01-15T08:00:00Z"),
            ambiente("E2", None, None, None, Some(106.0), "2025-01-15T09:00:00Z"),
        ];

        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());

        // Excesso: min 1.0 + max 1.0 → média 1.0 kPa × 1 h = 1.0 kPa·h.
        assert!((r.pressure_exposure_kpa_hours - 1.0).abs() < 1e-9);
        // Tempo fora: cruza 95 (baixo) e 105 (alto) por interpolação.
        // Abaixo de 95: de 0 até cruzar 95 (1/12 do intervalo = 300 s).
        // Acima de 105: de cruzar 105 (11/12) até o fim = 300 s.
        assert!((r.pressure_time_outside_range_seconds - 600.0).abs() < 1e-6);
    }

    #[test]
    fn validacoes_temporais_sao_aplicadas() {
        // Fim antes do início → presença ignorada.
        let presences = vec![presenca(
            "P1",
            "2025-01-15T09:00:00Z",
            Some("2025-01-15T08:00:00Z"),
        )];
        let env = vec![
            ambiente("E1", Some(800.0), None, None, None, "2025-01-15T08:00:00Z"),
            ambiente("E2", Some(800.0), None, None, None, "2025-01-15T09:00:00Z"),
        ];
        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());
        assert_eq!(r.analyzed_seconds, 0.0);

        // Timestamp inválido na presença → ignorada.
        let presences = vec![presenca("P1", "data-invalida", None)];
        let r = calculate_environmental_exposure(&presences, &env, &limiares(), 3600, Utc::now());
        assert_eq!(r.analyzed_seconds, 0.0);

        // Presença removida (deleted_at) → ignorada.
        let mut p = presenca("P1", "2025-01-15T08:00:00Z", Some("2025-01-15T09:00:00Z"));
        p.deleted_at = Some("2025-01-16T00:00:00Z".to_string());
        let r = calculate_environmental_exposure(&[p], &env, &limiares(), 3600, Utc::now());
        assert_eq!(r.analyzed_seconds, 0.0);

        // Leituras de outro módulo → não consideradas.
        let mut e1 = ambiente("E1", Some(800.0), None, None, None, "2025-01-15T08:00:00Z");
        e1.modulo_id = "OUTRO".to_string();
        let presences = vec![presenca(
            "P1",
            "2025-01-15T08:00:00Z",
            Some("2025-01-15T09:00:00Z"),
        )];
        let r = calculate_environmental_exposure(&presences, &[e1], &limiares(), 3600, Utc::now());
        assert_eq!(r.analyzed_seconds, 0.0);
    }

    #[test]
    fn limiares_invalidos_sao_rejeitados() {
        assert!(limiares().validar().is_ok());
        assert!(
            EnvironmentalThresholds {
                pressure_kpa_min: 105.0,
                ..limiares()
            }
            .validar()
            .is_err()
        );
        assert!(
            EnvironmentalThresholds {
                co2_ppm: f64::NAN,
                ..limiares()
            }
            .validar()
            .is_err()
        );
    }
}
