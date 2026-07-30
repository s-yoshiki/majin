//! PTY session management, shared by the axum server and the Tauri desktop app.
//!
//! Both frontends need the same thing: spawn a login shell on a pseudo
//! terminal, stream its output, forward keystrokes, and follow window resizes.
//! Keeping that in one crate is why the desktop build does not need a second
//! implementation of the terminal backend.

mod utf8;

use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use majin_protocol::TerminalSize;
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::utf8::Utf8Stream;

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("failed to open a pseudo terminal: {0}")]
    OpenPty(String),
    #[error("failed to spawn `{shell}`: {reason}")]
    Spawn { shell: String, reason: String },
    #[error("failed to write to the pseudo terminal: {0}")]
    Write(#[source] std::io::Error),
    #[error("failed to resize the pseudo terminal: {0}")]
    Resize(String),
}

/// Events produced by a running session, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyEvent {
    /// Decoded PTY output, ready to hand to xterm.
    Output(String),
    /// The child exited. No further events follow.
    Exit(i32),
}

/// How to start the shell.
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// Defaults to `$SHELL` on Unix and `%COMSPEC%` on Windows.
    pub shell: Option<String>,
    pub args: Vec<String>,
    /// Defaults to the user's home directory.
    pub cwd: Option<PathBuf>,
    /// Extra variables layered on top of the inherited environment.
    pub env: Vec<(String, String)>,
    pub size: TerminalSize,
    /// Read buffer size. Larger reads mean fewer, bigger frames.
    pub read_buffer_bytes: usize,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            shell: None,
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            size: TerminalSize::default(),
            read_buffer_bytes: 8192,
        }
    }
}

/// The shell to launch when none is configured.
pub fn default_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_owned())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_owned())
    }
}

