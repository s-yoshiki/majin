//! Wire protocol shared by every majin frontend and backend.
//!
//! These types are the single source of truth: the TypeScript definitions in
//! `packages/protocol/src/generated/` are produced from them by the
//! `export-bindings` binary, and CI fails if the committed output drifts.
//!
//! The pre-monorepo code is what motivated this. The browser sent
//! `{ resizer: [...] }` while the server read `msg.resize`, so window resizing
//! silently did nothing and nothing caught it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Bumped whenever a change would break an older client.
pub const PROTOCOL_VERSION: u32 = 1;

/// PTY resize rejects non-positive values, and an unbounded request would make
/// the kernel allocate scrollback buffers sized by whatever the client claims.
pub const MIN_COLS: u16 = 1;
pub const MIN_ROWS: u16 = 1;
pub const MAX_COLS: u16 = 1000;
pub const MAX_ROWS: u16 = 1000;

/// Terminal geometry in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Clamps to a range the PTY layer will actually accept.
    pub fn clamped(self) -> Self {
        Self {
            cols: self.cols.clamp(MIN_COLS, MAX_COLS),
            rows: self.rows.clamp(MIN_ROWS, MAX_ROWS),
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

/// Frames sent by the client.
///
/// Timestamps are `f64` milliseconds to line up with `Date.now()`, which is
/// also why this cannot derive `Eq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMessage {
    /// Keystrokes or pasted text typed into the terminal.
    Input { data: String },
    /// The viewport changed size and the PTY should follow.
    Resize { cols: u16, rows: u16 },
    /// Liveness probe; the server answers with `Pong` carrying the same `at`.
    Ping { at: f64 },
}

/// Frames sent by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    /// Sent once, right after the PTY is spawned.
    Ready {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
        #[serde(rename = "sessionId")]
        session_id: String,
        shell: String,
        cols: u16,
        rows: u16,
    },
    /// Raw PTY output, forwarded verbatim to xterm.
    Output {
        data: String,
    },
    /// The child process terminated. The socket closes right after.
    Exit {
        #[serde(rename = "exitCode")]
        exit_code: i32,
    },
    /// A recoverable or fatal server-side failure, described for the UI.
    Error {
        code: TerminalErrorCode,
        message: String,
    },
    Pong {
        at: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TerminalErrorCode {
    Unauthorized,
    SpawnFailed,
    SessionLimit,
    BadMessage,
    Internal,
}

/// Response body of `GET`/`POST`/`DELETE` on `/api/auth/session`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct SessionStatus {
    pub authenticated: bool,
    /// Unix milliseconds at which the current session stops being valid.
    /// Always present, and `null` when not signed in, matching the generated
    /// TypeScript type exactly.
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<f64>,
}

/// Request body of `POST /api/auth/session`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LoginRequest {
    pub token: String,
}

/// Error body returned by every `/api` route that fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

/// Server metadata exposed to the UI before sign-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ServerInfo {
    pub version: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    /// False for the desktop build, where the PTY is reached over local IPC.
    #[serde(rename = "authRequired")]
    pub auth_required: bool,
}

/// WebSocket close codes in the application-private range (4000-4999).
///
/// `UNAUTHORIZED` is deliberately distinct so the client can fall back to the
/// login screen instead of retrying a reconnect that can never succeed.
pub mod close_code {
    pub const NORMAL: u16 = 1000;
    pub const UNAUTHORIZED: u16 = 4001;
    pub const SESSION_LIMIT: u16 = 4002;
    pub const PROTOCOL_ERROR: u16 = 4003;
    pub const SHELL_EXITED: u16 = 4004;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_messages_use_a_type_tag() {
        let json = serde_json::to_string(&ClientMessage::Resize {
            cols: 120,
            rows: 40,
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"resize","cols":120,"rows":40}"#);
    }

    #[test]
    fn server_messages_use_camel_case_fields() {
        let json = serde_json::to_string(&ServerMessage::Exit { exit_code: 0 }).unwrap();
        assert_eq!(json, r#"{"type":"exit","exitCode":0}"#);
    }

    #[test]
    fn unknown_frames_are_rejected_rather_than_defaulted() {
        assert!(serde_json::from_str::<ClientMessage>(r#"{"type":"resizer"}"#).is_err());
    }

    #[test]
    fn sizes_are_clamped_into_the_pty_accepted_range() {
        assert_eq!(TerminalSize::new(0, 0).clamped(), TerminalSize::new(1, 1));
        assert_eq!(
            TerminalSize::new(5000, 5000).clamped(),
            TerminalSize::new(MAX_COLS, MAX_ROWS)
        );
    }
}
