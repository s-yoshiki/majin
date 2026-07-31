# アーキテクチャ

PTYの実装とプロトコル定義はそれぞれ1つです。2つのフロントエンドがReactコンポーネントを共有し、シェルへバイト列を届ける方法だけが異なります。

## 全体像

![ブラウザーとデスクトップのフロントエンドは共有のmajin-ptyクレートに合流します。](/architecture.svg)

*どちらの経路も同じクレートに合流します。デスクトップ版にはウィンドウとPTYの間にサーバーもポートも認証もありません。*

## リポジトリ構成

    majin/
    ├── crates/
    │   ├── majin-protocol/    すべてのフレームのRust型。TSのミラーを生成
    │   ├── majin-pty/         PTYの起動、読み書き、リサイズ。両方のビルドで共有
    │   └── majin-server/      axum HTTP + WebSocket、認証、フロントエンド埋め込み
    ├── apps/
    │   ├── web/               サーバーバイナリに組み込まれるReactアプリ
    │   └── desktop/           Tauri v2アプリ
    │       └── src-tauri/     Rust側。majin-pty経由のIPCコマンド
    ├── packages/
    │   ├── protocol/          生成型 + transportインターフェース + WebSocket transport
    │   └── terminal-ui/       両アプリが表示するxterm.jsのReactコンポーネント
    ├── configs/
    │   ├── tsconfig/          共有TypeScript設定
    │   └── biome/              共有Biome設定
    └── docs/                  このサイト

## transportの境界

`packages/terminal-ui`の`TerminalView`が端末UI全体です。WebSocketやTauriについては何も知りません。受け取るのは`TerminalTransport`です。

    interface TerminalTransport {
      readonly state: TransportState;
      connect(): void;
      send(message: ClientMessage): void;
      onMessage(listener: (message: ServerMessage) => void): Unsubscribe;
      onStateChange(listener: (state: TransportState) => void): Unsubscribe;
      dispose(): void;
    }

Webアプリは`WebSocketTransport`を、デスクトップアプリは`TauriTransport`を注入します。この単一のインターフェースがあるため、互いにずれていく2つの端末コンポーネントではなく、1つの端末コンポーネントで済みます。

## 型は手で複製せず生成する

`crates/majin-protocol`が正となる定義です。`pnpm generate:protocol`を実行すると、小さなRustバイナリが[ts-rs](https://docs.rs/ts-rs)経由で`packages/protocol/src/generated/protocol.ts`を出力します。CIでも再生成し差分があれば失敗するため、プロトコルの2つの表現がずれることはありません。

::: info なぜこの仕組みがあるのか

以前のバージョンでは両側を手で管理していたため、クライアントが`resizer`フィールドを送り、サーバーが`resize`を読むという間違いが起きました。その間、ウィンドウのリサイズは静かに機能していませんでした。

:::

## PTYからの読み取り

`portable-pty`が提供するのはブロッキングリーダーだけです。そのため`majin-pty`は専用スレッドで読み取り、デコードしたチャンクを`tokio`チャネルへ転送します。重要なのは次の2点です。

- **チャンク境界をまたぐUTF-8。** 読み取り位置は任意のバイト境界になるため、マルチバイト文字が2つのチャンクに分かれることがあります。各チャンクを個別にデコードすると、非ASCII出力が壊れます。特にCJKテキストや罫線文字で目立ちます。`Utf8Stream`は残りのバイトが届くまで不完全な末尾を保持し、本当に不正なバイトには停止する代わりにU+FFFDを代入します。
- **ライフタイム。** `PtySession`を破棄すると子プロセスも終了します。そのため、閉じたタブやウィンドウが孤立したシェルを残すことはありません。

## ビルドのオーケストレーション

Turborepoが2つの言語を駆動します。各Rustクレートには`cargo`を呼び出す薄い`package.json`があり、タスクグラフで重要な依存関係を表現できます。それは`rust-embed`がコンパイル時に`apps/web/dist`をバイナリへ埋め込むため、`crates/majin-server`より先に`apps/web`をビルドする必要があることです。

この依存関係はturboが理解できる形で宣言されています。`majin-server`の`package.json`が`@majin/web`をdevDependencyに登録しています。
