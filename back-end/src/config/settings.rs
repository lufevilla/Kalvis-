/*
CONFIGURAÇÃO CENTRAL DO SISTEMA (dotenvy + variáveis de ambiente).

Todas as constantes operacionais do projeto passam por aqui — nenhum
número mágico espalhado pelos módulos. Valores numéricos são validados
(finitos, não negativos, faixas coerentes) e cada variável tem valor
padrão seguro, documentado abaixo.

Variáveis de ambiente (todas opcionais; padrão entre parênteses):

Banco e coleta
  DATABASE_PATH                        caminho do SQLite ("data_base/kalvis.db")
  COLETA_INTERVALO_SEGUNDOS            intervalo do ciclo de coleta (5)
  SENSOR_ASTRONAUTA_ID                 astronauta simulado ("AST-001")
  SENSOR_MISSAO_ID                     missão simulada ("MISSION-001")
  SENSOR_MODULO_ID                     módulo simulado ("MODULE-001")

Análise
  ANALISE_JANELA_SEGUNDOS              janela de análise da avaliação (300)
  ANALISE_INTERVALO_AVALIACAO_SEGUNDOS intervalo entre avaliações (60)
  ANALISE_FASE_MISSAO                  fase assumida nas avaliações ("EM_VOO")
  AVALIACOES_ANTERIORES_LIMITE         avaliações anteriores no contexto (5)

Baseline
  BASELINE_K_MAD                       fator do filtro de atípicos (3.0)
  BASELINE_MINIMO_AMOSTRAS             mínimo após a filtragem (20)
  BASELINE_JANELA_SEGUNDOS             janela de cálculo (3600)
  BASELINE_IDADE_MAXIMA_SEGUNDOS       política de atualização (21600; 0 = sem política)

Variação
  VARIACAO_Z_ATENCAO                   |z| mínimo para Atenção (2.0)
  VARIACAO_Z_CRITICO                   |z| mínimo para Crítico (3.0)

Qualidade de dados
  QUALIDADE_MINIMO_AMOSTRAS            (2)
  QUALIDADE_MINIMO_PERCENTUAL_VALIDO   (80)
  QUALIDADE_MAXIMO_PERCENTUAL_INVALIDO (5)
  QUALIDADE_MAXIMO_PERCENTUAL_RUIDO    (5)
  QUALIDADE_LACUNA_MAXIMA_SEGUNDOS     (300)

Tendência
  TENDENCIA_MINIMO_AMOSTRAS            (3)
  TENDENCIA_PERIODO_MINIMO_SEGUNDOS    (600)
  TENDENCIA_INCLINACAO_MINIMA_PADRAO   unidade do indicador por hora (0.5)

Persistência
  PERSISTENCIA_JANELA_SEGUNDOS         (60)
  PERSISTENCIA_MINIMO_PERCENTUAL       (50)
  PERSISTENCIA_MINIMO_JANELAS_CONSECUTIVAS (2)

Exposição ambiental (limiares operacionais, não clínicos)
  EXPOSICAO_CO2_PPM                    (1000)
  EXPOSICAO_TEMPERATURA_C              (24)
  EXPOSICAO_PRESSAO_KPA_MIN            (95)
  EXPOSICAO_PRESSAO_KPA_MAX            (105)
  EXPOSICAO_LACUNA_MAXIMA_SEGUNDOS     (300)
  EXPOSICAO_TEMPO_MINIMO_EVIDENCIA_SEGUNDOS (60)

Alertas e regras
  ALERTA_RESOLUCAO_AUTOMATICA          (true)
  REGRAS_ELEVAR_COM_PERSISTENCIA       (true)
  REGRAS_ELEVAR_COM_TENDENCIA_ADVERSA  (true)
  REGRAS_ELEVAR_COM_EXPOSICAO_AMBIENTAL (true)
  REGRAS_ELEVAR_COM_MULTIPLos_INDICADORES (true)
  REGRAS_MIN_INDICADORES_MULTIPLOS     (2)

IA (Ollama — copiloto explicativo, opcional)
  OLLAMA_ENABLED                       (true)
  OLLAMA_URL                           ("http://127.0.0.1:11434")
  OLLAMA_MODEL                         ("llama3.2")
  OLLAMA_TIMEOUT_SECS                  (30)
*/

use std::time::Duration;

