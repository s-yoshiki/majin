# majin

A real login shell on a pseudo terminal, rendered with xterm.js. Ships two ways: a single self-contained web server binary, and a Tauri desktop app.

<div class="cards">

<a href="./deployment.html" class="card"><strong>Run it on a server</strong> <span>One binary with the UI baked in. No Node, no npm install.</span></a> <a href="./desktop.html" class="card"><strong>Run it on your desktop</strong> <span>Tauri app talking to a local PTY over IPC. No open ports.</span></a>

</div>

## What it is

majin spawns your login shell on a PTY and streams it to a browser. Everything a terminal emulator needs is handled end to end: window resizes reach the shell as `SIGWINCH`, multi-byte output is reassembled across read boundaries, and the child is killed when the connection goes away.

The PTY layer is one Rust crate, `majin-pty`, shared by both builds. The web server adds an HTTP and WebSocket surface with token authentication; the desktop app skips the network entirely and calls into the same crate over Tauri IPC.

## Quick start

### Web

Download the binary for your platform from [Releases](https://github.com/s-yoshiki/majin/releases) and run it:

    ./majin

It prints a sign-in link containing a freshly generated access token. Open it, and you have a terminal.

      majin 0.2.0 — terminal over the web

      Open:  http://127.0.0.1:8999/#token=1f3c…

      Token: 1f3c…

It binds to `127.0.0.1` by default. Read [Security](./security.md) before exposing it to anything wider.

### Desktop

Grab the `.dmg`, `.msi` or `.AppImage` from [Releases](https://github.com/s-yoshiki/majin/releases), install, launch. There is no configuration and nothing listening on a port.

### From source

    git clone https://github.com/s-yoshiki/majin.git
    cd majin
    pnpm install
    pnpm dev

See [Development](./development.md) for the full workflow.

## Why the rewrite

The original version was an Express server, a bare xterm.js page and two shell scripts. It worked, but the two halves of the protocol were written independently — the browser sent `{ resizer: [...] }` while the server read `msg.resize`, so resizing silently did nothing and nothing caught it.

Now the protocol is defined once, in Rust, and the TypeScript types are generated from it. A rename on one side is a compile error on the other. The rest of the rebuild follows from that: a pnpm and Turborepo monorepo, a shared PTY crate, Biome for formatting and linting, and release artifacts built in CI for both targets.

## Requirements

| To… | You need |
|----|----|
| Run a release binary | Nothing. The frontend is embedded and there is no runtime dependency. |
| Build from source | Node.js 20+, pnpm 11+, Rust 1.86+ |
| Build the desktop app | The above, plus your platform's webview toolchain (Xcode CLT on macOS, WebView2 on Windows, `libwebkit2gtk-4.1-dev` on Linux) |
