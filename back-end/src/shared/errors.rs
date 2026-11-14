/*
ERRO APLICACIONAL COMPARTILHADO.

Os casos de uso devolvem `AppErro` e a orquestração decide o que fazer:
falha pontual de indicador é registrada e refletida na qualidade/completude
da avaliação (nunca silenciada); falha de IA NUNCA interrompe o fluxo.
*/

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppErro {
    #[error("erro de repositório: {0}")]
    Repositorio(#[from] sqlx::Error),

    #[error("configuração inválida: {0}")]
    Configuracao(String),

    #[error("dados inválidos: {0}")]
    DadosInvalidos(String),

    #[error("orientação por IA indisponível: {0}")]
    IaIndisponivel(String),
}

pub type AppResultado<T> = Result<T, AppErro>;
