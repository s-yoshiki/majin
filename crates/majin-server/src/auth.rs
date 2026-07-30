//! Token sign-in and session tracking.
//!
//! A client proves it knows the access token once, over `POST
//! /api/auth/session`, and gets back an opaque random session id in an
//! httpOnly cookie. Sessions live in this process only: they are revocable
//! immediately, there is no signing key to manage or leak, and a restart
//! invalidates everything, which is the right default for a process that hands
//! out shells.
//!
//! The token itself is never accepted on the WebSocket URL. Query strings end
//! up in proxy logs, browser history and `Referer` headers, and a token that
//! grants shell access does not belong in any of them.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::RwLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Name of the session cookie.
pub const COOKIE_NAME: &str = "majin_session";

/// Failed sign-in attempts allowed per client address within `LOCKOUT_WINDOW`.
const MAX_FAILED_ATTEMPTS: u32 = 10;
const LOCKOUT_WINDOW: Duration = Duration::from_secs(60);

/// Returns a random lowercase hex string of `bytes * 2` characters.
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("the OS random source must be available");
    buf.iter()
        .fold(String::with_capacity(bytes * 2), |mut acc, byte| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

fn digest(value: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hasher.finalize().into()
}

fn unix_millis(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginOutcome {
    Ok,
    BadToken,
    RateLimited,
}

struct Attempts {
    failures: u32,
    window_start: Instant,
}

/// Sign-in state shared by every request handler.
pub struct AuthState {
    /// `None` disables authentication (`--insecure-no-auth`).
    token_digest: Option<[u8; 32]>,
    ttl: Duration,
    sessions: RwLock<HashMap<String, SystemTime>>,
    attempts: RwLock<HashMap<IpAddr, Attempts>>,
}

impl AuthState {
    pub fn new(token: Option<&str>, ttl: Duration) -> Self {
        Self {
            token_digest: token.map(digest),
            ttl,
            sessions: RwLock::new(HashMap::new()),
            attempts: RwLock::new(HashMap::new()),
        }
    }

    pub fn auth_required(&self) -> bool {
        self.token_digest.is_some()
    }

    /// Verifies the token and, on success, mints a session.
    ///
    /// Returns the cookie value and its expiry. Comparison runs over SHA-256
    /// digests so it is constant time and independent of token length.
    pub fn login(
        &self,
        client: IpAddr,
        token: &str,
    ) -> (LoginOutcome, Option<(String, SystemTime)>) {
        let Some(expected) = self.token_digest else {
            return (LoginOutcome::Ok, Some(self.create_session()));
        };

        if self.is_rate_limited(client) {
            return (LoginOutcome::RateLimited, None);
        }

        if digest(token).ct_eq(&expected).into() {
            self.clear_attempts(client);
            (LoginOutcome::Ok, Some(self.create_session()))
        } else {
            self.record_failure(client);
            (LoginOutcome::BadToken, None)
        }
    }

    fn create_session(&self) -> (String, SystemTime) {
        let id = random_hex(32);
        let expires_at = SystemTime::now() + self.ttl;
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(id.clone(), expires_at);
        (id, expires_at)
    }

    /// Returns the expiry when `session_id` names a live session.
    pub fn validate(&self, session_id: &str) -> Option<SystemTime> {
        if self.token_digest.is_none() {
            return Some(SystemTime::now() + self.ttl);
        }
        let expires_at = *self
            .sessions
            .read()
            .expect("sessions lock poisoned")
            .get(session_id)?;
        if expires_at <= SystemTime::now() {
            self.revoke(session_id);
            return None;
        }
        Some(expires_at)
    }

    pub fn revoke(&self, session_id: &str) {
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .remove(session_id);
    }

    /// Drops expired entries so the map cannot grow without bound.
    pub fn sweep(&self) {
        let now = SystemTime::now();
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .retain(|_, exp| *exp > now);
        let cutoff = Instant::now();
        self.attempts
            .write()
            .expect("attempts lock poisoned")
            .retain(|_, a| cutoff.duration_since(a.window_start) < LOCKOUT_WINDOW);
    }

    pub fn expires_at_millis(time: SystemTime) -> f64 {
        unix_millis(time)
    }

    fn is_rate_limited(&self, client: IpAddr) -> bool {
        let attempts = self.attempts.read().expect("attempts lock poisoned");
        match attempts.get(&client) {
            Some(entry) if entry.window_start.elapsed() < LOCKOUT_WINDOW => {
                entry.failures >= MAX_FAILED_ATTEMPTS
            }
            _ => false,
        }
    }

    fn record_failure(&self, client: IpAddr) {
        let mut attempts = self.attempts.write().expect("attempts lock poisoned");
        let entry = attempts.entry(client).or_insert(Attempts {
            failures: 0,
            window_start: Instant::now(),
        });
        if entry.window_start.elapsed() >= LOCKOUT_WINDOW {
            entry.failures = 0;
            entry.window_start = Instant::now();
        }
        entry.failures += 1;
    }

    fn clear_attempts(&self, client: IpAddr) {
        self.attempts
            .write()
            .expect("attempts lock poisoned")
            .remove(&client);
    }
}

/// Extracts one cookie value from a raw `Cookie` header.
pub fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key.trim() == name).then(|| value.trim())
    })
}

