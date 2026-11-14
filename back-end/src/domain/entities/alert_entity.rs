use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Projeção de um registro de risco em `avaliacoes_saude`
/// (histórico operacional: avaliações com nível >= ATENCAO).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertaSaude {
    pub id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub timestamp_utc: String,
    pub nivel_risco: String,
    pub resumo: String,
    pub explicacao: Option<String>,
    pub orientacao: Option<String>,
}

/*
ALERTA OPERACIONAL (tabela `alertas`, migration 008).

Um alerta é UM registro vivo por condição (astronauta + indicadores
alterados). Enquanto a condição persiste, o MESMO registro evolui
(nível, resumo, explicação, `atualizado_em`); quando a condição deixa de
existir, ele é marcado RESOLVIDO (`resolvido_em`). O índice único parcial
do banco garante no máximo um alerta ATIVO por (astronauta, condicao).
*/
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alerta {
    pub id: String,
    /// Avaliação que criou/atualizou o alerta mais recentemente.
    pub avaliacao_id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    /// Chave estável da condição (indicadores alterados + direções).
    pub condicao: String,
    /// Início do alerta (não muda enquanto o alerta evolui).
    pub timestamp_utc: String,
    pub nivel_risco: String,
    pub resumo: String,
    pub explicacao: Option<String>,
    pub orientacao: Option<String>,
    /// "ATIVO" ou "RESOLVIDO".
    pub estado: String,
    pub atualizado_em: String,
    pub resolvido_em: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

impl Alerta {
    pub const ESTADO_ATIVO: &'static str = "ATIVO";
    pub const ESTADO_RESOLVIDO: &'static str = "RESOLVIDO";

    pub fn ativo(&self) -> bool {
        self.estado == Self::ESTADO_ATIVO && self.deleted_at.is_none()
    }
}

/// Dados de entrada para criar ou atualizar um alerta.
#[derive(Debug, Clone)]
pub struct EntradaAlerta {
    pub avaliacao_id: String,
    pub astronauta_id: String,
    pub missao_id: Option<String>,
    pub condicao: String,
    pub timestamp_utc: String,
    pub nivel_risco: String,
    pub resumo: String,
    pub explicacao: Option<String>,
    pub orientacao: Option<String>,
    pub atualizado_em: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alerta(estado: &str) -> Alerta {
        Alerta {
            id: "AL1".into(),
            avaliacao_id: "AV1".into(),
            astronauta_id: "A1".into(),
            missao_id: Some("M1".into()),
            condicao: "SPO2:Abaixo".into(),
            timestamp_utc: "2025-01-15T08:00:00Z".into(),
            nivel_risco: "ALERTA".into(),
            resumo: "resumo".into(),
            explicacao: None,
            orientacao: None,
            estado: estado.into(),
            atualizado_em: "2025-01-15T08:00:00Z".into(),
            resolvido_em: None,
            updated_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn estado_do_alerta_e_interpretado() {
        assert!(alerta("ATIVO").ativo());
        assert!(!alerta("RESOLVIDO").ativo());
        let mut removido = alerta("ATIVO");
        removido.deleted_at = Some("2025-01-15T09:00:00Z".into());
        assert!(!removido.ativo(), "soft delete encerra o alerta");
    }
}
