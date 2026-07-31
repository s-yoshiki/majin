# Web版のデプロイ

リリース成果物はReactアプリを組み込んだ1つの実行ファイルです。コピーして実行してください。

::: danger まずこれを読んでください

リモートからシェルへアクセスできる状態をデプロイします。ループバック以外から到達可能にする前に、[セキュリティ](./security.md)ページを確認してください。

:::

## バイナリを入手する

[Releases](https://github.com/s-yoshiki/majin/releases)からプラットフォーム用のアーカイブをダウンロードします。

| プラットフォーム | ファイル |
|---------------------|---------------------|
| Linux x86-64 | `majin-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `majin-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple silicon | `majin-<version>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `majin-<version>-x86_64-apple-darwin.tar.gz` |
| Windows x86-64 | `majin-<version>-x86_64-pc-windows-msvc.zip` |

    tar xzf majin-0.2.0-x86_64-unknown-linux-gnu.tar.gz
    sudo install -m 0755 majin /usr/local/bin/

### 自分でビルドする場合

    pnpm install
    pnpm build:web
    # → target/release/majin

`rust-embed`がフロントエンドを見つけられるよう、先にフロントエンドをビルドする必要があります。`pnpm build:web`なら順序も処理されます。Rustサーバーだけを`cargo build --release -p majin-server`でビルドすると、何も配信しないバイナリになり、起動時にもその旨が表示されます。

## サービスとして実行する

専用の権限のないユーザー、固定トークン、ループバックのみを使い、前段のプロキシでTLSを終端します。

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

::: info 再起動するとセッションが終了します

セッションはメモリ上に保持されるため、再起動後は全員が再度サインインする必要があります。シェルもサーバープロセスの子なので終了します。

:::

## リバースプロキシ

プロキシはWebSocketのアップグレードを転送し、`X-Forwarded-Proto`を渡す必要があります。サーバーはこの値を使ってセッションCookieに`Secure`を付けるか判断します。

### Caddy

    majin.example.com {
        reverse_proxy 127.0.0.1:8999
    }

Caddyなら、追加設定なしで証明書、アップグレード、転送ヘッダーを処理します。

### nginx

    server {
        listen 443 ssl http2;
        server_name majin.example.com;

        ssl_certificate     /etc/letsencrypt/live/majin.example.com/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/majin.example.com/privkey.pem;

        location / {
            proxy_pass http://127.0.0.1:8999;
            proxy_http_version 1.1;

            # この2つがないとWebSocketのアップグレードに失敗し、
            # 端末は接続できません。
            proxy_set_header Upgrade    $http_upgrade;
            proxy_set_header Connection "upgrade";

            proxy_set_header Host              $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header X-Real-IP         $remote_addr;

            # キー入力の間隔が長くなることがあります。
            proxy_read_timeout 3600s;
            proxy_send_timeout 3600s;
        }
    }

::: warning Hostヘッダーを維持してください

オリジンチェックはリクエストの`Origin`と`Host`を比較します。プロキシが`Host`を`127.0.0.1:8999`に書き換えると、すべてのブラウザー接続がクロスオリジンと判定され、`4001`で切断されます。`Host`を維持するか、公開オリジンを`MAJIN_ALLOWED_ORIGINS`に追加してください。

:::

## Docker

公開済みのイメージはありません。バイナリは十分に静的なので、イメージを用意するメリットはあまりありません。必要なら次のようにできます。

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

コンテナ内では`MAJIN_HOST=0.0.0.0`で問題ありません。到達範囲を制限するのはコンテナ境界です。公開ポートは、到達させたい場所にだけ公開してください。

## 動作確認

    curl -s https://majin.example.com/api/info

    {"version":"0.2.0","protocolVersion":1,"authRequired":true}

ここで`authRequired: false`なら、サーバーが`--insecure-no-auth`で動作しており、到達できる人は誰でもシェルを使える状態です。

## 問題が起きたとき

| 症状 | 考えられる原因 |
|----|----|
| ログインできるが、端末が「再接続中…」のまま | プロキシがWebSocketのアップグレードを転送していません。 |
| すぐにログイン画面へ戻される | オリジンチェックの失敗（`Host`の書き換え）、またはブラウザーが通常のHTTPと認識する接続で`Secure` Cookieが設定されています。 |
| `no frontend is embedded in this binary` | `pnpm build:web`ではなく、`cargo`を直接使ってビルドしています。 |
| 数分アイドルにすると接続が切れる | プロキシの読み取りタイムアウトが短すぎます。 |

`MAJIN_LOG=majin_server=debug`でログレベルを上げられます。
