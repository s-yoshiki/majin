# Deploying the web build

The release artifact is one executable with the React app compiled into it. Copy it and run it.

::: danger Read this first

You are deploying remote shell access. Work through the [Security](./security.md) page before it is reachable from anywhere but loopback.

:::

## Getting the binary

Download the archive for your platform from [Releases](https://github.com/s-yoshiki/majin/releases):

| Platform            | Asset                                              |
|---------------------|----------------------------------------------------|
| Linux x86-64        | `majin-<version>-x86_64-unknown-linux-gnu.tar.gz`  |
| Linux ARM64         | `majin-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple silicon | `majin-<version>-aarch64-apple-darwin.tar.gz`      |
| macOS Intel         | `majin-<version>-x86_64-apple-darwin.tar.gz`       |
| Windows x86-64      | `majin-<version>-x86_64-pc-windows-msvc.zip`       |

    tar xzf majin-0.2.0-x86_64-unknown-linux-gnu.tar.gz
    sudo install -m 0755 majin /usr/local/bin/

### Or build it

    pnpm install
    pnpm build:web
    # → target/release/majin

The frontend must be built first so `rust-embed` can find it; `pnpm build:web` handles that ordering. Building the Rust server alone with `cargo build --release -p majin-server` will produce a binary that serves nothing, and says so at startup.

## Running it as a service

A dedicated unprivileged user, a fixed token, and loopback only — with TLS terminated by a proxy in front.

    sudo useradd --system --create-home --shell /bin/bash majin
    sudo install -d -o majin -g majin /etc/majin
    printf 'MAJIN_TOKEN=%s\n' "$(openssl rand -hex 24)" | sudo tee /etc/majin/env >/dev/null
    sudo chmod 600 /etc/majin/env
    sudo chown majin:majin /etc/majin/env

`/etc/systemd/system/majin.service`:

    [Unit]
    Description=majin terminal server
    After=network.target

    [Service]
    User=majin
    EnvironmentFile=/etc/majin/env
    Environment=MAJIN_HOST=127.0.0.1
    Environment=MAJIN_PORT=8999
    Environment=MAJIN_SECURE_COOKIE=1
    ExecStart=/usr/local/bin/majin
    Restart=on-failure
    RestartSec=2

    [Install]
    WantedBy=multi-user.target

    sudo systemctl daemon-reload
    sudo systemctl enable --now majin
    sudo systemctl status majin

::: info Restarts end sessions

Sessions are held in memory, so everyone signs in again after a restart. Shells are killed too — they are children of the server process.

:::

## Reverse proxy

The proxy must forward WebSocket upgrades and pass `X-Forwarded-Proto`, which is how the server decides to mark the session cookie `Secure`.

### Caddy

    majin.example.com {
        reverse_proxy 127.0.0.1:8999
    }

Caddy handles certificates, upgrades and forwarded headers with no further configuration.

### nginx

    server {
        listen 443 ssl http2;
        server_name majin.example.com;

        ssl_certificate     /etc/letsencrypt/live/majin.example.com/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/majin.example.com/privkey.pem;

        location / {
            proxy_pass http://127.0.0.1:8999;
            proxy_http_version 1.1;

            # Without these two the WebSocket upgrade fails and the terminal
            # never connects.
            proxy_set_header Upgrade    $http_upgrade;
            proxy_set_header Connection "upgrade";

            proxy_set_header Host              $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header X-Real-IP         $remote_addr;

            # A terminal can idle for a long time between keystrokes.
            proxy_read_timeout 3600s;
            proxy_send_timeout 3600s;
        }
    }

::: warning Keep the Host header intact

The origin check compares the request's `Origin` against its `Host`. A proxy that rewrites `Host` to `127.0.0.1:8999` will make every browser connection look cross-origin and get closed with 4001. Either preserve `Host`, or list the public origin in `MAJIN_ALLOWED_ORIGINS`.

:::

## Docker

There is no published image; the binary is static enough that one is rarely worth it. If you want one:

    FROM debian:bookworm-slim
    RUN apt-get update \
     && apt-get install -y --no-install-recommends ca-certificates bash \
     && rm -rf /var/lib/apt/lists/* \
     && useradd --create-home --shell /bin/bash majin

    COPY majin /usr/local/bin/majin

    USER majin
    ENV MAJIN_HOST=0.0.0.0 MAJIN_PORT=8999 MAJIN_SHELL=/bin/bash
    EXPOSE 8999
    CMD ["majin"]

    docker run --rm -p 8999:8999 -e MAJIN_TOKEN="$(openssl rand -hex 24)" majin

`MAJIN_HOST=0.0.0.0` is correct inside a container — the container boundary is what limits reachability. Publish the port only where you intend it to be reachable.

## Checking it works

    curl -s https://majin.example.com/api/info

    {"version":"0.2.0","protocolVersion":1,"authRequired":true}

`authRequired: false` here means the server is running with `--insecure-no-auth` and anyone who can reach it has a shell.

## When something is wrong

| Symptom | Likely cause |
|----|----|
| Login works, terminal stays "Reconnecting…" | The proxy is not forwarding the WebSocket upgrade. |
| Immediately bounced back to the login screen | Origin check failing (rewritten `Host`), or a `Secure` cookie set on a connection the browser sees as plain HTTP. |
| "no frontend is embedded in this binary" | Built with `cargo` directly instead of `pnpm build:web`. |
| Connection drops after a few minutes idle | Proxy read timeout too low. |

Raise the log level with `MAJIN_LOG=majin_server=debug`.
