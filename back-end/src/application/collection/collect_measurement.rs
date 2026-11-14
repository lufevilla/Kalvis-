/*
CASO DE USO: COLETA DE MEDIÇÃO.

Recebe os dados produzidos pelo SensorAdapter (fisiológicos + ambientais),
VALIDA os dados básicos e PERSISTE nas tabelas correspondentes
(`telemetria_fisiologica`, `ambiente_nave`, `presenca_modulo`).

NÃO substitui nem reimplementa o adapter — apenas o consome.

Validações aplicadas (dados inválidos nunca chegam ao banco):
- identificadores e timestamps obrigatórios;
- valores não finitos (NaN/±inf) são convertidos em ausência (NULL) com
  aviso registrado — nunca viram zero;
- `origem_medicao` fora do CHECK do banco é normalizada para ausência com
  aviso (o adapter simulado usa "SIMULADO", que o banco não aceita).

Falha de persistência é devolvida como erro explícito; a presença no
módulo é registrada de forma idempotente (nunca duplica presença aberta).
*/

use chrono::DateTime;
use std::sync::Arc;

use crate::domain::entities::measurement_entity::{AmbienteNave, TelemetriaFisiologica};
use crate::domain::repositories::measurement_repository::MeasurementRepository;
use crate::shared::errors::{AppErro, AppResultado};

/// Origens aceitas pelo CHECK da tabela `telemetria_fisiologica`.
const ORIGENS_VALIDAS: [&str; 3] = ["WEARABLE", "MANUAL", "IMPORTADO"];

pub struct ColetaMedicao {
    repo: Arc<dyn MeasurementRepository>,
}

#[derive(Debug)]
pub struct ResultadoColeta {
    pub telemetria_id: String,
    pub ambiente_id: String,
    pub presenca_id: Option<String>,
    /// Problemas normalizados durante a validação (dados não finitos,
    /// origem desconhecida…). Nunca silenciados.
    pub avisos: Vec<String>,
}

impl ColetaMedicao {
    pub fn novo(repo: Arc<dyn MeasurementRepository>) -> Self {
        Self { repo }
    }

    /// Valida, persiste e devolve o resultado da coleta.
    pub async fn executar(
        &self,
        mut fisiologica: TelemetriaFisiologica,
        mut ambiental: AmbienteNave,
    ) -> AppResultado<ResultadoColeta> {
        let mut avisos = Vec::new();

        // ===== Validações básicas =====
        validar_identificador(&fisiologica.id, "telemetria.id")?;
        validar_identificador(&fisiologica.astronauta_id, "telemetria.astronauta_id")?;
        validar_identificador(&ambiental.id, "ambiente.id")?;
        validar_identificador(&ambiental.modulo_id, "ambiente.modulo_id")?;

        if DateTime::parse_from_rfc3339(&fisiologica.timestamp_utc).is_err() {
            return Err(AppErro::DadosInvalidos(format!(
                "timestamp da telemetria não é ISO 8601 válido: '{}'",
                fisiologica.timestamp_utc
            )));
        }
        if DateTime::parse_from_rfc3339(&ambiental.timestamp_utc).is_err() {
            return Err(AppErro::DadosInvalidos(format!(
                "timestamp do ambiente não é ISO 8601 válido: '{}'",
                ambiental.timestamp_utc
            )));
        }

        // ===== Sanitização dos valores fisiológicos =====
        for (nome, valor) in [
            (
                "frequencia_cardiaca_bpm",
                &mut fisiologica.frequencia_cardiaca_bpm,
            ),
            ("spo2_percentual", &mut fisiologica.spo2_percentual),
            (
                "temperatura_corporal_c",
                &mut fisiologica.temperatura_corporal_c,
            ),
            ("temperatura_pele_c", &mut fisiologica.temperatura_pele_c),
            (
                "taxa_respiratoria_rpm",
                &mut fisiologica.taxa_respiratoria_rpm,
            ),
            ("hrv_ms", &mut fisiologica.hrv_ms),
            (
                "pressao_sistolica_mmhg",
                &mut fisiologica.pressao_sistolica_mmhg,
            ),
            (
                "pressao_diastolica_mmhg",
                &mut fisiologica.pressao_diastolica_mmhg,
            ),
            ("glicose_mg_dl", &mut fisiologica.glicose_mg_dl),
        ] {
            if let Some(v) = *valor
                && !v.is_finite()
            {
                avisos.push(format!(
                    "telemetria: valor não finito em {nome} registrado como ausência"
                ));
                *valor = None;
            }
        }

        // ===== Sanitização dos valores ambientais =====
        for (nome, valor) in [
            ("co2_ppm", &mut ambiental.co2_ppm),
            ("radiacao_msv_hora", &mut ambiental.radiacao_msv_hora),
            ("temperatura_cabine_c", &mut ambiental.temperatura_cabine_c),
            ("pressao_kpa", &mut ambiental.pressao_kpa),
        ] {
            if let Some(v) = *valor
                && !v.is_finite()
            {
                avisos.push(format!(
                    "ambiente: valor não finito em {nome} registrado como ausência"
                ));
                *valor = None;
            }
        }

        // `origem_medicao` fora do CHECK do banco → ausência com aviso.
        if let Some(origem) = &fisiologica.origem_medicao
            && !ORIGENS_VALIDAS.contains(&origem.as_str())
        {
            avisos.push(format!(
                "telemetria: origem_medicao '{origem}' não reconhecida; \
                 registrada como ausência"
            ));
            fisiologica.origem_medicao = None;
        }

        // ===== Persistência =====
        let telemetria_id = fisiologica.id.clone();
        self.repo
            .save_telemetria(&fisiologica)
            .await
            .map_err(AppErro::Repositorio)?;

        let ambiente_id = ambiental.id.clone();
        self.repo
            .save_ambiente(&ambiental)
            .await
            .map_err(AppErro::Repositorio)?;

        // Presença no módulo: idempotente; falha não perde a coleta.
        let presenca_id = match self
            .repo
            .obter_ou_iniciar_presenca_aberta(
                &fisiologica.astronauta_id,
                &ambiental.modulo_id,
                &ambiental.timestamp_utc,
            )
            .await
        {
            Ok(presenca) => Some(presenca.id),
            Err(erro) => {
                avisos.push(format!("presença no módulo não registrada: {erro}"));
                None
            }
        };

        Ok(ResultadoColeta {
            telemetria_id,
            ambiente_id,
            presenca_id,
            avisos,
        })
    }
}