use crate::analysis::repository::FaseMissao;
use crate::domain::entities::protocol_entity::{
    ParametrosAlerta, ParametrosBaseline, ParametrosExposicao, ParametrosPersistencia,
    ParametrosQualidade, ParametrosTendencia, ParametrosVariacao, ProtocoloAvaliacao,
};
use crate::infrastructure::ai::client::OllamaClient;

// ============================================================
// GRUPOS DE CONFIGURAÇÃO
// ============================================================

#[derive(Debug, Clone)]
pub struct BancoConfig {
    /// Caminho do arquivo SQLite.
    pub caminho: String,
}

#[derive(Debug, Clone)]
pub struct ColetaConfig {
    /// Intervalo do ciclo de coleta. O padrão do projeto é 5 segundos.
    pub intervalo: Duration,
    pub astronauta_id: String,
    /// `None` = coleta sem missão associada (o módulo ambiental exige missão).
    pub missao_id: Option<String>,
    pub modulo_id: String,
}

#[derive(Debug, Clone)]
pub struct AnaliseConfig {
    /// Janela de dados fisiológicos analisada em cada avaliação.
    pub janela_analise: Duration,
    /// Política de execução: avaliações no máximo a cada intervalo.
    pub intervalo_avaliacao: Duration,
    /// Quantas avaliações anteriores entram no contexto.
    pub limite_avaliacoes_anteriores: i64,
    /// Fase da missão assumida nas avaliações (padrão: EM_VOO).
    pub fase_missao: FaseMissao,
}

#[derive(Debug, Clone)]
pub struct RegrasConfig {
    pub elevar_com_persistencia: bool,
    pub elevar_com_tendencia_adversa: bool,
    pub elevar_com_exposicao_ambiental: bool,
    pub elevar_com_multiplos_indicadores: bool,
    pub min_indicadores_multiplos: usize,
}

#[derive(Debug, Clone)]
pub struct IaConfig {
    /// `None` = IA desativada (o sistema funciona normalmente sem ela).
    pub url: Option<String>,
    pub modelo: String,
    pub timeout: Duration,
}

// ============================================================
// SETTINGS
// ============================================================

#[derive(Debug, Clone)]
pub struct Settings {
    pub banco: BancoConfig,
    pub coleta: ColetaConfig,
    pub analise: AnaliseConfig,
    pub baseline: ParametrosBaseline,
    pub variacao: ParametrosVariacao,
    pub qualidade: ParametrosQualidade,
    pub tendencia: ParametrosTendencia,
    pub persistencia: ParametrosPersistencia,
    pub exposicao: ParametrosExposicao,
    pub alerta: ParametrosAlerta,
    pub regras: RegrasConfig,
    pub ia: IaConfig,
}

impl Settings {
    /// Carrega o `.env` (se existir) e lê as configurações.
    /// Falha de parsing/validação é erro explícito — nunca valor silencioso.
    pub fn carregar() -> Result<Self, String> {
        // Ignora "arquivo não encontrado": as variáveis podem vir do ambiente.
        let _ = dotenvy::dotenv();
        Self::from_env()
    }

