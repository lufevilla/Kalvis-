use axum::{
    Router,
    extract::{Query, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::Response,
    routing::get,
};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use crate::analysis::alert_stream::hub;

#[derive(Deserialize)]
pub struct Params {
    desde: Option<i64>,
    astronauta_id: Option<String>,
}

pub fn rotas() -> Router {
    Router::new().route("/ws/analises", get(ws))
}

async fn ws(Query(q): Query<Params>, up: WebSocketUpgrade) -> Response {
    let rx = hub().assinar(); // assina antes do replay: sem lacunas
    up.on_upgrade(move |s| sessao(s, rx, q))
}

async fn sessao(
    mut s: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<std::sync::Arc<crate::analysis::alert_stream::EventoAlerta>>,
    q: Params,
) {
    let filtro = q.astronauta_id.as_deref();
    let mut ultimo = q.desde.unwrap_or(0);

    for e in hub().desde(ultimo, filtro) {
        ultimo = ultimo.max(e.seq);
        if s.send(Message::Text(e.json.clone().into())).await.is_err() {
            return;
        }
    }
    loop {
        let msg = match rx.recv().await {
            Ok(e) if e.seq > ultimo && filtro.is_none_or(|a| a == e.astronauta_id) => {
                ultimo = e.seq;
                e.json.clone()
            }
            Ok(_) => continue,
            Err(RecvError::Lagged(_)) => r#"{"tipo":"stream.ressincronizar"}"#.to_string(),
            Err(RecvError::Closed) => return,
        };
        if s.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }
}
