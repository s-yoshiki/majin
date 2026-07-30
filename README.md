# majin

A real login shell on a pseudo terminal, rendered with [xterm.js](https://xtermjs.org/).
Ships two ways: a single self-contained web server binary, and a Tauri desktop app.

**[Documentation](https://s-yoshiki.github.io/majin/)** ·
**[Releases](https://github.com/s-yoshiki/majin/releases)**

> [!WARNING]
> This program hands out an interactive shell. The server binds to `127.0.0.1` by
> default; read the [security notes](https://s-yoshiki.github.io/majin/security.html)
> before it is reachable from anywhere else.

## Quick start

### Web

Download the binary for your platform from
[Releases](https://github.com/s-yoshiki/majin/releases) and run it:

```bash
./majin
```

It prints a sign-in link containing a freshly generated access token. There is no
`npm install`, no Node runtime and no static directory to deploy — the React app is
compiled into the binary.

### Desktop

Install the `.dmg`, `.msi` or `.AppImage` from
[Releases](https://github.com/s-yoshiki/majin/releases). Nothing listens on a
port; the window talks to a local PTY over IPC.

### From source

```bash
pnpm install
pnpm dev
```

Then open <http://localhost:5173>. Needs Node.js 20+, pnpm 11+ and Rust 1.86+.

## How it fits together

```
crates/
├── majin-protocol/    Rust types for every frame; generates the TypeScript mirror
├── majin-pty/         PTY spawn, read, write, resize — shared by both builds
└── majin-server/      axum HTTP + WebSocket, token auth, embedded frontend
apps/
├── web/               React app, compiled into the server binary
└── desktop/           Tauri v2 app; src-tauri calls majin-pty directly
packages/
├── protocol/          Generated types, transport interface, WebSocket transport
└── terminal-ui/       The xterm.js React component both apps render
configs/
├── tsconfig/          Shared TypeScript configs
└── biome/             Shared Biome config
docs/                  The documentation site (static HTML, GitHub Pages)
```

Both builds reach the same `majin-pty` crate. The terminal UI is one React component
that only knows about a `TerminalTransport` interface — the web app injects a
WebSocket implementation, the desktop app injects a Tauri IPC one — which is why
there is no second copy of anything.

The wire protocol is defined once in Rust and the TypeScript types are generated from
it, so a rename on one side is a compile error on the other. This is not incidental:
the previous version hand-maintained both halves and got it wrong, sending
`{ resizer: [...] }` from the browser while the server read `msg.resize`. Window
resizing silently did nothing, and nothing caught it.

## Commands

| Command | Does |
| --- | --- |
| `pnpm dev` | Rust server on `:8999` and Vite on `:5173` |
| `pnpm dev:desktop` | Tauri window with frontend hot reload |
| `pnpm build:web` | Frontend, then the server binary that embeds it |
| `pnpm build:desktop` | Platform installers into `target/release/bundle/` |
| `pnpm lint` | Biome on TypeScript, Clippy on Rust, warnings denied |
| `pnpm typecheck` | `tsc --noEmit` per package, `cargo check` per crate |
| `pnpm test` | `cargo test --workspace`, including real-PTY tests |
| `pnpm test:e2e` | Starts the built binary and drives auth, WebSocket and a shell |
| `pnpm generate:protocol` | Regenerates the TypeScript protocol types from Rust |

## Documentation

| Page | |
| --- | --- |
| [Overview](https://s-yoshiki.github.io/majin/) | What it is and how to start |
| [Development](https://s-yoshiki.github.io/majin/development.html) | Monorepo setup and dev loops |
| [Deploying the web build](https://s-yoshiki.github.io/majin/deployment.html) | systemd, reverse proxies, Docker |
| [Desktop build](https://s-yoshiki.github.io/majin/desktop.html) | Installing and building the app |
| [Architecture](https://s-yoshiki.github.io/majin/architecture.html) | How the pieces relate |
| [Wire protocol](https://s-yoshiki.github.io/majin/protocol.html) | Every frame and endpoint |
| [Security](https://s-yoshiki.github.io/majin/security.html) | Auth model and what it does not do |
| [Configuration](https://s-yoshiki.github.io/majin/configuration.html) | Every flag and environment variable |

## Built with

[xterm.js](https://github.com/xtermjs/xterm.js) ·
[axum](https://github.com/tokio-rs/axum) ·
[portable-pty](https://github.com/wez/wezterm/tree/main/pty) ·
[Tauri](https://tauri.app/) ·
[React](https://react.dev/) ·
[Vite](https://vite.dev/) ·
[Turborepo](https://turborepo.com/) ·
[Biome](https://biomejs.dev/) ·
[ts-rs](https://github.com/Aleph-Alpha/ts-rs)

## License

MIT
