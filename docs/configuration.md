# Configuration

Every option is available as both a flag and an environment variable. Flags win.

Run `majin --help` for the same list from the binary.

## Network

| Flag | Environment | Default | Notes |
|----|----|----|----|
| `--host` | `MAJIN_HOST` | `127.0.0.1` | Loopback by default on purpose. Widening it should be deliberate — see [Security](./security.md). |
| `--port`, `-p` | `MAJIN_PORT` | `8999` | Use `0` to let the OS pick; the banner prints what it got. |
| `--allowed-origin` | `MAJIN_ALLOWED_ORIGINS` | — | Extra origins permitted to open a WebSocket. Same-origin always passes, so this is only for a UI served from elsewhere, such as the Vite dev server. Repeat the flag, or comma-separate the variable. |

## Authentication

| Flag | Environment | Default | Notes |
|----|----|----|----|
| `--token` | `MAJIN_TOKEN` | generated | A fresh 192-bit token is generated and printed when unset, which means it changes on every restart. Set it explicitly for anything long-lived. |
| `--session-ttl-minutes` | `MAJIN_SESSION_TTL_MINUTES` | `720` | How long a session stays valid. 12 hours by default. |
| `--secure-cookie` | `MAJIN_SECURE_COOKIE` | `false` | Forces `Secure` on the session cookie. Detected automatically from `X-Forwarded-Proto: https`; set it when your proxy omits that header. |
| `--insecure-no-auth` | `MAJIN_INSECURE_NO_AUTH` | `false` | Disables authentication entirely. Only defensible when something else in front is doing it. |

::: danger --insecure-no-auth

With this on, anyone who can open a socket to the port has a shell as the user running the server. The startup banner says so in as many words.

:::

## Sessions and the shell

| Flag | Environment | Default | Notes |
|----|----|----|----|
| `--shell` | `MAJIN_SHELL` | `$SHELL` | `%COMSPEC%` on Windows, `/bin/bash` as a last resort. |
| `--cwd` | `MAJIN_CWD` | home directory | Working directory for new sessions. |
| `--max-sessions` | `MAJIN_MAX_SESSIONS` | `8` | Concurrent terminals. Further connections are closed with `4002`. |

Spawned shells inherit the server's environment plus `TERM=xterm-256color` and `COLORTERM=truecolor`. Without those, programs assume a dumb terminal and colour and cursor addressing stop working.

## Logging

| Environment | Default | Notes |
|----|----|----|
| `MAJIN_LOG` | `majin_server=info,tower_http=warn` | [EnvFilter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax. Terminal contents are never logged; only connections and lifecycle events are. |

## Examples

Local use, nothing to configure:

    majin

Behind a TLS-terminating proxy, with a fixed token:

    MAJIN_TOKEN="$(openssl rand -hex 24)" \
    MAJIN_SECURE_COOKIE=1 \
    majin --host 127.0.0.1 --port 8999

Backing the Vite dev server:

    majin --allowed-origin http://localhost:5173

A single restricted session in a container:

    majin --host 0.0.0.0 --shell /bin/bash --max-sessions 1 \
                 --session-ttl-minutes 60
