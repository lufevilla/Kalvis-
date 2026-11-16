/*
ERRO APLICACIONAL COMPARTILHADO.

Os casos de uso devolvem `AppErro` e a orquestração decide o que fazer:
falha pontual de indicador é registrada e refletida na qualidade/completude
da avaliação (nunca silenciada); falha de IA NUNCA interrompe o fluxo.
*/

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
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

    #[error("não encontrado: {0}")]
    NaoEncontrado(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppErro {
    fn into_response(self) -> Response {
        let (status, error_msg) = match self {
            AppErro::Repositorio(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppErro::Configuracao(msg) => (StatusCode::BAD_REQUEST, msg),
            AppErro::DadosInvalidos(msg) => (StatusCode::BAD_REQUEST, msg),
            AppErro::IaIndisponivel(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            AppErro::NaoEncontrado(msg) => (StatusCode::NOT_FOUND, msg),
        };
        (status, Json(ErrorResponse { error: error_msg })).into_response()
    }
}

pub type AppResultado<T> = Result<T, AppErro>;