/// Builds the `Set-Cookie` header for a new session.
///
/// `SameSite=Strict` keeps another site from riding the cookie into a
/// WebSocket upgrade; the explicit `Origin` check in `ws.rs` covers the same
/// ground for clients where that guarantee is weaker.
pub fn session_cookie(value: &str, max_age: Duration, secure: bool) -> String {
    let mut cookie = format!(
        "{COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        max_age.as_secs()
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn clear_cookie(secure: bool) -> String {
    let mut cookie = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCALHOST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);

    fn state() -> AuthState {
        AuthState::new(Some("s3cret"), Duration::from_secs(60))
    }

    #[test]
    fn a_correct_token_mints_a_session_that_validates() {
        let auth = state();
        let (outcome, session) = auth.login(LOCALHOST, "s3cret");
        assert_eq!(outcome, LoginOutcome::Ok);
        let (id, _) = session.expect("session");
        assert!(auth.validate(&id).is_some());
    }

    #[test]
    fn a_wrong_token_mints_nothing() {
        let auth = state();
        let (outcome, session) = auth.login(LOCALHOST, "wrong");
        assert_eq!(outcome, LoginOutcome::BadToken);
        assert!(session.is_none());
    }

    #[test]
    fn an_unknown_session_id_is_rejected() {
        assert!(state().validate("not-a-session").is_none());
    }

    #[test]
    fn revoking_invalidates_immediately() {
        let auth = state();
        let (_, session) = auth.login(LOCALHOST, "s3cret");
        let (id, _) = session.expect("session");
        auth.revoke(&id);
        assert!(auth.validate(&id).is_none());
    }

    #[test]
    fn repeated_failures_trip_the_rate_limiter() {
        let auth = state();
        for _ in 0..MAX_FAILED_ATTEMPTS {
            assert_eq!(auth.login(LOCALHOST, "wrong").0, LoginOutcome::BadToken);
        }
        // Even the correct token is refused once the window is exhausted.
        assert_eq!(auth.login(LOCALHOST, "s3cret").0, LoginOutcome::RateLimited);
    }

    #[test]
    fn expired_sessions_do_not_validate() {
        let auth = AuthState::new(Some("s3cret"), Duration::from_secs(0));
        let (_, session) = auth.login(LOCALHOST, "s3cret");
        let (id, _) = session.expect("session");
        assert!(auth.validate(&id).is_none());
    }

    #[test]
    fn disabled_auth_accepts_anything() {
        let auth = AuthState::new(None, Duration::from_secs(60));
        assert!(!auth.auth_required());
        assert_eq!(auth.login(LOCALHOST, "").0, LoginOutcome::Ok);
        assert!(auth.validate("anything").is_some());
    }

    #[test]
    fn cookies_are_parsed_out_of_a_multi_value_header() {
        let header = "theme=dark; majin_session=abc123; other=1";
        assert_eq!(cookie_value(header, COOKIE_NAME), Some("abc123"));
        assert_eq!(cookie_value(header, "missing"), None);
    }

    #[test]
    fn session_cookie_is_http_only_and_same_site_strict() {
        let cookie = session_cookie("abc", Duration::from_secs(60), true);
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Secure"));
        assert!(!session_cookie("abc", Duration::from_secs(60), false).contains("Secure"));
    }

    #[test]
    fn random_ids_are_unique_and_full_length() {
        let a = random_hex(32);
        let b = random_hex(32);
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }
}
