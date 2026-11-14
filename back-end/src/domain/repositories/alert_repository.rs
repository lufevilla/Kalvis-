use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::domain::entities::alert_entity::{Alerta, AlertaSaude, EntradaAlerta};

/// Projeção da Query 7 — quantidade de alertas por nível de risco.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContagemNivelRisco {
    pub nivel_risco: String,
    pub quantidade: i64,
}

#[async_trait]
pub trait AlertRepository: Send + Sync {
    /// Query 7.1 — busca um alerta de saúde pelo `id`.
    async fn find_by_id(&self, id: &str) -> Result<Option<AlertaSaude>, sqlx::Error>;

    /// Query 7.2 — lista alertas com filtros opcionais e paginação.
    async fn list(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
        nivel_risco: Option<&str>,
        limite: i64,
        deslocamento: i64,
    ) -> Result<Vec<AlertaSaude>, sqlx::Error>;

    /// Query 7.3 — lista os alertas mais recentes de um astronauta.
    async fn list_recent_by_astronaut(
        &self,
        astronauta_id: &str,
        limite: i64,
    ) -> Result<Vec<AlertaSaude>, sqlx::Error>;

    /// Query 7.4 — conta os alertas agrupados por nível de risco.
    async fn count_by_risk_level(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
    ) -> Result<Vec<ContagemNivelRisco>, sqlx::Error>;

    // ============================================================
    // OPERAÇÕES DE ESCRITA — tabela `alertas` (migration 008).
    //
    // Alertas operacionais: UM registro ATIVO por condição
    // (astronauta + indicadores alterados). Enquanto a condição persiste,
    // o mesmo registro é atualizado; ao terminar, é marcado RESOLVIDO.
    // ============================================================

    /// Cria o alerta se não existir um ATIVO para a mesma condição, ou
    /// ATUALIZA o existente (nível/resumo/explicação/`atualizado_em`).
    /// A operação é atômica (transação). Devolve `(alerta, criado)`.
    async fn criar_ou_atualizar_alerta(
        &self,
        entrada: &EntradaAlerta,
    ) -> Result<(Alerta, bool), sqlx::Error>;

    /// Lista os alertas ATIVOS (`estado = 'ATIVO'`), ordenados do mais
    /// recente para o mais antigo.
    async fn listar_alertas_ativos(
        &self,
        astronauta_id: Option<&str>,
        missao_id: Option<&str>,
    ) -> Result<Vec<Alerta>, sqlx::Error>;

    /// Busca o alerta ATIVO de uma condição específica do astronauta.
    async fn buscar_alerta_ativo(
        &self,
        astronauta_id: &str,
        condicao: &str,
    ) -> Result<Option<Alerta>, sqlx::Error>;

    /// Marca um alerta como RESOLVIDO. Devolve `true` se algum registro
    /// foi alterado.
    async fn resolver_alerta(&self, id: &str, resolvido_em: &str) -> Result<bool, sqlx::Error>;

    /// Marca como RESOLVIDOS todos os alertas ativos do astronauta
    /// (opcionalmente por missão). Devolve a quantidade resolvida.
    async fn resolver_todos(
        &self,
        astronauta_id: &str,
        missao_id: Option<&str>,
        resolvido_em: &str,
    ) -> Result<u64, sqlx::Error>;

    /// Atualiza `explicacao`/`orientacao` de um alerta com o resultado da
    /// orientação da IA (sem alterar nível, resumo nem estado).
    async fn atualizar_orientacao_alerta(
        &self,
        id: &str,
        explicacao: Option<&str>,
        orientacao: Option<&str>,
        atualizado_em: &str,
    ) -> Result<(), sqlx::Error>;
}
