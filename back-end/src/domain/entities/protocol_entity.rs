/*
PROTOCOLO DE AVALIAÇÃO — ORGANIZA OS PARÂMETROS DETERMINÍSTICOS.

O protocolo NÃO define limites clínicos: todos os valores são parâmetros
estatísticos/operacionais CONFIGURÁVEIS (preenchidos a partir das variáveis
de ambiente em `config::settings`) e os limites de plausibilidade de coleta
por indicador continuam no `analysis::data_quality` (padrão do projeto).
*/

use serde::{Deserialize, Serialize};

/// Parâmetros do baseline individual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosBaseline {
    /// Fator k do filtro de atípicos (mediana ± k·MAD escalado).
    pub k_mad: f64,
    /// Mínimo de amostras que precisam restar após a filtragem.
    pub minimo_amostras: usize,
    /// Janela de amostras usada para calcular/atualizar o baseline (s).
    pub janela_calculo_segundos: i64,
    /// Idade máxima aceita (`atualizado_em`) antes de recalcular (s).
    /// `None` desativa a política de atualização por idade.
    pub idade_maxima_segundos: Option<i64>,
}

/// Parâmetros da variação em relação ao baseline individual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosVariacao {
    /// |z| acima disso = Atenção (política estatística, não clínica).
    pub z_atencao: f64,
    /// |z| acima disso = Crítico. Deve ser maior que `z_atencao`.
    pub z_critico: f64,
}

/// Parâmetros de qualidade dos dados (aplicados sobre os padrões
/// por indicador de `analysis::data_quality`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosQualidade {
    /// Quantidade mínima de amostras válidas na janela de análise.
    pub minimo_amostras: usize,
    /// Percentual mínimo de amostras válidas (0–100).
    pub minimo_percentual_valido: f64,
    /// Percentual máximo de amostras inválidas (0–100).
    pub maximo_percentual_invalido: f64,
    /// Percentual máximo de amostras ruidosas (0–100).
    pub maximo_percentual_ruido: f64,
    /// Lacuna temporal máxima aceita entre amostras (s).
    pub lacuna_maxima_segundos: i64,
}

/// Parâmetros das tendências.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosTendencia {
    /// Quantidade mínima de amostras para analisar tendência.
    pub minimo_amostras: usize,
    /// Período mínimo que a série precisa cobrir (s).
    pub periodo_minimo_segundos: i64,
    /// Inclinação mínima padrão em UNIDADE DO INDICADOR POR HORA, usada
    /// quando o baseline não fornece um desvio-padrão para derivar o limiar.
    pub inclinacao_minima_padrao: f64,
}

/// Parâmetros da persistência das alterações.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosPersistencia {
    /// Duração de cada janela de persistência (s).
    /// `<= 0` trata cada amostra como uma janela.
    pub janela_segundos: i64,
    /// Percentual mínimo de janelas alteradas (0–100).
    pub minimo_percentual: f64,
    /// Maior sequência consecutiva de janelas alteradas exigida.
    pub minimo_janelas_consecutivas: usize,
}

/// Limites de exposição ambiental (operacionais, configuráveis —
/// NÃO são limites de dose pessoal nem clínicos).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosExposicao {
    /// Limiar de CO₂ no módulo (ppm).
    pub co2_ppm: f64,
    /// Limiar de temperatura da cabine (°C).
    pub temperatura_c: f64,
    /// Faixa aceitável de pressão da cabine (kPa).
    pub pressao_kpa_min: f64,
    pub pressao_kpa_max: f64,
    /// Lacuna temporal máxima integrada nas leituras ambientais (s).
    pub lacuna_maxima_segundos: i64,
    /// Tempo mínimo acima dos limites para a exposição virar evidência (s).
    pub tempo_minimo_evidencia_segundos: f64,
}

/// Política dos alertas (escrita determinística, nunca dependente da IA).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosAlerta {
    /// Resolve automaticamente os alertas ativos quando a avaliação
    /// retorna a Normal / a condição deixa de ser justificada.
    pub resolucao_automatica: bool,
}

/// Protocolo completo de avaliação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocoloAvaliacao {
    pub id: String,
    pub nome: String,
    /// Versão do protocolo (muda quando os parâmetros mudam a política).
    pub versao: u32,
    pub baseline: ParametrosBaseline,
    pub variacao: ParametrosVariacao,
    pub qualidade: ParametrosQualidade,
    pub tendencia: ParametrosTendencia,
    pub persistencia: ParametrosPersistencia,
    pub exposicao: ParametrosExposicao,
    pub alerta: ParametrosAlerta,
    /// Regras determinísticas de agregação (domain::evaluation::rules).
    pub elevar_com_persistencia: bool,
    pub elevar_com_tendencia_adversa: bool,
    pub elevar_com_exposicao_ambiental: bool,
    pub elevar_com_multiplos_indicadores: bool,
    pub min_indicadores_multiplos: usize,
}

