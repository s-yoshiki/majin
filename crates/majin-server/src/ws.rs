//! The `/ws` endpoint: one WebSocket, one PTY.

use std::{net::SocketAddr, sync::atomic::Ordering};

use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket},
    },
    http::HeaderMap,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use majin_protocol::{
    ClientMessage, PROTOCOL_VERSION, ServerMessage, TerminalErrorCode, TerminalSize, close_code,
};
use majin_pty::{PtyConfig, PtyEvent, PtySession};

use crate::{
    AppState,
    auth::{COOKIE_NAME, cookie_value},
};

/// Why a connection was refused, kept separate from the upgrade itself.
enum Refusal {
    Unauthorized,
    ForbiddenOrigin,
    SessionLimit,
}

impl Refusal {
    fn close_code(&self) -> u16 {
        match self {
            Self::Unauthorized | Self::ForbiddenOrigin => close_code::UNAUTHORIZED,
            Self::SessionLimit => close_code::SESSION_LIMIT,
        }
    }

    fn error(&self) -> ServerMessage {
        let (code, message) = match self {
            Self::Unauthorized => (
                TerminalErrorCode::Unauthorized,
                "Not signed in, or the session expired.",
            ),
            Self::ForbiddenOrigin => (
                TerminalErrorCode::Unauthorized,
                "Request origin is not allowed.",
            ),
            Self::SessionLimit => (
                TerminalErrorCode::SessionLimit,
                "Too many terminals are already open.",
            ),
        };
        ServerMessage::Error {
            code,
            message: message.to_owned(),
        }
    }
}

pub async fn handler(
    upgrade: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let refusal = check(&state, &headers);

    // The upgrade is accepted even when the request is refused: a browser
    // cannot read the status code of a failed handshake, but it can read a
    // close code, which is how the client knows to show the login screen
    // instead of retrying forever.
    upgrade.on_upgrade(move |socket| async move {
        match refusal {
            Some(reason) => refuse(socket, reason).await,
            None => run_session(socket, state, peer).await,
        }
    })
}

fn check(state: &AppState, headers: &HeaderMap) -> Option<Refusal> {
    if !origin_allowed(state, headers) {
        return Some(Refusal::ForbiddenOrigin);
    }

    if state.auth.auth_required() {
        let session = headers
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|header| cookie_value(header, COOKIE_NAME));

        match session {
            Some(id) if state.auth.validate(id).is_some() => {}
            _ => return Some(Refusal::Unauthorized),
        }
    }

    if state.active_sessions.load(Ordering::SeqCst) >= state.max_sessions {
        return Some(Refusal::SessionLimit);
    }

    None
}

/// Cookie-authenticated WebSockets need an explicit origin check.
///
/// `SameSite=Strict` already stops most cross-site upgrades, but the guarantee
/// is not uniform across clients, and without this any page the user visits
/// could open a shell on their machine.
fn origin_allowed(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        // Non-browser clients omit `Origin`; there is no CSRF risk without a
        // browser to carry the cookie implicitly.
        return true;
    };

    if state
        .allowed_origins
        .iter()
        .any(|allowed| allowed == origin)
    {
        return true;
    }

    // Same-origin: the Origin authority matches the Host we were reached on.
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok());
    match (origin.split_once("://"), host) {
        (Some((_, origin_authority)), Some(host)) => origin_authority == host,
        _ => false,
    }
}

async fn refuse(mut socket: WebSocket, reason: Refusal) {
    let message = reason.error();
    if let Ok(json) = serde_json::to_string(&message) {
        let _ = socket.send(Message::Text(json.into())).await;
    }
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: reason.close_code(),
            reason: "refused".into(),
        })))
        .await;
}

async fn run_session(socket: WebSocket, state: AppState, peer: SocketAddr) {
    state.active_sessions.fetch_add(1, Ordering::SeqCst);
    let _guard = SessionGuard(state.clone());

    let session_id = crate::auth::random_hex(8);
    tracing::info!(%peer, session = %session_id, "terminal session opened");

    let (mut sender, mut receiver) = socket.split();

    let spawned = PtySession::spawn(PtyConfig {
        shell: state.shell.clone(),
        cwd: state.cwd.clone(),
        size: TerminalSize::default(),
        ..Default::default()
    });

    let (mut pty, mut events) = match spawned {
        Ok(pair) => pair,
        Err(err) => {
            tracing::error!(%err, "failed to spawn a shell");
            let message = ServerMessage::Error {
                code: TerminalErrorCode::SpawnFailed,
                message: err.to_string(),
            };
            if let Ok(json) = serde_json::to_string(&message) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
            return;
        }
    };

    let ready = ServerMessage::Ready {
        protocol_version: PROTOCOL_VERSION,
        session_id: session_id.clone(),
        shell: pty.shell().to_owned(),
        cols: pty.size().cols,
        rows: pty.size().rows,
    };
    if let Ok(json) = serde_json::to_string(&ready) {
        if sender.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    let mut close_code = close_code::NORMAL;

    loop {
        tokio::select! {
            event = events.recv() => match event {
                Some(PtyEvent::Output(data)) => {
                    let message = ServerMessage::Output { data };
                    let Ok(json) = serde_json::to_string(&message) else { continue };
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Some(PtyEvent::Exit(exit_code)) => {
                    let message = ServerMessage::Exit { exit_code };
                    if let Ok(json) = serde_json::to_string(&message) {
                        let _ = sender.send(Message::Text(json.into())).await;
                    }
                    close_code = close_code::SHELL_EXITED;
                    break;
                }
                None => break,
            },

            incoming = receiver.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Input { data }) => {
                            if let Err(err) = pty.write(&data) {
                                tracing::warn!(%err, "write to pty failed");
                                break;
                            }
                        }
                        Ok(ClientMessage::Resize { cols, rows }) => {
                            if let Err(err) = pty.resize(TerminalSize::new(cols, rows)) {
                                tracing::warn!(%err, "resize failed");
                            }
                        }
                        Ok(ClientMessage::Ping { at }) => {
                            let message = ServerMessage::Pong { at };
                            if let Ok(json) = serde_json::to_string(&message) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                        }
                        Err(err) => {
                            tracing::debug!(%err, "discarding malformed frame");
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    tracing::debug!(%err, "websocket error");
                    break;
                }
            },
        }
    }

    let _ = sender
        .send(Message::Close(Some(CloseFrame {
            code: close_code,
            reason: "session ended".into(),
        })))
        .await;

    tracing::info!(session = %session_id, "terminal session closed");
    // Dropping `pty` kills the shell, so a closed tab never leaves one behind.
}

/// Decrements the active-session count however the handler exits.
struct SessionGuard(AppState);

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.0.active_sessions.fetch_sub(1, Ordering::SeqCst);
    }
}