    /// Lê as configurações do ambiente (sem tocar no `.env`).
    pub fn from_env() -> Result<Self, String> {
        let intervalo_coleta = env_u64("COLETA_INTERVALO_SEGUNDOS", 5)?;
        if intervalo_coleta == 0 {
            return Err("COLETA_INTERVALO_SEGUNDOS deve ser >= 1".to_string());
        }

        let janela_analise = env_i64("ANALISE_JANELA_SEGUNDOS", 300)?;
        if janela_analise <= 0 {
            return Err("ANALISE_JANELA_SEGUNDOS deve ser > 0".to_string());
        }
        let intervalo_avaliacao = env_i64("ANALISE_INTERVALO_AVALIACAO_SEGUNDOS", 60)?;
        if intervalo_avaliacao <= 0 {
            return Err("ANALISE_INTERVALO_AVALIACAO_SEGUNDOS deve ser > 0".to_string());
        }

        let idade_maxima = env_i64("BASELINE_IDADE_MAXIMA_SEGUNDOS", 21_600)?;
        if idade_maxima < 0 {
            return Err(
                "BASELINE_IDADE_MAXIMA_SEGUNDOS deve ser >= 0 (0 = sem política)".to_string(),
            );
        }

        let missao_id = std::env::var("SENSOR_MISSAO_ID")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        let modulo_id = env_string("SENSOR_MODULO_ID", "MODULE-001");
        if modulo_id.trim().is_empty() {
            return Err("SENSOR_MODULO_ID não pode ser vazio".to_string());
        }

        let co2 = env_f64("EXPOSICAO_CO2_PPM", 1000.0)?;
        let temperatura = env_f64("EXPOSICAO_TEMPERATURA_C", 24.0)?;
        let pressao_min = env_f64("EXPOSICAO_PRESSAO_KPA_MIN", 95.0)?;
        let pressao_max = env_f64("EXPOSICAO_PRESSAO_KPA_MAX", 105.0)?;
        if pressao_min >= pressao_max {
            return Err(format!(
                "EXPOSICAO_PRESSAO_KPA_MIN ({pressao_min}) deve ser < EXPOSICAO_PRESSAO_KPA_MAX \
                 ({pressao_max})"
            ));
        }

        let z_atencao = env_f64("VARIACAO_Z_ATENCAO", 2.0)?;
        let z_critico = env_f64("VARIACAO_Z_CRITICO", 3.0)?;
        if !z_atencao.is_finite() || z_atencao < 0.0 {
            return Err("VARIACAO_Z_ATENCAO deve ser finito e >= 0".to_string());
        }
        if !z_critico.is_finite() || z_critico <= z_atencao {
            return Err(format!(
                "VARIACAO_Z_CRITICO ({z_critico}) deve ser finito e > VARIACAO_Z_ATENCAO \
                 ({z_atencao})"
            ));
        }

        let ia_ativa = env_bool("OLLAMA_ENABLED", true)?;
        let ia_url = if ia_ativa {
            Some(env_string("OLLAMA_URL", "http://127.0.0.1:11434"))
        } else {
            None
        };
        let ia_timeout = env_u64("OLLAMA_TIMEOUT_SECS", 30)?;
        if ia_timeout == 0 {
            return Err("OLLAMA_TIMEOUT_SECS deve ser >= 1".to_string());
        }

        let settings = Self {
            banco: BancoConfig {
                caminho: env_string("DATABASE_PATH", "data_base/kalvis.db"),
            },
            coleta: ColetaConfig {
                intervalo: Duration::from_secs(intervalo_coleta),
                astronauta_id: env_string("SENSOR_ASTRONAUTA_ID", "AST-001"),
                missao_id,
                modulo_id,
            },
            analise: AnaliseConfig {
                janela_analise: Duration::from_secs(janela_analise as u64),
                intervalo_avaliacao: Duration::from_secs(intervalo_avaliacao as u64),
                limite_avaliacoes_anteriores: env_i64("AVALIACOES_ANTERIORES_LIMITE", 5)?,
                fase_missao: {
                    let nome = env_string("ANALISE_FASE_MISSAO", "EM_VOO");
                    FaseMissao::from_nome(&nome).ok_or_else(|| {
                        format!(
                            "ANALISE_FASE_MISSAO deve ser PRE_VOO, EM_VOO ou POS_VOO \
                             (recebido '{nome}')"
                        )
                    })?
                },
            },
            baseline: ParametrosBaseline {
                k_mad: env_f64("BASELINE_K_MAD", 3.0)?,
                minimo_amostras: env_usize("BASELINE_MINIMO_AMOSTRAS", 20)?,
                janela_calculo_segundos: env_i64("BASELINE_JANELA_SEGUNDOS", 3600)?,
                idade_maxima_segundos: (idade_maxima > 0).then_some(idade_maxima),
            },
            variacao: ParametrosVariacao {
                z_atencao,
                z_critico,
            },
            qualidade: ParametrosQualidade {
                minimo_amostras: env_usize("QUALIDADE_MINIMO_AMOSTRAS", 2)?,
                minimo_percentual_valido: env_f64("QUALIDADE_MINIMO_PERCENTUAL_VALIDO", 80.0)?,
                maximo_percentual_invalido: env_f64("QUALIDADE_MAXIMO_PERCENTUAL_INVALIDO", 5.0)?,
                maximo_percentual_ruido: env_f64("QUALIDADE_MAXIMO_PERCENTUAL_RUIDO", 5.0)?,
                lacuna_maxima_segundos: env_i64("QUALIDADE_LACUNA_MAXIMA_SEGUNDOS", 300)?,
            },
            tendencia: ParametrosTendencia {
                minimo_amostras: env_usize("TENDENCIA_MINIMO_AMOSTRAS", 3)?,
                periodo_minimo_segundos: env_i64("TENDENCIA_PERIODO_MINIMO_SEGUNDOS", 600)?,
                inclinacao_minima_padrao: env_f64("TENDENCIA_INCLINACAO_MINIMA_PADRAO", 0.5)?,
            },
            persistencia: ParametrosPersistencia {
                janela_segundos: env_i64("PERSISTENCIA_JANELA_SEGUNDOS", 60)?,
                minimo_percentual: env_f64("PERSISTENCIA_MINIMO_PERCENTUAL", 50.0)?,
                minimo_janelas_consecutivas: env_usize(
                    "PERSISTENCIA_MINIMO_JANELAS_CONSECUTIVAS",
                    2,
                )?,
            },
            exposicao: ParametrosExposicao {
                co2_ppm: co2,
                temperatura_c: temperatura,
                pressao_kpa_min: pressao_min,
                pressao_kpa_max: pressao_max,
                lacuna_maxima_segundos: env_i64("EXPOSICAO_LACUNA_MAXIMA_SEGUNDOS", 300)?,
                tempo_minimo_evidencia_segundos: env_f64(
                    "EXPOSICAO_TEMPO_MINIMO_EVIDENCIA_SEGUNDOS",
                    60.0,
                )?,
            },
            alerta: ParametrosAlerta {
                resolucao_automatica: env_bool("ALERTA_RESOLUCAO_AUTOMATICA", true)?,
            },
            regras: RegrasConfig {
                elevar_com_persistencia: env_bool("REGRAS_ELEVAR_COM_PERSISTENCIA", true)?,
                elevar_com_tendencia_adversa: env_bool(
                    "REGRAS_ELEVAR_COM_TENDENCIA_ADVERSA",
                    true,
                )?,
                elevar_com_exposicao_ambiental: env_bool(
                    "REGRAS_ELEVAR_COM_EXPOSICAO_AMBIENTAL",
                    true,
                )?,
                elevar_com_multiplos_indicadores: env_bool(
                    "REGRAS_ELEVAR_COM_MULTIPLos_INDICADORES",
                    true,
                )?,
                min_indicadores_multiplos: env_usize("REGRAS_MIN_INDICADORES_MULTIPLOS", 2)?,
            },
            ia: IaConfig {
                url: ia_url,
                modelo: env_string("OLLAMA_MODEL", "llama3.2"),
                timeout: Duration::from_secs(ia_timeout),
            },
        };

        settings.validar()?;
        Ok(settings)
    }

