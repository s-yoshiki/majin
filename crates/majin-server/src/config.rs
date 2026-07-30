//! Command line and environment configuration.

use std::{net::IpAddr, path::PathBuf, time::Duration};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "majin",
    version,
    about = "Serve a terminal over HTTP and WebSocket"
)]
pub struct Cli {
    /// Address to bind.
    ///
    /// Defaults to loopback on purpose: this process hands out an interactive
    /// shell, so exposing it to a network has to be a deliberate act.
    #[arg(long, env = "MAJIN_HOST", default_value = "127.0.0.1")]
    pub host: IpAddr,

    #[arg(long, short, env = "MAJIN_PORT", default_value_t = 8999)]
    pub port: u16,

    /// Access token clients must present. Generated and printed when unset.
    #[arg(long, env = "MAJIN_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// Disable authentication entirely. Only sane behind another auth layer.
    #[arg(long, env = "MAJIN_INSECURE_NO_AUTH", default_value_t = false)]
    pub insecure_no_auth: bool,

    /// Shell to spawn. Defaults to `$SHELL`, or `%COMSPEC%` on Windows.
    #[arg(long, env = "MAJIN_SHELL")]
    pub shell: Option<String>,

    /// Working directory for new sessions. Defaults to the user's home.
    #[arg(long, env = "MAJIN_CWD")]
    pub cwd: Option<PathBuf>,

    /// How long a signed-in session stays valid, in minutes.
    #[arg(long, env = "MAJIN_SESSION_TTL_MINUTES", default_value_t = 720)]
    pub session_ttl_minutes: u64,

    /// Maximum number of concurrent terminals.
    #[arg(long, env = "MAJIN_MAX_SESSIONS", default_value_t = 8)]
    pub max_sessions: usize,

    /// Extra origins allowed to open a WebSocket, e.g. the Vite dev server.
    ///
    /// Same-origin requests are always allowed; this is only needed when the
    /// page is served from somewhere other than this process.
    #[arg(
        long = "allowed-origin",
        env = "MAJIN_ALLOWED_ORIGINS",
        value_delimiter = ','
    )]
    pub allowed_origins: Vec<String>,

    /// Force the `Secure` flag on the session cookie.
    ///
    /// Detected automatically from `X-Forwarded-Proto`; set this when the
    /// proxy does not send that header.
    #[arg(long, env = "MAJIN_SECURE_COOKIE", default_value_t = false)]
    pub secure_cookie: bool,
}

impl Cli {
    pub fn session_ttl(&self) -> Duration {
        Duration::from_secs(self.session_ttl_minutes.max(1) * 60)
    }
}
