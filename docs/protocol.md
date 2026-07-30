# Wire protocol

JSON text frames over a WebSocket, tagged by a `type` field. Version 1.

::: info Generated, not written twice

The types below are defined in `crates/majin-protocol/src/lib.rs`. Run `pnpm generate:protocol` to regenerate `packages/protocol/src/generated/protocol.ts`; CI fails if the committed output does not match.

:::

## Client to server

| Frame | Meaning |
|----|----|
| `{"type":"input","data":"ls\r"}` | Keystrokes or pasted text, written straight to the PTY. |
| `{"type":"resize","cols":120,"rows":40}` | New viewport geometry. Resizing the PTY also raises `SIGWINCH` in the foreground process. Values are clamped to 1–1000 on both axes. |
| `{"type":"ping","at":1712345678901}` | Liveness probe. The server echoes `at` back in a `pong`. |

## Server to client

| Frame | Meaning |
|----|----|
| `{"type":"ready","protocolVersion":1,"sessionId":"…","shell":"/bin/zsh","cols":80,"rows":24}` | Sent once, immediately after the shell is spawned. |
| `{"type":"output","data":"…"}` | Decoded PTY output, written verbatim into xterm. |
| `{"type":"exit","exitCode":0}` | The shell terminated. The socket closes right after. |
| `{"type":"error","code":"…","message":"…"}` | A failure worth showing the user. See the codes below. |
| `{"type":"pong","at":1712345678901}` | Reply to a `ping`. |

### Error codes

| Code | Cause |
|----|----|
| `unauthorized` | No valid session cookie, or the request origin was refused. |
| `spawn_failed` | The configured shell could not be started. |
| `session_limit` | `--max-sessions` terminals are already open. |
| `bad_message` | A frame failed validation. Malformed frames are dropped, not fatal. |
| `internal` | Anything else. |

## Close codes

Browsers cannot read the HTTP status of a failed WebSocket handshake, so the server always completes the upgrade and then closes with a code the client can act on.

| Code   | Name          | Client behaviour                                      |
|--------|---------------|-------------------------------------------------------|
| `1000` | normal        | Stop. Session ended cleanly.                          |
| `4001` | unauthorized  | Return to the login screen. Reconnecting cannot help. |
| `4002` | sessionLimit  | Show the limit; do not hammer the server.             |
| `4003` | protocolError | Stop.                                                 |
| `4004` | shellExited   | Stop. The shell is gone, not the connection.          |

Anything else is treated as a transient network failure and retried with exponential backoff, capped at 8 attempts and 10 seconds between them.

## HTTP endpoints

| Endpoint | Returns |
|----|----|
| `GET /api/info` | `{ version, protocolVersion, authRequired }`. Public — the UI needs it before sign-in. |
| `GET /api/auth/session` | `{ authenticated, expiresAt }` for the current cookie. |
| `POST /api/auth/session` | Body `{ token }`. On success sets the session cookie and returns the same shape. `401` on a bad token, `429` when rate limited. |
| `DELETE /api/auth/session` | Revokes the session server-side and clears the cookie. |
| `GET /ws` | WebSocket upgrade. One connection, one PTY. |

Errors carry `{ error, message }`; `message` is safe to display.

## Validation

Frames off a socket are untrusted. Rust rejects unknown shapes at deserialization, and the TypeScript side runs the guards in `packages/protocol/src/codec.ts` before anything reaches the terminal. A malformed frame is dropped and logged, never fatal — a buggy client should not be able to kill someone else's shell.
