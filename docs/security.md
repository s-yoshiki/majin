# Security

This program hands out an interactive shell. Anyone who reaches it and authenticates can do anything the account running it can do.

::: danger Start here

`majin` binds to `127.0.0.1` by default, and that default exists for a reason. Reaching it from another machine should be a deliberate decision with TLS in front of it, not something that happens because you copied a `--host 0.0.0.0` from somewhere.

:::

## How sign-in works

1.  At startup the server takes the token from `MAJIN_TOKEN`, or generates a 192-bit random one and prints it.
2.  The client posts the token to `POST /api/auth/session`. The server compares SHA-256 digests with a constant-time comparison, so the check leaks neither the token nor its length.
3.  On success the server mints a 256-bit random session id, keeps it in memory, and returns it as an `HttpOnly; SameSite=Strict` cookie.
4.  The WebSocket upgrade is authenticated by that cookie, which the browser attaches automatically.

### Why sessions live in memory

There is no signing key to manage or leak, sessions can be revoked the instant someone signs out, and a restart invalidates everything. For a process that hands out shells, losing sessions on restart is the correct trade.

### Why the token is never in the URL query

Query strings end up in proxy logs, server access logs, browser history and `Referer` headers. A token that grants shell access does not belong in any of them, so the server never accepts one there.

The startup banner instead prints a `#token=…` *fragment* link. Fragments are never sent to a server, so they cannot reach a log. The page reads it, exchanges it for a cookie, and clears it from the address bar with `history.replaceState` before anything else happens.

::: info The remaining trade-off

A fragment is still visible in the address bar and can land in local browser history until it is cleared. If that matters in your environment, set `MAJIN_TOKEN` yourself and paste it into the login form instead of using the link.

:::

## Cross-site request forgery

Cookie-authenticated WebSockets need care: without a check, any page the user visits could open a socket back to `localhost` and get a shell, because the browser would attach the cookie for them.

Two defences, deliberately overlapping:

- `SameSite=Strict` on the session cookie, which stops the browser sending it on cross-site requests.
- An explicit `Origin` check on the upgrade. Same-origin requests pass; anything else must be listed in `--allowed-origin`. Requests with no `Origin` header at all are allowed, since that means a non-browser client and therefore no ambient cookie to abuse.

## Rate limiting

Ten failed sign-ins from one address within a minute locks that address out for the rest of the window, correct token included. It is deliberately simple: the token is 192 bits of randomness, so this exists to stop noise and log spam rather than to be the thing standing between an attacker and your shell.

## What the server does not do

| Not provided | What to do instead |
|----|----|
| TLS | Terminate it in front — Caddy, nginx, a cloud load balancer. Over plain HTTP the session cookie and every keystroke travel in clear text. |
| Multiple users or accounts | There is one token and one shell identity. If you need per-user access, put an authenticating proxy in front and run separate instances. |
| Sandboxing | The shell runs as the user that started the server, with that user's full environment. Run it as a restricted account, or inside a container, if that is not what you want. |
| Audit logging of session contents | Connections and lifecycle events are logged; keystrokes and output are not. |

## Deployment checklist

- Serve it over HTTPS, and set `MAJIN_SECURE_COOKIE=1` if your proxy does not send `X-Forwarded-Proto`.
- Set `MAJIN_TOKEN` explicitly so it survives restarts and is not printed to a shared log.
- Keep the bind address as tight as possible; prefer loopback plus a reverse proxy over binding `0.0.0.0`.
- Run as a dedicated unprivileged user, never root.
- Set `--allowed-origin` only if the UI is served from a different origin than the server.
- Shorten `--session-ttl-minutes` from the 12-hour default if the machine is shared.

## The desktop build

None of the above applies to it. There is no port, no token and no cookie — the PTY is reached over Tauri IPC, and the only thing that can call those commands is the window in that process. Its threat model is the same as any other terminal emulator you install.

## Reporting a vulnerability

Open a [private security advisory](https://github.com/s-yoshiki/majin/security/advisories/new) rather than a public issue.
