# 開発

同じリポジトリ内にpnpmワークスペースとCargoワークスペースがあり、Turborepoが両方を実行します。

## 前提条件

| ツール | バージョン | 用途 |
|---------|---------|------|
| Node.js | 20以上 | フロントエンドのツールチェーン |
| pnpm | 11以上 | ワークスペース管理 |
| Rust | 1.86以上 | バックエンド全体 |

デスクトップアプリのビルドには、プラットフォームのWebViewツールチェーンも必要です。macOSではXcode Command Line Tools、WindowsではWebView2ランタイム、Linuxでは`libwebkit2gtk-4.1-dev`と`libgtk-3-dev`を用意してください。[Tauriの前提条件](https://tauri.app/start/prerequisites/)も参照してください。

## セットアップ

    git clone https://github.com/s-yoshiki/majin.git
    cd majin
    pnpm install

## Web版の開発ループ

    pnpm dev

2つのプロセスが起動します。Rustサーバーは`:8999`、Vite開発サーバーは`:5173`で動作します。`http://localhost:5173`にアクセスしてください。Viteは`/api`と`/ws`をバックエンドへプロキシし、サーバーはオリジンチェックで`http://localhost:5173`を許可するよう`--allowed-origin`付きで起動します。

サーバーは起動時にアクセストークンと、直接サインインできるリンクを表示します。ただし、表示されるリンクは`:8999`を指します。Viteサーバーを使う場合は、トークンを`:5173`のログインフォームへ入力してください。

### ビルド済みフロントエンドを使う

開発サーバーではなく、実際の埋め込みアセットの経路を確認するには次のようにします。

    pnpm build:web
    ./target/release/majin

デバッグビルドでは、`rust-embed`は実行時に`apps/web/dist`を読み取るため、`cargo run`でRustを再コンパイルせずにフロントエンドの更新を反映できます。リリースビルドではファイルがバイナリに埋め込まれます。

## デスクトップ版の開発ループ

    pnpm dev:desktop

Tauriは`:5174`でViteを起動し、そこを開くウィンドウを表示します。フロントエンドはホットリロードに対応しています。Rustを変更すると再ビルドと再起動が行われます。

## プロトコルを変更する

`crates/majin-protocol/src/lib.rs`の型を編集してから、次を実行します。

    pnpm generate:protocol

`packages/protocol/src/generated/protocol.ts`が書き換えられ、フォーマットされます。コミットしてください。CIは再生成した結果と差分があると失敗します。`packages/protocol/src/codec.ts`の実行時ガードは手書きなので、同じ変更で更新する必要があります。

## チェック

| コマンド | 内容 |
|----|----|
| `pnpm lint` | TypeScriptではBiome、RustではClippyを実行。警告はエラー扱い。 |
| `pnpm lint:fix` | 両方で自動修正できるものを適用。 |
| `pnpm format` | Biomeフォーマッターと`cargo fmt`を実行。 |
| `pnpm typecheck` | パッケージごとに`tsc --noEmit`、クレートごとに`cargo check`を実行。 |
| `pnpm test` | `cargo test --workspace`。実PTYを起動し、シェルが受け取る内容を検証するテストを含みます。 |
| `pnpm build` | 依存関係順にすべてビルド。 |

## ファイルの場所

| 変更したいもの | 編集するファイル |
|----|----|
| 両方のアプリでの端末の見た目や動作 | `packages/terminal-ui/src/TerminalView.tsx` |
| カラースキーム | `packages/terminal-ui/src/theme.ts` |
| シェルの起動、リサイズ、出力デコード | `crates/majin-pty/src/lib.rs` |
| 認証 | `crates/majin-server/src/auth.rs` |
| サーバールート | `crates/majin-server/src/main.rs` |
| デスクトップIPCコマンド | `apps/desktop/src-tauri/src/lib.rs` |
| アプリアイコン | `apps/desktop/scripts/generate-icon.mjs`、その後`pnpm --filter @majin/desktop icon` |

## 共有設定

TypeScriptとBiomeの設定は、アプリごとにコピーせず、`configs/`のワークスペースパッケージで共有しています。

    configs/tsconfig/base.json        strictのデフォルト、DOMなし
    configs/tsconfig/node.json        + Nodeの型
    configs/tsconfig/react-lib.json   + DOMとJSX
    configs/tsconfig/react-app.json   + Viteクライアントの型
    configs/biome/base.json           フォーマッターとLintのルール

パッケージからは次のように継承して利用します。

    { "extends": "@majin/tsconfig/react-app.json", "include": ["src"] }

::: info Biomeについて知っておくこと

共有設定のファイル名は意図的に`biome.json`ではなく`base.json`になっています。Biomeは`biome.json`という名前のファイルを自動検出し、共有設定を別のルート設定として扱ってしまうためです。

:::
