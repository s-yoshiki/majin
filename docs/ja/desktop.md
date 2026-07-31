# デスクトップ版

Web版と同じ端末UIをネイティブウィンドウで表示し、Tauri IPCでローカルPTYに接続します。サーバーもポートもトークンも必要ありません。

## インストール

[Releases](https://github.com/s-yoshiki/majin/releases)からダウンロードします。

| プラットフォーム | ファイル |
|---------------------|----------------------------|
| macOS Apple silicon | `majin_<version>_aarch64.dmg` |
| macOS Intel | `majin_<version>_x64.dmg` |
| Windows | `majin_<version>_x64-setup.exe` または `.msi` |
| Linux | `majin_<version>_amd64.AppImage` または `.deb` |

::: warning 署名されていないビルド

リリースにはコード署名も公証もありません。macOSでは初回起動が拒否されます。アプリを右クリックして「開く」を選ぶか、`xattr -dr com.apple.quarantine /Applications/majin.app`で隔離属性を解除してください。WindowsのSmartScreenでは「詳細情報」を選んでから「実行」を押してください。

:::

## Web版との違い

|  | Web版 | デスクトップ版 |
|----|----|----|
| transport | HTTP上のWebSocket | Tauri IPC |
| 認証 | トークン、その後セッションCookie | なし。PTYへ外部から到達できません |
| ネットワーク公開 | 待ち受けポートあり | なし |
| PTY実装 | `majin-pty`。両方で同一 |  |
| 端末UI | `@majin/terminal-ui`。両方で同一 |  |

## 自分でビルドする

Node、pnpm、Rustに加えて、プラットフォームのWebViewツールチェーンが必要です。[Tauriの前提条件](https://tauri.app/start/prerequisites/)を参照してください。

    pnpm install
    pnpm build:desktop

インストーラーは`target/release/bundle/`に生成されます。開発時はフロントエンドをホットリロードできます。

    pnpm dev:desktop

::: info macOSでDMGの作成に失敗する場合

TauriはAppleScriptでFinderを操作してディスクイメージのウィンドウを整えます。そのためSSH経由、CIコンテナ内、Automation権限が与えられていない環境では失敗し、通常はAppleEventのタイムアウト（`-1712`）になります。`target/release/bundle/macos/`の`.app`は完全で実行可能です。足りないのは`.dmg`のラッパーだけです。

:::

## Rust側の接続方法

`apps/desktop/src-tauri/src/lib.rs`には4つのコマンドと2つのイベントがあります。

| コマンド | 内容 |
|----|----|
| `pty_open(cols, rows)` | シェルを起動して`{ shell, cols, rows }`を返します。再度呼ぶと前のセッションを置き換え、リークを防ぎます。 |
| `pty_write(data)` | キー入力を転送します。 |
| `pty_resize(cols, rows)` | PTYをリサイズします。`pty_open`より先にリサイズが到着する競合を避けるため、セッションがない場合は無視します。 |
| `pty_close()` | シェルを終了します。 |

出力は`pty://output`イベント、終了は`pty://exit`イベントで届きます。`TauriTransport`はこれらをWeb版が受け取るものと同じ`ServerMessage`フレームに変換します。そのため`TerminalView`は両者を区別する必要がありません。

## 起動するシェル

macOSとLinuxでは`$SHELL`、Windowsでは`%COMSPEC%`を、ホームディレクトリで`TERM=xterm-256color`を設定して起動します。

::: info 重いシェル設定について

対話的な入力を要求するものを含め、完全な起動ファイルが実行されます。起動時に質問するプラグインマネージャーがあると、最初に入力したキーを取り込むことがあります。それは端末ではなく、シェル設定の動作です。

:::
