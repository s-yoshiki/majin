# majin - 魔人🧞

[English](README.md) | [日本語](README.ja.md)

[![CI](https://github.com/s-yoshiki/majin/actions/workflows/ci.yml/badge.svg)](https://github.com/s-yoshiki/majin/actions/workflows/ci.yml)
[![GitHub release](https://img.shields.io/github/v/release/s-yoshiki/majin)](https://github.com/s-yoshiki/majin/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[xterm.js](https://xtermjs.org/) で描画する、疑似端末（PTY）上の本物のログインシェルです。
単一バイナリのWebサーバー、またはネイティブな
[Tauri](https://tauri.app/) デスクトップアプリとして利用できます。

**[ドキュメント](https://s-yoshiki.github.io/majin/)** ·
**[リリース](https://github.com/s-yoshiki/majin/releases)**

> [!WARNING]
> majinは、実行したアカウントと同じ権限を持つ対話型シェルを提供します。
> サーバーはデフォルトで `127.0.0.1` にバインドされます。別のマシンから
> 到達できるようにする前に、必ず
> [セキュリティに関する注意事項](https://s-yoshiki.github.io/majin/security.html)
> を確認してください。

## majinの特徴

- **コマンド実行APIではなく、本物のPTY。** 対話型プログラム、ANSIカラー、
  Unicode、ウィンドウのリサイズ、シグナルがローカル端末と同じように動作します。
- **2つの提供形態。** Web版はReactアプリを1つのRustバイナリに埋め込み、
  デスクトップ版は同じPTY実装へローカルIPCで接続します。
- **小さな運用負荷。** Webリリースの実行にNode.jsプロセス、パッケージの
  インストール、静的ファイルの個別デプロイは不要です。
- **共有された型安全なプロトコル。** Rustをすべてのフレーム定義の正とし、
  TypeScriptの型を生成することで、サーバーとクライアントの不整合を防ぎます。
- **安全側のデフォルト。** ループバックへのバインド、ランダムなアクセストークン、
  `HttpOnly` セッション、Origin検証、サインインのレート制限を備えています。

## ビルドの選び方

| | Webサーバー | デスクトップアプリ |
| --- | --- | --- |
| 用途 | ブラウザからシェルへアクセス | ネイティブウィンドウでローカル端末を使用 |
| 通信方式 | HTTP + WebSocket | Tauri IPC |
| ネットワーク待受 | デフォルトで `127.0.0.1:8999` | なし |
| 認証 | アクセストークンをセッションCookieへ交換 | 不要 |
| 配布形式 | Web UIを埋め込んだ単一バイナリ | `.dmg`、`.msi`、`.AppImage` など |

## クイックスタート

### Webサーバー

[Releases](https://github.com/s-yoshiki/majin/releases) から環境に合う
サーバーアーカイブをダウンロードして展開し、次を実行します。

```bash
./majin
```

Windowsでは `majin.exe` を実行します。サーバーは1回限りのサインインURLと
アクセストークンを表示します。

```text
majin 0.2.0 — terminal over the web

Open:  http://127.0.0.1:8999/#token=1f3c…
Token: 1f3c…
```

表示されたURLをブラウザで開きます。トークンはURLフラグメントで渡され、
`HttpOnly` セッションCookieへ交換された後、アドレスバーから削除されます。

よく使う設定例:

```bash
# シェルと作業ディレクトリを指定
majin --shell /bin/bash --cwd /srv/project

# 固定トークンを使用（常時稼働させる場合に推奨）
MAJIN_TOKEN="$(openssl rand -hex 24)" majin

# 全フラグと対応する環境変数を表示
majin --help
```

### デスクトップアプリ

[Releases](https://github.com/s-yoshiki/majin/releases) から環境に合う
パッケージをインストールします。デスクトップアプリはローカルのログインシェルを
IPC経由で直接開くため、サーバーの起動やポートの待受は行いません。

現在のリリースビルドは未署名です。macOSとWindowsでは初回起動時に確認を
求められる場合があります。

## ソースからビルド

必要なツール:

| ツール | バージョン |
| --- | --- |
| Node.js | 20以上 |
| pnpm | 11以上 |
| Rust | 1.86以上 |

```bash
git clone https://github.com/s-yoshiki/majin.git
cd majin
pnpm install
pnpm dev
```

<http://localhost:5173> を開きます。この開発用コマンドはRustサーバーを
ポート `8999`、Viteをポート `5173` で起動します。サーバーが表示した
トークンをViteアプリへ貼り付けてください。

デスクトップ版を開発する場合は
[Tauriのプラットフォーム別要件](https://tauri.app/start/prerequisites/)
をインストールして、次を実行します。

```bash
pnpm dev:desktop
```

プラットフォーム別パッケージ、プロトコル型の生成、開発フロー全体については
[開発ガイド](https://s-yoshiki.github.io/majin/development.html)を参照してください。

## 設定

サーバーの各オプションは、コマンドラインフラグと環境変数の両方で指定できます。
両方を指定した場合はフラグが優先されます。

| フラグ | 環境変数 | デフォルト | 用途 |
| --- | --- | --- | --- |
| `--host` | `MAJIN_HOST` | `127.0.0.1` | バインドアドレス |
| `--port`, `-p` | `MAJIN_PORT` | `8999` | 待受ポート |
| `--token` | `MAJIN_TOKEN` | 自動生成 | アクセストークン |
| `--shell` | `MAJIN_SHELL` | `$SHELL` / `%COMSPEC%` | 起動するシェル |
| `--cwd` | `MAJIN_CWD` | ユーザーのホーム | 初期作業ディレクトリ |
| `--max-sessions` | `MAJIN_MAX_SESSIONS` | `8` | 同時接続できる端末数 |
| `--session-ttl-minutes` | `MAJIN_SESSION_TTL_MINUTES` | `720` | セッションの有効期間 |

[設定リファレンス](https://s-yoshiki.github.io/majin/configuration.html)では、
許可Origin、Secure Cookie、ログ、認証関連の設定も確認できます。

## セキュリティモデル

Webサーバーは、認証済みクライアントにOSプロセスと同じ権限のシェルを提供します。
TLS、ユーザー分離、サンドボックス、ユーザーごとのアカウント機能は提供しません。

ローカル以外で使用する場合:

- リバースプロキシでTLSを終端する
- majinを専用の非特権OSユーザーで実行する
- 固定されたランダムな `MAJIN_TOKEN` を設定する
- プロキシが同じホストにある場合、サーバーはループバックで待ち受ける
- Cookieと許可Originの設定を確認する

信頼できないネットワークへ `--insecure-no-auth` を公開しないでください。
デプロイ前に
[Security](https://s-yoshiki.github.io/majin/security.html) と
[Deploying the web build](https://s-yoshiki.github.io/majin/deployment.html)
を確認してください。脆弱性は
[非公開のSecurity Advisory](https://github.com/s-yoshiki/majin/security/advisories/new)
から報告してください。

## アーキテクチャ

```text
ブラウザ ── WebSocket ── majin-server ─┐
                                       ├── majin-pty ── ログインシェル
デスクトップ ── Tauri IPC ────────────┘
           │
           └── 共通terminal-ui（React + xterm.js）
```

```text
crates/
├── majin-protocol/    Rustの通信型とTypeScript型の生成
├── majin-pty/         PTYのライフサイクル、I/O、リサイズ処理
└── majin-server/      axum HTTP/WebSocketサーバーと埋め込みフロントエンド
apps/
├── web/               ブラウザ向けフロントエンド
└── desktop/           Tauri v2デスクトップアプリ
packages/
├── protocol/          生成された型とトランスポート実装
└── terminal-ui/       共通のxterm.js Reactコンポーネント
configs/               TypeScriptとBiomeの共通設定
docs/                  VitePressドキュメントサイト
```

詳細は
[アーキテクチャガイド](https://s-yoshiki.github.io/majin/architecture.html) と
[通信プロトコルのリファレンス](https://s-yoshiki.github.io/majin/protocol.html)
を参照してください。

## 開発コマンド

| コマンド | 内容 |
| --- | --- |
| `pnpm dev` | RustサーバーとVite Webアプリを起動 |
| `pnpm dev:desktop` | フロントエンドのホットリロード付きでTauriアプリを起動 |
| `pnpm build:web` | フロントエンドと、それを埋め込むサーバーバイナリをビルド |
| `pnpm build:desktop` | `target/release/bundle/` に各プラットフォーム用インストーラーを生成 |
| `pnpm lint` | BiomeとClippyを警告エラー設定で実行 |
| `pnpm typecheck` | 全TypeScriptパッケージとRustクレートを型チェック |
| `pnpm test` | 実PTYを使うテストを含むRustワークスペーステストを実行 |
| `pnpm test:e2e` | 認証、WebSocket通信、シェルをE2Eテスト |
| `pnpm generate:protocol` | RustからTypeScriptのプロトコル型を再生成 |

Pull Requestを作成する前に、`pnpm lint`、`pnpm typecheck`、`pnpm test` を
実行してください。`crates/majin-protocol` を変更した場合は、TypeScriptの型も
再生成してコミットします。

## ドキュメント

| ページ | 内容 |
| --- | --- |
| [Overview](https://s-yoshiki.github.io/majin/) | majinの概要と開始方法 |
| [Development](https://s-yoshiki.github.io/majin/development.html) | モノレポのセットアップと開発フロー |
| [Deployment](https://s-yoshiki.github.io/majin/deployment.html) | systemd、リバースプロキシ、Docker |
| [Desktop](https://s-yoshiki.github.io/majin/desktop.html) | デスクトップアプリのインストールとビルド |
| [Architecture](https://s-yoshiki.github.io/majin/architecture.html) | コンポーネントとデータフロー |
| [Wire protocol](https://s-yoshiki.github.io/majin/protocol.html) | フレーム、エンドポイント、終了コード |
| [Security](https://s-yoshiki.github.io/majin/security.html) | 認証と信頼境界 |
| [Configuration](https://s-yoshiki.github.io/majin/configuration.html) | フラグと環境変数 |

## ライセンス

[MIT](LICENSE)