    /// Validação coerente de todos os grupos (reutiliza a do protocolo).
    pub fn validar(&self) -> Result<(), String> {
        self.protocolo().validar()?;
        if self.analise.limite_avaliacoes_anteriores < 0 {
            return Err("AVALIACOES_ANTERIORES_LIMITE deve ser >= 0".to_string());
        }
        if self.coleta.astronauta_id.trim().is_empty() {
            return Err("SENSOR_ASTRONAUTA_ID não pode ser vazio".to_string());
        }
        if self.banco.caminho.trim().is_empty() {
            return Err("DATABASE_PATH não pode ser vazio".to_string());
        }
        Ok(())
    }

    /// Protocolo de avaliação derivado das configurações.
    pub fn protocolo(&self) -> ProtocoloAvaliacao {
        ProtocoloAvaliacao {
            id: "PROTOCOLO_PADRAO".to_string(),
            nome: "Protocolo padrão do Space Health".to_string(),
            versao: 1,
            baseline: self.baseline.clone(),
            variacao: self.variacao.clone(),
            qualidade: self.qualidade.clone(),
            tendencia: self.tendencia.clone(),
            persistencia: self.persistencia.clone(),
            exposicao: self.exposicao.clone(),
            alerta: self.alerta.clone(),
            elevar_com_persistencia: self.regras.elevar_com_persistencia,
            elevar_com_tendencia_adversa: self.regras.elevar_com_tendencia_adversa,
            elevar_com_exposicao_ambiental: self.regras.elevar_com_exposicao_ambiental,
            elevar_com_multiplos_indicadores: self.regras.elevar_com_multiplos_indicadores,
            min_indicadores_multiplos: self.regras.min_indicadores_multiplos,
        }
    }

