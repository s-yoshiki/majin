# 設定

すべてのオプションはフラグと環境変数の両方で指定できます。フラグが優先されます。

`majin --help`を実行すると、バイナリから同じ一覧を確認できます。

## ネットワーク

| フラグ | 環境変数 | デフォルト | 説明 |
|----|----|----|----|
| `--host` | `MAJIN_HOST` | `127.0.0.1` | 意図的にループバックがデフォルトです。広げる場合は慎重に判断してください。[セキュリティ](./security.md)を参照。 |
| `--port`, `-p` | `MAJIN_PORT` | `8999` | `0`にするとOSに割り当てを任せます。バナーに割り当てられたポートが表示されます。 |
| `--allowed-origin` | `MAJIN_ALLOWED_ORIGINS` | — | WebSocketの接続を許可する追加オリジン。同一オリジンは常に許可されるため、Vite開発サーバーなど別の場所からUIを配信する場合だけ使います。フラグは繰り返し指定でき、変数はカンマ区切りにできます。 |

## 認証

| フラグ | 環境変数 | デフォルト | 説明 |
|----|----|----|----|
| `--token` | `MAJIN_TOKEN` | 生成 | 未指定なら192ビットの新しいトークンを生成して表示します。再起動ごとに変わるため、長期間運用する場合は明示的に設定してください。 |
| `--session-ttl-minutes` | `MAJIN_SESSION_TTL_MINUTES` | `720` | セッションの有効時間。デフォルトは12時間です。 |
| `--secure-cookie` | `MAJIN_SECURE_COOKIE` | `false` | セッションCookieに`Secure`を強制します。`X-Forwarded-Proto: https`から自動検出します。プロキシがこのヘッダーを付けない場合に設定してください。 |
| `--insecure-no-auth` | `MAJIN_INSECURE_NO_AUTH` | `false` | 認証を完全に無効にします。前段で別の認証を行っている場合だけ許容できます。 |

::: danger --insecure-no-auth

これを有効にすると、ポートへ接続できる人は誰でも、サーバーを実行しているユーザーとしてシェルを操作できます。起動バナーにもそのことが明示されます。

:::

## セッションとシェル

| フラグ | 環境変数 | デフォルト | 説明 |
|----|----|----|----|
| `--shell` | `MAJIN_SHELL` | `$SHELL` | Windowsでは`%COMSPEC%`、最後のフォールバックは`/bin/bash`です。 |
| `--cwd` | `MAJIN_CWD` | ホームディレクトリ | 新しいセッションの作業ディレクトリ。 |
| `--max-sessions` | `MAJIN_MAX_SESSIONS` | `8` | 同時に開ける端末数。超過した接続は`4002`で閉じられます。 |

起動したシェルは、サーバーの環境に加えて`TERM=xterm-256color`と`COLORTERM=truecolor`を継承します。これらがないとプログラムは貧弱な端末だと判断し、色やカーソル制御が機能しなくなります。

## ログ

| 環境変数 | デフォルト | 説明 |
|----|----|----|
| `MAJIN_LOG` | `majin_server=info,tower_http=warn` | [EnvFilter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)の構文。端末の内容は記録せず、接続とライフサイクルイベントだけを記録します。 |

## 例

ローカルで使う場合。設定は不要です。

    majin

TLS終端プロキシの背後で固定トークンを使う場合。

    MAJIN_TOKEN="$(openssl rand -hex 24)" \
    MAJIN_SECURE_COOKIE=1 \
    majin --host 127.0.0.1 --port 8999

Vite開発サーバーをバックエンドにする場合。

    majin --allowed-origin http://localhost:5173

コンテナ内で制限付きセッションを1つだけ許可する場合。

    majin --host 0.0.0.0 --shell /bin/bash --max-sessions 1 \
                 --session-ttl-minutes 60