/// A UTF-8 locale to fall back on when the environment specifies none.
///
/// This matters more than it looks. Under `LC_CTYPE=C`, readline treats typed
/// bytes as 8-bit characters, so entering anything non-ASCII — Japanese, an
/// accented letter, an emoji — makes the shell ring the bell and fire filename
/// completion instead of inserting the text. Daemons and CI runners routinely
/// have no locale set at all, so without this the terminal is unusable for
/// exactly the users who need multi-byte input.
///
/// Returns `None` when the environment already specifies one, since an
/// explicit choice by the operator always wins.
fn default_locale() -> Option<(&'static str, &'static str)> {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if std::env::var_os(key).is_some_and(|value| !value.is_empty()) {
            return None;
        }
    }

    // macOS understands the bare `UTF-8` charmap; glibc and musl want a full
    // locale name, and `C.UTF-8` is the one that is always present.
    #[cfg(target_os = "macos")]
    {
        Some(("LC_CTYPE", "UTF-8"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(("LC_CTYPE", "C.UTF-8"))
    }
}

fn default_cwd() -> Option<PathBuf> {
    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME");
    home.map(PathBuf::from)
}

fn to_pty_size(size: TerminalSize) -> PtySize {
    let size = size.clamped();
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// A live pseudo terminal with a shell running on it.
///
/// Dropping the session kills the child, so neither frontend can leak shells
/// when a socket or window goes away.
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    shell: String,
    size: Arc<Mutex<TerminalSize>>,
}

impl PtySession {
    /// Spawns the shell and starts streaming its output into the returned channel.
    pub fn spawn(config: PtyConfig) -> Result<(Self, UnboundedReceiver<PtyEvent>), PtyError> {
        let shell = config.shell.clone().unwrap_or_else(default_shell);
        let size = config.size.clamped();

        let pair = native_pty_system()
            .openpty(to_pty_size(size))
            .map_err(|err| PtyError::OpenPty(err.to_string()))?;

        let mut command = CommandBuilder::new(&shell);
        for arg in &config.args {
            command.arg(arg);
        }
        command.cwd(
            config
                .cwd
                .clone()
                .or_else(default_cwd)
                .unwrap_or_else(|| PathBuf::from(".")),
        );

        // Programs decide their capabilities from TERM; without this they fall
        // back to a dumb terminal and colour and cursor addressing stop working.
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        if let Some((key, value)) = default_locale() {
            command.env(key, value);
        }
        for (key, value) in &config.env {
            command.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| PtyError::Spawn {
                shell: shell.clone(),
                reason: err.to_string(),
            })?;

        // The slave handle must go before the child can see EOF on exit.
        drop(pair.slave);

        let killer = child.clone_killer();
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| PtyError::OpenPty(err.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| PtyError::OpenPty(err.to_string()))?;

        let (tx, rx) = unbounded_channel();
        spawn_reader_thread(reader, child, tx, config.read_buffer_bytes);

        Ok((
            Self {
                master: pair.master,
                writer,
                killer,
                shell,
                size: Arc::new(Mutex::new(size)),
            },
            rx,
        ))
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn size(&self) -> TerminalSize {
        *self.size.lock().expect("size mutex poisoned")
    }

    /// Forwards keystrokes to the shell.
    pub fn write(&mut self, data: &str) -> Result<(), PtyError> {
        self.writer
            .write_all(data.as_bytes())
            .map_err(PtyError::Write)?;
        self.writer.flush().map_err(PtyError::Write)
    }

    /// Resizes the PTY, which also sends `SIGWINCH` to the foreground process.
    pub fn resize(&self, size: TerminalSize) -> Result<(), PtyError> {
        let size = size.clamped();
        self.master
            .resize(to_pty_size(size))
            .map_err(|err| PtyError::Resize(err.to_string()))?;
        *self.size.lock().expect("size mutex poisoned") = size;
        Ok(())
    }

    /// Terminates the shell. Safe to call more than once.
    pub fn kill(&mut self) {
        if let Err(err) = self.killer.kill() {
            tracing::debug!(%err, "pty child was already gone");
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Bridges the blocking PTY reader onto the async channel the callers consume.
///
/// `portable-pty` only offers a blocking reader, so this has to be a real
/// thread rather than a tokio task.
fn spawn_reader_thread(
    mut reader: Box<dyn Read + Send>,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    tx: UnboundedSender<PtyEvent>,
    buffer_bytes: usize,
) {
    std::thread::spawn(move || {
        let mut buffer = vec![0u8; buffer_bytes.max(1024)];
        let mut decoder = Utf8Stream::new();

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let text = decoder.push(&buffer[..count]);
                    if !text.is_empty() && tx.send(PtyEvent::Output(text)).is_err() {
                        // The consumer hung up; killing the child is `PtySession`'s job.
                        return;
                    }
                }
                Err(err) => {
                    // A closed PTY surfaces as an I/O error on some platforms.
                    tracing::debug!(%err, "pty reader stopped");
                    break;
                }
            }
        }

        let tail = decoder.finish();
        if !tail.is_empty() {
            let _ = tx.send(PtyEvent::Output(tail));
        }

        let exit_code = match child.wait() {
            Ok(status) => status.exit_code() as i32,
            Err(err) => {
                tracing::warn!(%err, "failed to reap pty child");
                -1
            }
        };
        let _ = tx.send(PtyEvent::Exit(exit_code));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[tokio::test]
    async fn streams_output_then_reports_exit() {
        let (session, mut events) = PtySession::spawn(PtyConfig {
            shell: Some("/bin/sh".to_owned()),
            args: vec!["-c".to_owned(), "printf 'ready'".to_owned()],
            ..Default::default()
        })
        .expect("spawn");

        let mut output = String::new();
        let mut exit = None;
        while let Some(event) = events.recv().await {
            match event {
                PtyEvent::Output(text) => output.push_str(&text),
                PtyEvent::Exit(code) => {
                    exit = Some(code);
                    break;
                }
            }
        }

        assert!(output.contains("ready"), "unexpected output: {output:?}");
        assert_eq!(exit, Some(0));
        drop(session);
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn the_shell_gets_a_utf8_locale_and_can_echo_multibyte_text() {
        let (mut session, mut events) = PtySession::spawn(PtyConfig {
            shell: Some("/bin/sh".to_owned()),
            ..Default::default()
        })
        .expect("spawn");

        // Typed non-ASCII only survives if the child is in a UTF-8 locale;
        // under LC_CTYPE=C readline mangles it into completion requests.
        session
            .write("printf '日本語 %s\\n' テスト; exit\n")
            .expect("write");

        let mut output = String::new();
        while let Some(event) = events.recv().await {
            match event {
                PtyEvent::Output(text) => output.push_str(&text),
                PtyEvent::Exit(_) => break,
            }
        }
        assert!(
            output.contains("日本語 テスト"),
            "unexpected output: {output:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn resize_is_visible_to_the_child() {
        let (mut session, mut events) = PtySession::spawn(PtyConfig {
            shell: Some("/bin/sh".to_owned()),
            ..Default::default()
        })
        .expect("spawn");

        session.resize(TerminalSize::new(120, 40)).expect("resize");
        assert_eq!(session.size(), TerminalSize::new(120, 40));

        session.write("stty size; exit\n").expect("write");

        let mut output = String::new();
        while let Some(event) = events.recv().await {
            match event {
                PtyEvent::Output(text) => output.push_str(&text),
                PtyEvent::Exit(_) => break,
            }
        }
        assert!(output.contains("40 120"), "unexpected output: {output:?}");
    }
}