    /// Cliente opcional do Ollama. `Ok(None)` = IA desativada por
    /// configuração; o restante do sistema funciona normalmente.
    pub fn cliente_ia(&self) -> Result<Option<OllamaClient>, String> {
        match &self.ia.url {
            None => Ok(None),
            Some(url) => {
                OllamaClient::novo(url, &self.ia.modelo, self.ia.timeout.as_secs()).map(Some)
            }
        }
    }
}

// ============================================================
// LEITURA E VALIDAÇÃO DE VARIÁVEIS
// ============================================================

fn env_string(nome: &str, padrao: &str) -> String {
    std::env::var(nome)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| padrao.to_string())
}

fn env_f64(nome: &str, padrao: f64) -> Result<f64, String> {
    match std::env::var(nome) {
        Ok(valor) => {
            let v: f64 = valor
                .trim()
                .parse()
                .map_err(|_| format!("{nome} não é um número válido ('{valor}')"))?;
            if !v.is_finite() {
                return Err(format!("{nome} deve ser finito (recebido '{valor}')"));
            }
            Ok(v)
        }
        Err(_) => Ok(padrao),
    }
}

fn env_i64(nome: &str, padrao: i64) -> Result<i64, String> {
    match std::env::var(nome) {
        Ok(valor) => valor
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("{nome} não é um inteiro válido ('{valor}')")),
        Err(_) => Ok(padrao),
    }
}

fn env_u64(nome: &str, padrao: u64) -> Result<u64, String> {
    match std::env::var(nome) {
        Ok(valor) => {
            let v: i64 = valor
                .trim()
                .parse()
                .map_err(|_| format!("{nome} não é um inteiro válido ('{valor}')"))?;
            if v < 0 {
                return Err(format!("{nome} não pode ser negativo (recebido '{valor}')"));
            }
            Ok(v as u64)
        }
        Err(_) => Ok(padrao),
    }
}

fn env_usize(nome: &str, padrao: usize) -> Result<usize, String> {
    env_u64(nome, padrao as u64).map(|v| v as usize)
}

