# Architecture

One PTY implementation, one protocol definition, two frontends that share a React component and differ only in how bytes get to the shell.

## The shape of it

![Browser and desktop frontends converge on the shared majin-pty crate.](/architecture.svg)

*Both paths converge on the same crate. The desktop build has no server, no port and no authentication because there is nothing between the window and the PTY.*

## Repository layout

    majin/
    ├── crates/
    │   ├── majin-protocol/    Rust types for every frame; generates the TS mirror
    │   ├── majin-pty/         PTY spawn, read, write, resize — shared by both builds
    │   └── majin-server/      axum HTTP + WebSocket, auth, embedded frontend
    ├── apps/
    │   ├── web/               React app, built into the server binary
    │   └── desktop/           Tauri v2 app
    │       └── src-tauri/     Rust side: IPC commands over majin-pty
    ├── packages/
    │   ├── protocol/          Generated types + transport interface + WebSocket transport
    │   └── terminal-ui/       The xterm.js React component both apps render
    ├── configs/
    │   ├── tsconfig/          Shared TypeScript configs
    │   └── biome/             Shared Biome config
    └── docs/                  This site

## The transport seam

`TerminalView` in `packages/terminal-ui` is the entire terminal UI, and it knows nothing about WebSockets or Tauri. It takes a `TerminalTransport`:

    interface TerminalTransport {
      readonly state: TransportState;
      connect(): void;
      send(message: ClientMessage): void;
      onMessage(listener: (message: ServerMessage) => void): Unsubscribe;
      onStateChange(listener: (state: TransportState) => void): Unsubscribe;
      dispose(): void;
    }

The web app injects `WebSocketTransport`; the desktop app injects `TauriTransport`. That single interface is why there is one terminal component rather than two that drift apart.

## Types are generated, not mirrored by hand

`crates/majin-protocol` is the source of truth. Running `pnpm generate:protocol` executes a small Rust binary that emits `packages/protocol/src/generated/protocol.ts` via [ts-rs](https://docs.rs/ts-rs). CI regenerates and fails on any diff, so the two views of the protocol cannot drift.

::: info Why this exists

The previous version hand-maintained both sides and got it wrong: the client sent a `resizer` field, the server read `resize`, and window resizing quietly did nothing for as long as that code lived.

:::

## Reading from a PTY

`portable-pty` only offers a blocking reader, so `majin-pty` runs it on a dedicated thread and forwards decoded chunks over a `tokio` channel. Two details matter:

- **UTF-8 across chunk boundaries.** Reads land on arbitrary byte offsets, so a multi-byte character can straddle two of them. Decoding each chunk on its own would corrupt any non-ASCII output — CJK text and box-drawing characters break first and most visibly. `Utf8Stream` holds back a partial tail until the rest arrives, and substitutes U+FFFD for genuinely invalid bytes rather than stalling.
- **Lifetime.** Dropping a `PtySession` kills its child, so a closed tab or window never leaves an orphaned shell behind.

## Build orchestration

Turborepo drives both languages. Each Rust crate has a thin `package.json` wrapping `cargo`, which lets the task graph express the one ordering constraint that actually matters: `apps/web` must be built before `crates/majin-server`, because [rust-embed](https://docs.rs/rust-embed) bakes `apps/web/dist` into the binary at compile time.

That dependency is declared the way turbo understands it — `majin-server`'s `package.json` lists `@majin/web` as a devDependency.
