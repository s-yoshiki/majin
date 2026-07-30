# Development

A pnpm workspace and a Cargo workspace in the same repository, with Turborepo running both.

## Prerequisites

| Tool    | Version | Needed for           |
|---------|---------|----------------------|
| Node.js | 20+     | Frontend tooling     |
| pnpm    | 11+     | Workspace management |
| Rust    | 1.86+   | Everything backend   |

Building the desktop app additionally needs your platform's webview toolchain: Xcode Command Line Tools on macOS, the WebView2 runtime on Windows, and `libwebkit2gtk-4.1-dev` plus `libgtk-3-dev` on Linux. See the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

## Setup

    git clone https://github.com/s-yoshiki/majin.git
    cd majin
    pnpm install

## The web dev loop

    pnpm dev

This runs two processes: the Rust server on `:8999` and the Vite dev server on `:5173`. Work against `http://localhost:5173` — Vite proxies `/api` and `/ws` through to the backend, and the server is started with `--allowed-origin http://localhost:5173` so the origin check accepts it.

The server prints its access token on startup, along with a link that signs you in directly. Note that the printed link points at `:8999`; when using the Vite server, copy the token into the login form on `:5173` instead.

### Against the built frontend

To exercise the real embedded-asset path rather than the dev server:

    pnpm build:web
    ./target/release/majin

In a debug build, `rust-embed` reads from `apps/web/dist` at runtime, so `cargo run` picks up frontend rebuilds without recompiling Rust. Release builds bake the files in.

## The desktop dev loop

    pnpm dev:desktop

Tauri starts Vite on `:5174` and opens the window against it, with hot reload for the frontend. Changing Rust triggers a rebuild and restart.

## Changing the protocol

Edit the types in `crates/majin-protocol/src/lib.rs`, then:

    pnpm generate:protocol

This rewrites `packages/protocol/src/generated/protocol.ts` and formats it. Commit the result — CI regenerates and fails on a diff. Runtime guards in `packages/protocol/src/codec.ts` are hand-written and need updating in the same change.

## Checks

| Command | What it does |
|----|----|
| `pnpm lint` | Biome on the TypeScript, Clippy on the Rust, warnings denied |
| `pnpm lint:fix` | Applies what both can fix automatically |
| `pnpm format` | Biome formatter and `cargo fmt` |
| `pnpm typecheck` | `tsc --noEmit` per package, `cargo check` per crate |
| `pnpm test` | `cargo test --workspace`. Includes tests that spawn a real PTY and assert on what the shell sees. |
| `pnpm build` | Everything, in dependency order |

## Where things live

| To change… | Edit |
|----|----|
| How the terminal looks or behaves, in both apps | `packages/terminal-ui/src/TerminalView.tsx` |
| The colour scheme | `packages/terminal-ui/src/theme.ts` |
| Shell spawning, resizing, output decoding | `crates/majin-pty/src/lib.rs` |
| Authentication | `crates/majin-server/src/auth.rs` |
| Server routes | `crates/majin-server/src/main.rs` |
| Desktop IPC commands | `apps/desktop/src-tauri/src/lib.rs` |
| The app icon | `apps/desktop/scripts/generate-icon.mjs`, then `pnpm --filter @majin/desktop icon` |

## Shared configuration

TypeScript and Biome settings live in `configs/` as workspace packages rather than being copied per app.

    configs/tsconfig/base.json        strict defaults, no DOM
    configs/tsconfig/node.json        + Node types
    configs/tsconfig/react-lib.json   + DOM and JSX
    configs/tsconfig/react-app.json   + Vite client types
    configs/biome/base.json           formatter and lint rules

A package opts in by extending it:

    { "extends": "@majin/tsconfig/react-app.json", "include": ["src"] }

::: info One thing to know about Biome

The shared config is `base.json`, not `biome.json`, on purpose. Biome auto-discovers any file named `biome.json` and would treat the shared one as a competing root configuration.

:::