fn validar_identificador(valor: &str, campo: &str) -> AppResultado<()> {
    if valor.trim().is_empty() {
        return Err(AppErro::DadosInvalidos(format!(
            "{campo} não pode ser vazio"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::infrastructure::database::repositories::measurement_repository::SqliteMeasurementRepository;

    fn ts(segundos: i64) -> String {
        Utc.timestamp_opt(1_700_000_000 + segundos, 0)
            .unwrap()
            .to_rfc3339()
    }

    async fn repo_com_base() -> (Arc<dyn MeasurementRepository>, SqlitePool) {
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
            "INSERT INTO astronautas (id, nome, data_nascimento) VALUES ('A1', 'Kalvis', '1990-05-01')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO missoes (id, nome_missao) VALUES ('M1', 'Artemis III')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO modulos_nave (id, missao_id, nome) VALUES ('MOD1', 'M1', 'Habitat')",
        )
        .execute(&pool)
        .await
        .unwrap();

        (
            Arc::new(SqliteMeasurementRepository::new(pool.clone())),
            pool,
        )
    }

    fn telemetria(id: &str, astro: &str, ts_texto: &str) -> TelemetriaFisiologica {
        TelemetriaFisiologica {
            id: id.to_string(),
            astronauta_id: astro.to_string(),
            missao_id: Some("M1".to_string()),
            timestamp_utc: ts_texto.to_string(),
            frequencia_cardiaca_bpm: Some(72.0),
            spo2_percentual: Some(98.0),
            temperatura_corporal_c: Some(36.8),
            temperatura_pele_c: Some(33.5),
            taxa_respiratoria_rpm: Some(14.0),
            hrv_ms: Some(55.0),
            pressao_sistolica_mmhg: Some(118.0),
            pressao_diastolica_mmhg: Some(76.0),
            glicose_mg_dl: Some(95.0),
            origem_medicao: Some("WEARABLE".to_string()),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn ambiente(id: &str, ts_texto: &str) -> AmbienteNave {
        AmbienteNave {
            id: id.to_string(),
            modulo_id: "MOD1".to_string(),
            timestamp_utc: ts_texto.to_string(),
            co2_ppm: Some(800.0),
            radiacao_msv_hora: Some(0.005),
            temperatura_cabine_c: Some(21.0),
            pressao_kpa: Some(101.3),
            updated_at: None,
            deleted_at: None,
        }
    }

    #[tokio::test]
    async fn coleta_valida_e_persistida() {
        let (repo, pool) = repo_com_base().await;
        let caso = ColetaMedicao::novo(repo.clone());

        let resultado = caso
            .executar(telemetria("T1", "A1", &ts(0)), ambiente("E1", &ts(0)))
            .await
            .expect("coleta válida");

        assert_eq!(resultado.telemetria_id, "T1");
        assert_eq!(resultado.ambiente_id, "E1");
        assert!(resultado.avisos.is_empty());
        assert!(
            resultado.presenca_id.is_some(),
            "presença aberta registrada"
        );

        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM telemetria_fisiologica WHERE id = 'T1'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM ambiente_nave WHERE id = 'E1'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );

        // Segunda coleta NÃO duplica a presença aberta.
        let segunda = caso
            .executar(telemetria("T2", "A1", &ts(5)), ambiente("E2", &ts(5)))
            .await
            .unwrap();
        assert_eq!(segunda.presenca_id, resultado.presenca_id);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM presenca_modulo")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn dados_invalidos_sao_rejeitados_ou_normalizados() {
        let (repo, pool) = repo_com_base().await;
        let caso = ColetaMedicao::novo(repo.clone());

        // Timestamp inválido → erro explícito.
        let erro = caso
            .executar(telemetria("T1", "A1", "nao-e-data"), ambiente("E1", &ts(0)))
            .await
            .unwrap_err();
        assert!(matches!(erro, AppErro::DadosInvalidos(_)));

        // Astronauta vazio → erro explícito.
        let erro = caso
            .executar(telemetria("T1", " ", &ts(0)), ambiente("E1", &ts(0)))
            .await
            .unwrap_err();
        assert!(matches!(erro, AppErro::DadosInvalidos(_)));

        // Valores não finitos → registrados como ausência (NUNCA zero).
        let mut t = telemetria("T2", "A1", &ts(0));
        t.frequencia_cardiaca_bpm = Some(f64::NAN);
        t.spo2_percentual = Some(f64::INFINITY);
        let mut e = ambiente("E2", &ts(0));
        e.co2_ppm = Some(f64::NAN);

        let resultado = caso.executar(t, e).await.expect("coleta sanitizada");
        assert_eq!(resultado.avisos.len(), 3, "um aviso por valor não finito");

        let (fc, spo2): (Option<f64>, Option<f64>) = sqlx::query_as(
            "SELECT frequencia_cardiaca_bpm, spo2_percentual FROM telemetria_fisiologica \
             WHERE id = 'T2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(fc, None, "NaN → ausência, não zero");
        assert_eq!(spo2, None);
        let co2: Option<f64> =
            sqlx::query_scalar("SELECT co2_ppm FROM ambiente_nave WHERE id = 'E2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(co2, None);
    }

    #[tokio::test]
    async fn origem_desconhecida_e_normalizada_com_aviso() {
        let (repo, pool) = repo_com_base().await;
        let caso = ColetaMedicao::novo(repo.clone());

        let mut t = telemetria("T1", "A1", &ts(0));
        t.origem_medicao = Some("SIMULADO".to_string());
        let resultado = caso.executar(t, ambiente("E1", &ts(0))).await.unwrap();

        assert!(
            resultado
                .avisos
                .iter()
                .any(|a| a.contains("origem_medicao 'SIMULADO'"))
        );
        let origem: Option<String> =
            sqlx::query_scalar("SELECT origem_medicao FROM telemetria_fisiologica WHERE id = 'T1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(origem, None, "CHECK do banco aceita NULL");
    }

    #[tokio::test]
    async fn astronauta_inexistente_falha_com_erro_de_repositorio() {
        let (repo, _pool) = repo_com_base().await;
        let caso = ColetaMedicao::novo(repo);

        let erro = caso
            .executar(telemetria("T1", "FANTASMA", &ts(0)), ambiente("E1", &ts(0)))
            .await
            .unwrap_err();
        assert!(matches!(erro, AppErro::Repositorio(_)), "FK violada");
    }
}
