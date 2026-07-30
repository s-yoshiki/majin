//! Desktop backend: the same `majin-pty` crate the server uses, reached over
//! Tauri IPC instead of a WebSocket.
//!
//! Nothing is bound to a network port and there is no token to manage — the
//! only thing that can talk to this PTY is the window in this process.

use std::sync::Mutex;

use majin_protocol::TerminalSize;
use majin_pty::{PtyConfig, PtyEvent, PtySession};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

/// Event names the frontend listens on.
const EVENT_OUTPUT: &str = "pty://output";
const EVENT_EXIT: &str = "pty://exit";

#[derive(Default)]
struct PtyState(Mutex<Option<PtySession>>);

/// Payload of a successful `pty_open`, mirroring the protocol's `ready` frame.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyReady {
    shell: String,
    cols: u16,
    rows: u16,
}

#[tauri::command]
fn pty_open(
    app: AppHandle,
    state: State<'_, PtyState>,
    cols: u16,
    rows: u16,
) -> Result<PtyReady, String> {
    let mut slot = state
        .0
        .lock()
        .map_err(|_| "pty state is poisoned".to_owned())?;

    // Reopening replaces the previous shell rather than leaking it; dropping
    // the old session kills its child.
    slot.take();

    let (session, mut events) = PtySession::spawn(PtyConfig {
        size: TerminalSize::new(cols, rows),
        ..Default::default()
    })
    .map_err(|err| err.to_string())?;

    let ready = PtyReady {
        shell: session.shell().to_owned(),
        cols: session.size().cols,
        rows: session.size().rows,
    };

    // The reader lives on its own thread inside `majin-pty`; this task only
    // forwards what it produces onto the window's event bus.
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            let sent = match event {
                PtyEvent::Output(data) => app.emit(EVENT_OUTPUT, data),
                PtyEvent::Exit(code) => app.emit(EVENT_EXIT, code),
            };
            if let Err(err) = sent {
                tracing::warn!(%err, "failed to emit pty event");
                break;
            }
        }
    });

    *slot = Some(session);
    Ok(ready)
}

#[tauri::command]
fn pty_write(state: State<'_, PtyState>, data: String) -> Result<(), String> {
    let mut slot = state
        .0
        .lock()
        .map_err(|_| "pty state is poisoned".to_owned())?;
    match slot.as_mut() {
        Some(session) => session.write(&data).map_err(|err| err.to_string()),
        None => Err("no terminal session is open".to_owned()),
    }
}

#[tauri::command]
fn pty_resize(state: State<'_, PtyState>, cols: u16, rows: u16) -> Result<(), String> {
    let slot = state
        .0
        .lock()
        .map_err(|_| "pty state is poisoned".to_owned())?;
    match slot.as_ref() {
        Some(session) => session
            .resize(TerminalSize::new(cols, rows))
            .map_err(|err| err.to_string()),
        // A resize can race ahead of `pty_open`; the shell picks up the real
        // size from the next one, so this is not worth surfacing as an error.
        None => Ok(()),
    }
}

#[tauri::command]
fn pty_close(state: State<'_, PtyState>) -> Result<(), String> {
    let mut slot = state
        .0
        .lock()
        .map_err(|_| "pty state is poisoned".to_owned())?;
    slot.take();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("MAJIN_LOG")
                .unwrap_or_else(|_| "majin_desktop_lib=info".into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            app.manage(PtyState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pty_open, pty_write, pty_resize, pty_close
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the majin window");
}
