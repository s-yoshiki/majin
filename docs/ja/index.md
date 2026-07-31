# majin

xterm.jsで擬似端末上のログインシェルを表示します。UIを内蔵した自己完結型のWebサーバーバイナリと、Tauriデスクトップアプリの2通りで利用できます。

<div class="cards">

<a href="./deployment.html" class="card"><strong>サーバーで実行</strong> <span>UIを内蔵した単一バイナリ。Nodeもnpm installも必要ありません。</span></a> <a href="./desktop.html" class="card"><strong>デスクトップで実行</strong> <span>TauriアプリからIPCでローカルPTYに接続。ポートを開きません。</span></a>

</div>

## これは何か

majinはPTY上でログインシェルを起動し、ブラウザーへストリーミングします。端末エミュレーターに必要な処理をエンドツーエンドで扱い、ウィンドウのリサイズを`SIGWINCH`としてシェルに伝え、読み取り境界をまたぐマルチバイト出力を再構成し、接続が切れると子プロセスを終了します。

PTY層は、どちらのビルドでも共有する1つのRustクレート`majin-pty`です。Webサーバーはトークン認証付きのHTTPとWebSocketを提供します。デスクトップアプリはネットワークを使わず、同じクレートをTauri IPC経由で呼び出します。

## クイックスタート

### Web版

[Releases](https://github.com/s-yoshiki/majin/releases)からお使いのプラットフォーム用バイナリをダウンロードして実行します。

    ./majin

起動すると、新しく生成されたアクセストークンを含むサインイン用リンクが表示されます。リンクを開けば端末を利用できます。

      majin 0.2.0 — terminal over the web

      Open:  http://127.0.0.1:8999/#token=1f3c…

      Token: 1f3c…

デフォルトでは`127.0.0.1`にバインドします。より広い範囲に公開する前に[セキュリティ](./security.md)を確認してください。

### デスクトップ版

[Releases](https://github.com/s-yoshiki/majin/releases)から`.dmg`、`.msi`または`.AppImage`をダウンロードしてインストールし、起動します。設定は不要で、ポートを待ち受けることもありません。

### ソースから実行

    git clone https://github.com/s-yoshiki/majin.git
    cd majin
    pnpm install
    pnpm dev

詳しい手順は[開発](./development.md)を参照してください。

## なぜ書き直したのか

以前のバージョンはExpressサーバー、素のxterm.jsページ、2つのシェルスクリプトで構成されていました。動作はしていましたが、プロトコルの両端を別々に実装していたため、ブラウザーは`{ resizer: [...] }`を送る一方、サーバーは`msg.resize`を読んでいました。その結果、リサイズは何も起こさず、コードが修正されるまで検出もされませんでした。

現在はプロトコルをRustで一度だけ定義し、TypeScriptの型をそこから生成します。一方の名前を変更すると、もう一方でコンパイルエラーになります。その他の再構築もこの方針に沿っています。pnpmとTurborepoによるモノレポ、共有PTYクレート、フォーマットとLintに使うBiome、両方のターゲット向けリリース成果物をCIでビルドする構成です。

## 必要なもの

| 目的 | 必要なもの |
|----|----|
| リリースバイナリを実行 | なし。フロントエンド内蔵で、実行時の依存関係はありません。 |
| ソースからビルド | Node.js 20以上、pnpm 11以上、Rust 1.86以上 |
| デスクトップアプリをビルド | 上記に加えて、プラットフォームのWebViewツールチェーン（macOSはXcode CLT、WindowsはWebView2、Linuxは`libwebkit2gtk-4.1-dev`） |
