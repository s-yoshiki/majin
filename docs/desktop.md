# Desktop build

The same terminal UI in a native window, talking to a local PTY over Tauri IPC. No server, no port, no token.

## Installing

Download from [Releases](https://github.com/s-yoshiki/majin/releases):

| Platform            | Asset                                      |
|---------------------|--------------------------------------------|
| macOS Apple silicon | `majin_<version>_aarch64.dmg`              |
| macOS Intel         | `majin_<version>_x64.dmg`                  |
| Windows             | `majin_<version>_x64-setup.exe` or `.msi`  |
| Linux               | `majin_<version>_amd64.AppImage` or `.deb` |

::: warning Unsigned builds

Releases are not code-signed or notarised. macOS will refuse the first launch — right-click the app and choose Open, or clear the quarantine flag with `xattr -dr com.apple.quarantine /Applications/majin.app`. Windows SmartScreen will want "More info" then "Run anyway".

:::

## How it differs from the web build

|  | Web | Desktop |
|----|----|----|
| Transport | WebSocket over HTTP | Tauri IPC |
| Authentication | Token, then a session cookie | None — nothing external can reach the PTY |
| Network exposure | A listening port | None |
| PTY implementation | `majin-pty`, identical in both |  |
| Terminal UI | `@majin/terminal-ui`, identical in both |  |

## Building it yourself

Beyond Node, pnpm and Rust you need your platform's webview toolchain; see the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

    pnpm install
    pnpm build:desktop

Installers land in `target/release/bundle/`. Development runs with hot reload for the frontend:

    pnpm dev:desktop

::: info If DMG bundling fails locally on macOS

Tauri styles the disk-image window by driving Finder with AppleScript, which fails over SSH, in CI containers, and anywhere Automation permission has not been granted — usually as an AppleEvent timeout (`-1712`). The `.app` in `target/release/bundle/macos/` is complete and runnable regardless; only the `.dmg` wrapper is missing.

:::

## How the Rust side is wired

Four commands and two events, in `apps/desktop/src-tauri/src/lib.rs`:

| Command | Does |
|----|----|
| `pty_open(cols, rows)` | Spawns the shell and returns `{ shell, cols, rows }`. Calling it again replaces the previous session rather than leaking it. |
| `pty_write(data)` | Forwards keystrokes. |
| `pty_resize(cols, rows)` | Resizes the PTY. Ignored when no session is open, since a resize can race ahead of `pty_open`. |
| `pty_close()` | Kills the shell. |

Output arrives on the `pty://output` event and termination on `pty://exit`. `TauriTransport` adapts those into the same `ServerMessage` frames the web build receives, which is why `TerminalView` cannot tell the two apart.

## Which shell it runs

`$SHELL` on macOS and Linux, `%COMSPEC%` on Windows, started in your home directory with `TERM=xterm-256color`.

::: info Heavy shell configs

Your full startup files run, including any that prompt interactively. A plugin manager that asks a question at startup will swallow the first keystrokes you type — that is your shell config, not the terminal.

:::
