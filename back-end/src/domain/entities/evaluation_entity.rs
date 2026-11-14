/*
ENTIDADE DE AVALIAÇÃO DE SAÚDE.

Compatível com a migration 006 (tabela `avaliacoes_saude`):
  id, astronauta_id, missao_id, timestamp_utc, nivel_risco,
  resumo, indicadores_analisados_json, fatores_contextuais_json,
  explicacao, orientacao, updated_at, deleted_at.

`nivel_risco` é o nível operacional determinístico (RiskLevel).
`indicadores_analisados_json` guarda as EvidenciaIndicador (regras.rs).
`fatores_contextuais_json` guarda o FatoresContextuais (regras.rs).
`explicacao` / `orientacao` são preenchidos opcionalmente pela IA copiloto
explicativa — nunca alteram `nivel_risco` nem as evidências.
*/

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::domain::evaluation::rules::{EvidenciaIndicador, FatoresContextuais, RiskLevel};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AvaliacaoSaude {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub timestamp_utc: String,
    pub nivel_risco: String,
    pub resumo: String,
    pub indicadores_analisados_json: String,
    pub fatores_contextuais_json: Option<String>,
    pub explicacao: Option<String>,
    pub orientacao: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

impl AvaliacaoSaude {
    /// Nível de risco interpretado da coluna gravada.
    pub fn nivel(&self) -> Option<RiskLevel> {
        RiskLevel::from_nome(&self.nivel_risco)
    }

    /// Evidências por indicador desserializadas de `indicadores_analisados_json`.
    /// Lista vazia quando o JSON não puder ser interpretado (nunca pânico).
    pub fn evidencias(&self) -> Vec<EvidenciaIndicador> {
        serde_json::from_str(&self.indicadores_analisados_json).unwrap_or_default()
    }

    /// Fatores contextuais desserializados, quando presentes e válidos.
    pub fn fatores_contextuais(&self) -> Option<FatoresContextuais> {
        self.fatores_contextuais_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn avaliacao(nivel: &str) -> AvaliacaoSaude {
        AvaliacaoSaude {
            id: "AV1".to_string(),
            astronauta_id: "A1".to_string(),
            missao_id: Some("M1".to_string()),
            timestamp_utc: "2025-01-15T08:00:00Z".to_string(),
            nivel_risco: nivel.to_string(),
            resumo: "resumo determinístico".to_string(),
            indicadores_analisados_json: r#"[
                {
                    "indicador": "SPO2",
                    "estado": "ANALISADO",
                    "nivel_variacao": "Atencao",
                    "z_score": -2.2,
                    "valor_atual": 91.0,
                    "valor_referencia": 98.0,
                    "direcao": "Abaixo",
                    "tendencia": "Diminuindo",
                    "persistente": true,
                    "baseline_desatualizado": null,
                    "qualidade_aceitavel": true,
                    "amostras_validas": 12,
                    "amostras_ausentes": 0,
                    "amostras_invalidas": 1,
                    "justificativa": "j"
                }
            ]"#
            .to_string(),
            fatores_contextuais_json: Some(
                r#"{
                    "janela_analise_segundos": 300,
                    "exposicao_ambiental": null,
                    "indicadores_insuficientes": [],
                    "indicadores_indisponiveis": ["HRV"],
                    "indicadores_invalidos": [],
                    "avaliacoes_anteriores_consideradas": 0,
                    "observacoes": []
                }"#
                .to_string(),
            ),
            explicacao: None,
            orientacao: None,
            updated_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn nivel_e_interpretado_da_coluna_do_banco() {
        assert_eq!(avaliacao("ATENCAO").nivel(), Some(RiskLevel::Atencao));
        assert_eq!(avaliacao("CRITICO").nivel(), Some(RiskLevel::Critico));
        assert_eq!(avaliacao("INVALIDO").nivel(), None);
    }

    #[test]
    fn evidencias_e_fatores_sao_desserializados() {
        let a = avaliacao("ATENCAO");
        let evidencias = a.evidencias();
        assert_eq!(evidencias.len(), 1);
        assert_eq!(evidencias[0].indicador, "SPO2");
        assert_eq!(evidencias[0].z_score, Some(-2.2));
        assert_eq!(evidencias[0].persistente, Some(true));

        let fatores = a.fatores_contextuais().expect("fatores válidos");
        assert_eq!(fatores.janela_analise_segundos, 300);
        assert_eq!(fatores.indicadores_indisponiveis, vec!["HRV"]);
    }

    #[test]
    fn json_invalido_nao_trava_a_entidade() {
        let mut a = avaliacao("NORMAL");
        a.indicadores_analisados_json = "nao-e-json".to_string();
        assert!(a.evidencias().is_empty());
        a.fatores_contextuais_json = Some("{{".to_string());
        assert!(a.fatores_contextuais().is_none());
    }
}