fn env_bool(nome: &str, padrao: bool) -> Result<bool, String> {
    match std::env::var(nome) {
        Ok(valor) => match valor.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "sim" | "yes" | "on" => Ok(true),
            "0" | "false" | "nao" | "não" | "no" | "off" => Ok(false),
            _ => Err(format!(
                "{nome} deve ser um booleano (1/true/on ou 0/false/off), recebido '{valor}'"
            )),
        },
        Err(_) => Ok(padrao),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limpar(chave: &str) {
        // Seguro em contexto de teste single-threaded por teste.
        unsafe { std::env::remove_var(chave) };
    }

    fn definir(chave: &str, valor: &str) {
        unsafe { std::env::set_var(chave, valor) };
    }

    #[test]
    fn padroes_sao_validos() {
        // Os testes rodam no mesmo processo; remove as chaves relevantes.
        let chaves = [
            "DATABASE_PATH",
            "COLETA_INTERVALO_SEGUNDOS",
            "SENSOR_ASTRONAUTA_ID",
            "SENSOR_MISSAO_ID",
            "SENSOR_MODULO_ID",
            "ANALISE_JANELA_SEGUNDOS",
            "ANALISE_INTERVALO_AVALIACAO_SEGUNDOS",
            "ANALISE_FASE_MISSAO",
            "AVALIACOES_ANTERIORES_LIMITE",
            "BASELINE_K_MAD",
            "BASELINE_MINIMO_AMOSTRAS",
            "BASELINE_JANELA_SEGUNDOS",
            "BASELINE_IDADE_MAXIMA_SEGUNDOS",
            "VARIACAO_Z_ATENCAO",
            "VARIACAO_Z_CRITICO",
            "QUALIDADE_MINIMO_AMOSTRAS",
            "QUALIDADE_MINIMO_PERCENTUAL_VALIDO",
            "QUALIDADE_MAXIMO_PERCENTUAL_INVALIDO",
            "QUALIDADE_MAXIMO_PERCENTUAL_RUIDO",
            "QUALIDADE_LACUNA_MAXIMA_SEGUNDOS",
            "TENDENCIA_MINIMO_AMOSTRAS",
            "TENDENCIA_PERIODO_MINIMO_SEGUNDOS",
            "TENDENCIA_INCLINACAO_MINIMA_PADRAO",
            "PERSISTENCIA_JANELA_SEGUNDOS",
            "PERSISTENCIA_MINIMO_PERCENTUAL",
            "PERSISTENCIA_MINIMO_JANELAS_CONSECUTIVAS",
            "EXPOSICAO_CO2_PPM",
            "EXPOSICAO_TEMPERATURA_C",
            "EXPOSICAO_PRESSAO_KPA_MIN",
            "EXPOSICAO_PRESSAO_KPA_MAX",
            "EXPOSICAO_LACUNA_MAXIMA_SEGUNDOS",
            "EXPOSICAO_TEMPO_MINIMO_EVIDENCIA_SEGUNDOS",
            "ALERTA_RESOLUCAO_AUTOMATICA",
            "REGRAS_ELEVAR_COM_PERSISTENCIA",
            "REGRAS_ELEVAR_COM_TENDENCIA_ADVERSA",
            "REGRAS_ELEVAR_COM_EXPOSICAO_AMBIENTAL",
            "REGRAS_ELEVAR_COM_MULTIPLos_INDICADORES",
            "REGRAS_MIN_INDICADORES_MULTIPLOS",
            "OLLAMA_ENABLED",
            "OLLAMA_URL",
            "OLLAMA_MODEL",
            "OLLAMA_TIMEOUT_SECS",
        ];
        for c in chaves {
            limpar(c);
        }

        let s = Settings::from_env().expect("padrões válidos");
        // Intervalo de coleta padrão do projeto: 5 segundos.
        assert_eq!(s.coleta.intervalo, Duration::from_secs(5));
        assert_eq!(s.baseline.minimo_amostras, 20);
        assert_eq!(s.variacao.z_atencao, 2.0);
        assert_eq!(s.variacao.z_critico, 3.0);
        assert!(s.ia.url.is_some());
        assert!(s.protocolo().validar().is_ok());
        assert!(s.cliente_ia().is_ok());
    }

    #[test]
    fn valores_invalidos_sao_rejeitados() {
        definir("COLETA_INTERVALO_SEGUNDOS", "0");
        assert!(Settings::from_env().is_err());
        definir("COLETA_INTERVALO_SEGUNDOS", "-5");
        assert!(Settings::from_env().is_err());
        limpar("COLETA_INTERVALO_SEGUNDOS");

        definir("VARIACAO_Z_CRITICO", "nan");
        assert!(Settings::from_env().is_err());
        limpar("VARIACAO_Z_CRITICO");

        definir("VARIACAO_Z_ATENCAO", "5");
        assert!(Settings::from_env().is_err(), "z_critico <= z_atencao");
        limpar("VARIACAO_Z_ATENCAO");

        definir("EXPOSICAO_PRESSAO_KPA_MAX", "90");
        assert!(Settings::from_env().is_err(), "faixa de pressão invertida");
        limpar("EXPOSICAO_PRESSAO_KPA_MAX");

        definir("OLLAMA_ENABLED", "talvez");
        assert!(Settings::from_env().is_err());
        limpar("OLLAMA_ENABLED");

        definir("BASELINE_IDADE_MAXIMA_SEGUNDOS", "-1");
        assert!(Settings::from_env().is_err());
        limpar("BASELINE_IDADE_MAXIMA_SEGUNDOS");
    }

    #[test]
    fn ia_desativada_nao_gera_cliente() {
        limpar("OLLAMA_ENABLED");
        definir("OLLAMA_ENABLED", "false");
        let s = Settings::from_env().expect("config válida");
        assert!(s.ia.url.is_none());
        assert!(matches!(s.cliente_ia(), Ok(None)));
        limpar("OLLAMA_ENABLED");
    }

    #[test]
    fn idade_zero_desativa_a_politica_do_baseline() {
        definir("BASELINE_IDADE_MAXIMA_SEGUNDOS", "0");
        let s = Settings::from_env().expect("config válida");
        assert_eq!(s.baseline.idade_maxima_segundos, None);
        limpar("BASELINE_IDADE_MAXIMA_SEGUNDOS");
    }

    #[test]
    fn sensor_sem_missao_e_aceito() {
        limpar("SENSOR_MISSAO_ID");
        let s = Settings::from_env().expect("config válida");
        assert_eq!(s.coleta.missao_id, None);
    }
}
