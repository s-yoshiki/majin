//! `majin-server`: serves the terminal UI and a PTY over WebSocket.

mod assets;
mod auth;
mod config;
mod ws;

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
    time::{Duration, SystemTime},
};

use axum::{
    Json, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use clap::Parser;
use majin_protocol::{ApiError, LoginRequest, PROTOCOL_VERSION, ServerInfo, SessionStatus};

use crate::{
    auth::{AuthState, COOKIE_NAME, LoginOutcome, clear_cookie, cookie_value, session_cookie},
    config::Cli,
};

/// Shared handler state. Cheap to clone; everything costly sits behind an `Arc`.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<AuthState>,
    pub shell: Option<String>,
    pub cwd: Option<PathBuf>,
    pub max_sessions: usize,
    pub allowed_origins: Arc<Vec<String>>,
    pub active_sessions: Arc<AtomicUsize>,
    pub secure_cookie: bool,
    pub session_ttl: Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MAJIN_LOG")
                .unwrap_or_else(|_| "majin_server=info,tower_http=warn".into()),
        )
        .init();

    // An explicitly generated token is far safer than a default one, and
    // printing it is the only way the operator can use it.
    let token = if cli.insecure_no_auth {
        None
    } else {
        Some(cli.token.clone().unwrap_or_else(|| auth::random_hex(24)))
    };

    let state = AppState {
        auth: Arc::new(AuthState::new(token.as_deref(), cli.session_ttl())),
        shell: cli.shell.clone(),
        cwd: cli.cwd.clone(),
        max_sessions: cli.max_sessions,
        allowed_origins: Arc::new(cli.allowed_origins.clone()),
        active_sessions: Arc::new(AtomicUsize::new(0)),
        secure_cookie: cli.secure_cookie,
        session_ttl: cli.session_ttl(),
    };

    spawn_session_sweeper(state.auth.clone());

    let app = Router::new()
        .route("/api/info", get(get_info))
        .route(
            "/api/auth/session",
            get(get_session).post(post_session).delete(delete_session),
        )
        .route("/ws", any(ws::handler))
        .fallback(assets::handler)
        .with_state(state);

    let addr = SocketAddr::new(cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;

    print_banner(&cli, bound, token.as_deref());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

// --- Routes -----------------------------------------------------------------

async fn get_info(State(state): State<AppState>) -> Json<ServerInfo> {
    Json(ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol_version: PROTOCOL_VERSION,
        auth_required: state.auth.auth_required(),
    })
}

async fn get_session(State(state): State<AppState>, headers: HeaderMap) -> Json<SessionStatus> {
    Json(match current_session(&state, &headers) {
        Some(expires_at) => SessionStatus {
            authenticated: true,
            expires_at: Some(AuthState::expires_at_millis(expires_at)),
        },
        None => SessionStatus {
            authenticated: false,
            expires_at: None,
        },
    })
}

async fn post_session(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Response {
    let (outcome, session) = state.auth.login(peer.ip(), &body.token);

    match outcome {
        LoginOutcome::RateLimited => api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Too many failed attempts. Wait a minute and try again.",
        ),
        LoginOutcome::BadToken => api_error(
            StatusCode::UNAUTHORIZED,
            "invalid_token",
            "That token is not valid.",
        ),
        LoginOutcome::Ok => {
            let Some((id, expires_at)) = session else {
                return api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Failed to create a session.",
                );
            };

            let cookie =
                session_cookie(&id, state.session_ttl, use_secure_cookie(&state, &headers));
            let body = Json(SessionStatus {
                authenticated: true,
                expires_at: Some(AuthState::expires_at_millis(expires_at)),
            });

            match HeaderValue::from_str(&cookie) {
                Ok(value) => ([(header::SET_COOKIE, value)], body).into_response(),
                Err(_) => api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Failed to create a session.",
                ),
            }
        }
    }
}

async fn delete_session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(id) = session_id(&headers) {
        state.auth.revoke(id);
    }

    let cookie = clear_cookie(use_secure_cookie(&state, &headers));
    let body = Json(SessionStatus {
        authenticated: false,
        expires_at: None,
    });

    match HeaderValue::from_str(&cookie) {
        Ok(value) => ([(header::SET_COOKIE, value)], body).into_response(),
        Err(_) => body.into_response(),
    }
}

// --- Helpers ----------------------------------------------------------------

fn session_id(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|header| cookie_value(header, COOKIE_NAME))
}

fn current_session(state: &AppState, headers: &HeaderMap) -> Option<SystemTime> {
    if !state.auth.auth_required() {
        return Some(SystemTime::now() + state.session_ttl);
    }
    state.auth.validate(session_id(headers)?)
}

/// `Secure` cookies are dropped over plain HTTP, which would break the common
/// `http://localhost` case, so it is enabled only when the request really did
/// arrive over TLS or the operator forced it on.
fn use_secure_cookie(state: &AppState, headers: &HeaderMap) -> bool {
    if state.secure_cookie {
        return true;
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|proto| proto.eq_ignore_ascii_case("https"))
}

fn api_error(status: StatusCode, error: &str, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            error: error.to_owned(),
            message: message.to_owned(),
        }),
    )
        .into_response()
}

fn spawn_session_sweeper(auth: Arc<AuthState>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            auth.sweep();
        }
    });
}

fn print_banner(cli: &Cli, bound: SocketAddr, token: Option<&str>) {
    let host = if bound.ip().is_unspecified() {
        "localhost".to_owned()
    } else {
        bound.ip().to_string()
    };
    let base = format!("http://{host}:{}", bound.port());

    println!();
    println!(
        "  majin {} — terminal over the web",
        env!("CARGO_PKG_VERSION")
    );
    println!();

    match token {
        Some(token) => {
            println!("  Open:  {base}/#token={token}");
            println!();
            println!("  Token: {token}");
            println!("  The link carries the token in the URL fragment, which browsers");
            println!("  never send to a server. It is cleared from the address bar as");
            println!("  soon as the page exchanges it for a session cookie.");
        }
        None => {
            println!("  Open:  {base}");
            println!();
            println!("  !! Authentication is DISABLED (--insecure-no-auth).");
            println!(
                "  !! Anyone who can reach this port gets a shell as {}.",
                std::env::var("USER").unwrap_or_else(|_| "this user".into())
            );
        }
    }

    if !bound.ip().is_loopback() {
        println!();
        println!("  !! Listening on {bound}, which is reachable beyond this machine.");
        println!("  !! Put it behind TLS; the session cookie and every keystroke");
        println!("  !! travel in clear text over plain HTTP.");
    }

    if !assets::is_embedded() {
        println!();
        println!("  Note: no frontend is embedded in this binary. Build it with");
        println!("  `pnpm build:web`, or use the Vite dev server on port 5173.");
    }

    if cli.insecure_no_auth && !bound.ip().is_loopback() {
        tracing::warn!("authentication disabled on a non-loopback address");
    }

    println!();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