impl ProtocoloAvaliacao {
    pub fn validar(&self) -> Result<(), String> {
        if !self.baseline.k_mad.is_finite() || self.baseline.k_mad < 0.0 {
            return Err(format!(
                "baseline.k_mad deve ser finito e >= 0 (recebido {})",
                self.baseline.k_mad
            ));
        }
        if self.baseline.minimo_amostras == 0 {
            return Err("baseline.minimo_amostras deve ser >= 1".to_string());
        }
        if self.baseline.janela_calculo_segundos <= 0 {
            return Err("baseline.janela_calculo_segundos deve ser > 0".to_string());
        }
        if let Some(idade) = self.baseline.idade_maxima_segundos
            && idade <= 0
        {
            return Err("baseline.idade_maxima_segundos deve ser > 0".to_string());
        }
        if !self.variacao.z_atencao.is_finite() || self.variacao.z_atencao < 0.0 {
            return Err("variacao.z_atencao deve ser finito e >= 0".to_string());
        }
        if !self.variacao.z_critico.is_finite()
            || self.variacao.z_critico <= self.variacao.z_atencao
        {
            return Err(format!(
                "variacao.z_critico ({}) deve ser finito e > z_atencao ({})",
                self.variacao.z_critico, self.variacao.z_atencao
            ));
        }
        if !self.qualidade.minimo_percentual_valido.is_finite()
            || !(0.0..=100.0).contains(&self.qualidade.minimo_percentual_valido)
        {
            return Err("qualidade.minimo_percentual_valido deve estar em [0, 100]".to_string());
        }
        if !self.qualidade.maximo_percentual_invalido.is_finite()
            || !(0.0..=100.0).contains(&self.qualidade.maximo_percentual_invalido)
        {
            return Err("qualidade.maximo_percentual_invalido deve estar em [0, 100]".to_string());
        }
        if !self.qualidade.maximo_percentual_ruido.is_finite()
            || !(0.0..=100.0).contains(&self.qualidade.maximo_percentual_ruido)
        {
            return Err("qualidade.maximo_percentual_ruido deve estar em [0, 100]".to_string());
        }
        if self.qualidade.lacuna_maxima_segundos <= 0 {
            return Err("qualidade.lacuna_maxima_segundos deve ser > 0".to_string());
        }
        if !self.tendencia.inclinacao_minima_padrao.is_finite()
            || self.tendencia.inclinacao_minima_padrao < 0.0
        {
            return Err("tendencia.inclinacao_minima_padrao deve ser finita e >= 0".to_string());
        }
        if self.tendencia.periodo_minimo_segundos < 0 {
            return Err("tendencia.periodo_minimo_segundos deve ser >= 0".to_string());
        }
        if !self.persistencia.minimo_percentual.is_finite()
            || !(0.0..=100.0).contains(&self.persistencia.minimo_percentual)
        {
            return Err("persistencia.minimo_percentual deve estar em [0, 100]".to_string());
        }
        if self.persistencia.minimo_janelas_consecutivas == 0 {
            return Err("persistencia.minimo_janelas_consecutivas deve ser >= 1".to_string());
        }
        if !self.exposicao.co2_ppm.is_finite() || !self.exposicao.temperatura_c.is_finite() {
            return Err("limiares de CO₂ e temperatura devem ser finitos".to_string());
        }
        if !self.exposicao.pressao_kpa_min.is_finite()
            || !self.exposicao.pressao_kpa_max.is_finite()
            || self.exposicao.pressao_kpa_min >= self.exposicao.pressao_kpa_max
        {
            return Err("faixa de pressão deve ser finita e com min < max".to_string());
        }
        if self.exposicao.tempo_minimo_evidencia_segundos < 0.0 {
            return Err("exposicao.tempo_minimo_evidencia_segundos deve ser >= 0".to_string());
        }
        if self.min_indicadores_multiplos < 2 {
            return Err("min_indicadores_multiplos deve ser >= 2".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protocolo() -> ProtocoloAvaliacao {
        ProtocoloAvaliacao {
            id: "PROTOCOLO_PADRAO".into(),
            nome: "Protocolo padrão".into(),
            versao: 1,
            baseline: ParametrosBaseline {
                k_mad: 3.0,
                minimo_amostras: 20,
                janela_calculo_segundos: 3600,
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
                periodo_minimo_segundos: 600,
                inclinacao_minima_padrao: 0.5,
            },
            persistencia: ParametrosPersistencia {
                janela_segundos: 60,
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
        }
    }

    #[test]
    fn protocolo_padrao_e_valido() {
        assert!(protocolo().validar().is_ok());
    }

    #[test]
    fn protocolo_rejeita_parametros_incoerentes() {
        let mut p = protocolo();
        p.variacao.z_critico = p.variacao.z_atencao;
        assert!(p.validar().is_err());

        let mut p = protocolo();
        p.baseline.k_mad = -1.0;
        assert!(p.validar().is_err());

        let mut p = protocolo();
        p.exposicao.pressao_kpa_min = p.exposicao.pressao_kpa_max;
        assert!(p.validar().is_err());

        let mut p = protocolo();
        p.persistencia.minimo_percentual = 150.0;
        assert!(p.validar().is_err());

        let mut p = protocolo();
        p.baseline.minimo_amostras = 0;
        assert!(p.validar().is_err());
    }
}
