use std::sync::Arc;

use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::{AppState, models::WSFilter};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(filter): Query<WSFilter>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    validate_filter(&filter).unwrap();
    ws.on_upgrade(move |socket| handle_socket(socket, filter, state))
}

async fn handle_socket(socket: WebSocket, ws_filter: WSFilter, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.broadcaster.subscribe();

    let peer_info = format!(
        "serivce={:?} min_level={:?}",
        ws_filter.service, ws_filter.min_level
    );

    info!(filter = %peer_info, "Websocket client connected");

    let filter = Arc::new(Mutex::new(ws_filter));
    let filter_writer = filter.clone();

    // Send to ws client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(entry) => {
                    let matches = {
                        let f = filter.lock().await;
                        f.matches(&entry)
                    };

                    if !matches {
                        continue;
                    }

                    let json = match serde_json::to_string(&entry) {
                        Ok(j) => j,
                        Err(e) => {
                            warn!(errro = %e,"Failed to serialize log entr for websocket");
                            continue;
                        }
                    };

                    if sender.send(Message::Text(json.into())).await.is_err() {
                        debug!("WebSocket send failed. client likely disconnected");
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        dropped = n,
                        "Websockt subscriber lagger. Some entries were dropped"
                    );

                    let notice = serde_json::json!({
                        "type": "lag_notice",
                        "dropped": n,
                        "message": format!("{} entries werre dropped due to slow consumption", n)
                    });

                    if sender
                        .send(Message::Text(notice.to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    debug!("Broadcast channel closed, terminating WebSocket send task");
                    break;
                }
            }
        }
    });

    // Takes updated filter from ws client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<WSFilter>(&text) {
                    Ok(new_filter) => {
                        if let Err(e) = validate_filter(&new_filter) {
                            warn!(error = %e, "Client send invalid filter update. Ignore!");
                        } else {
                            let mut f = filter_writer.lock().await;
                            info!(service = ?new_filter.service, min_level = ?new_filter.min_level, "Websocket client updated filter");

                            *f = new_filter;
                        }
                    }
                    Err(e) => {
                        debug!(error = %e, raw = %text, "Received non-filter websocket message. Ignore!");
                    }
                },
                Message::Close(_) => {
                    debug!("Websocket client sent close");
                    break;
                }
                Message::Ping(_) => {} // axum auto-responds with Pong
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("WebSocket client disconnected");
}

pub fn validate_filter(filter: &WSFilter) -> Result<(), &'static str> {
    if let Some(ref svc) = filter.service {
        if svc.is_empty() {
            return Err("Service field must not be empty".into());
        }

        if svc.len() > 64 {
            return Err("Servic field must be of 64 character of fewer".into());
        }
    }

    Ok(())
}
