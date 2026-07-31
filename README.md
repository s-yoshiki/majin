# majin - 魔人🧞

[English](README.md) | [日本語](README.ja.md)

[![CI](https://github.com/s-yoshiki/majin/actions/workflows/ci.yml/badge.svg)](https://github.com/s-yoshiki/majin/actions/workflows/ci.yml)
[![GitHub release](https://img.shields.io/github/v/release/s-yoshiki/majin)](https://github.com/s-yoshiki/majin/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A real login shell on a pseudo terminal, rendered with
[xterm.js](https://xtermjs.org/). Use it as either a self-contained web server
or a native [Tauri](https://tauri.app/) desktop app.

**[Documentation](https://s-yoshiki.github.io/majin/)** ·
**[Releases](https://github.com/s-yoshiki/majin/releases)**

> [!WARNING]
> majin provides an interactive shell with the permissions of the account that
> runs it. The server binds to `127.0.0.1` by default. Read the
> [security notes](https://s-yoshiki.github.io/majin/security.html) before
> making it reachable from another machine.

## Why majin?

- **A real PTY, not a command runner.** Interactive programs, ANSI colours,
  Unicode, window resizing and signals behave as they do in a local terminal.
- **Two delivery modes.** The web build embeds the React app in one Rust binary;
  the desktop build reaches the same PTY implementation over local IPC.
- **Small operational footprint.** Running the web release requires no Node.js
  process, package installation or separate static-file deployment.
- **Shared, type-safe protocol.** Rust is the source of truth for every frame
  and generates its TypeScript mirror, preventing the two sides from drifting.
- **Security-conscious defaults.** Loopback binding, generated access tokens,
  `HttpOnly` sessions, origin checks and sign-in rate limiting are built in.

## Choose a build

| | Web server | Desktop app |
| --- | --- | --- |
| Best for | Reaching a shell from a browser | A local terminal in a native window |
| Transport | HTTP + WebSocket | Tauri IPC |
| Network listener | `127.0.0.1:8999` by default | None |
| Authentication | Access token exchanged for a session cookie | Not needed |
| Distribution | One binary with the web UI embedded | `.dmg`, `.msi`, `.AppImage` and other platform packages |

## Quick start

### Web server

Download and extract the server archive for your platform from
[Releases](https://github.com/s-yoshiki/majin/releases), then run:

```bash
./majin
```

On Windows, run `majin.exe`. The server prints a one-time sign-in URL and an
access token:

```text
majin 0.2.0 — terminal over the web

Open:  http://127.0.0.1:8999/#token=1f3c…
Token: 1f3c…
```

Open the URL in a browser. The token is sent in a URL fragment, exchanged for
an `HttpOnly` session cookie, and then removed from the address bar.

Useful overrides:

```bash
# Pick a shell and working directory
majin --shell /bin/bash --cwd /srv/project

# Use a stable token (recommended for a long-running instance)
MAJIN_TOKEN="$(openssl rand -hex 24)" majin

# See every flag and its environment-variable counterpart
majin --help
```

### Desktop app

Install the package for your platform from
[Releases](https://github.com/s-yoshiki/majin/releases). The desktop app opens
the local login shell directly over IPC; it does not start a server or listen
on a port.

Release builds are currently unsigned, so macOS and Windows may ask you to
confirm the first launch.

## Build from source

You need:

| Tool | Version |
| --- | --- |
| Node.js | 20+ |
| pnpm | 11+ |
| Rust | 1.86+ |

```bash
git clone https://github.com/s-yoshiki/majin.git
cd majin
pnpm install
pnpm dev
```

Open <http://localhost:5173>. This development command starts the Rust server
on port `8999` and Vite on port `5173`; paste the token printed by the server
into the Vite app.

For desktop development, install the
[Tauri platform prerequisites](https://tauri.app/start/prerequisites/) and run:

```bash
pnpm dev:desktop
```

See the
[development guide](https://s-yoshiki.github.io/majin/development.html) for
platform packages, generated protocol types and the full development workflow.

## Configuration

Every server option is available as both a command-line flag and an environment
variable. Flags take precedence.

| Flag | Environment variable | Default | Purpose |
| --- | --- | --- | --- |
| `--host` | `MAJIN_HOST` | `127.0.0.1` | Bind address |
| `--port`, `-p` | `MAJIN_PORT` | `8999` | Listening port |
| `--token` | `MAJIN_TOKEN` | Generated | Access token |
| `--shell` | `MAJIN_SHELL` | `$SHELL` / `%COMSPEC%` | Shell executable |
| `--cwd` | `MAJIN_CWD` | User home | Initial working directory |
| `--max-sessions` | `MAJIN_MAX_SESSIONS` | `8` | Concurrent terminal limit |
| `--session-ttl-minutes` | `MAJIN_SESSION_TTL_MINUTES` | `720` | Session lifetime |

The [configuration reference](https://s-yoshiki.github.io/majin/configuration.html)
also covers allowed origins, secure cookies, logging and authentication options.

## Security model

The web server gives authenticated clients the same shell access as its OS
process. It does **not** provide TLS, user isolation, sandboxing or per-user
accounts.

For anything beyond local-only use:

- terminate TLS in a reverse proxy;
- run majin as a dedicated, unprivileged OS user;
- set a stable, random `MAJIN_TOKEN`;
- keep the server on loopback when the proxy is on the same host;
- review cookie and allowed-origin settings.

Do not expose `--insecure-no-auth` to an untrusted network. See
[Security](https://s-yoshiki.github.io/majin/security.html) and
[Deploying the web build](https://s-yoshiki.github.io/majin/deployment.html)
before deployment. Report vulnerabilities through a
[private security advisory](https://github.com/s-yoshiki/majin/security/advisories/new).

## Architecture

```text
Browser ── WebSocket ── majin-server ─┐
                                      ├── majin-pty ── login shell
Desktop ─── Tauri IPC ────────────────┘
        │
        └── shared terminal-ui (React + xterm.js)
```

```text
crates/
├── majin-protocol/    Rust wire types and TypeScript type generation
├── majin-pty/         PTY lifecycle, I/O and resize handling
└── majin-server/      axum HTTP/WebSocket server and embedded frontend
apps/
├── web/               Browser frontend
└── desktop/           Tauri v2 desktop app
packages/
├── protocol/          Generated types and transport implementations
└── terminal-ui/       Shared xterm.js React component
configs/               Shared TypeScript and Biome configuration
docs/                  VitePress documentation site
```

Read the [architecture guide](https://s-yoshiki.github.io/majin/architecture.html)
and [wire protocol reference](https://s-yoshiki.github.io/majin/protocol.html)
for the details.

## Development commands

| Command | What it does |
| --- | --- |
| `pnpm dev` | Run the Rust server and Vite web app |
| `pnpm dev:desktop` | Run the Tauri app with frontend hot reload |
| `pnpm build:web` | Build the frontend and the server binary that embeds it |
| `pnpm build:desktop` | Build platform installers in `target/release/bundle/` |
| `pnpm lint` | Run Biome and Clippy with warnings denied |
| `pnpm typecheck` | Type-check every TypeScript package and Rust crate |
| `pnpm test` | Run the Rust workspace tests, including real-PTY tests |
| `pnpm test:e2e` | Test authentication, WebSocket transport and a shell end to end |
| `pnpm generate:protocol` | Regenerate TypeScript protocol types from Rust |

Before opening a pull request, run `pnpm lint`, `pnpm typecheck` and
`pnpm test`. If you change `crates/majin-protocol`, regenerate and commit the
TypeScript types as well.

## Documentation

| Page | Contents |
| --- | --- |
| [Overview](https://s-yoshiki.github.io/majin/) | What majin is and how to start |
| [Development](https://s-yoshiki.github.io/majin/development.html) | Monorepo setup and development loops |
| [Deployment](https://s-yoshiki.github.io/majin/deployment.html) | systemd, reverse proxies and Docker |
| [Desktop](https://s-yoshiki.github.io/majin/desktop.html) | Installing and building the desktop app |
| [Architecture](https://s-yoshiki.github.io/majin/architecture.html) | Components and data flow |
| [Wire protocol](https://s-yoshiki.github.io/majin/protocol.html) | Frames, endpoints and close codes |
| [Security](https://s-yoshiki.github.io/majin/security.html) | Authentication and trust boundaries |
| [Configuration](https://s-yoshiki.github.io/majin/configuration.html) | Flags and environment variables |

## License

[MIT](LICENSE)
