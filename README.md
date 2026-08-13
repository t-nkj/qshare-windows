# QShare for Windows

コンソールを表示せず、クリップボード・URL・ファイルをQShareと送受信するWindows用クライアントです。

## セットアップ

リリースの`QShare.exe`と同じフォルダーに`.env`を配置し、端末トークンを設定してください。

`.env`が存在しない状態で起動すると、EXEに埋め込んだテンプレートから`.env`を自動作成して終了します。表示される通知（または「編集する」ボタン）をクリックすると、既定のエディターで`.env`を開けます。

```dotenv
QSHARE_TOKEN=qsh_replace_me
API_BASE_URL=https://qshare.trap.show/api/
# QSHARE_DOWNLOAD_DIR=C:\\path\\to\\custom-folder
```

`API_BASE_URL`は`/api/`までを指定します。クライアントがAPIバージョンの`v1/`を追加します。`QSHARE_DOWNLOAD_DIR`を省略すると、`QShare.exe`と同じフォルダーにある`files/`へ保存します。`files/`は最初の受信時に自動作成されます。

## 使い方

```powershell
# 最新のメモ・URL・ファイルを受信
.\QShare.exe --receiver

# クリップボードのテキストをメモとして送信
.\QShare.exe --sender

# 1件以上のファイルをまとめて送信
.\QShare.exe --sender C:\path\to\one.pdf C:\path\to\two.png
```

エクスプローラーからファイルを`QShare.exe`へドラッグ＆ドロップした場合も、Windowsが渡す複数ファイルパスをまとめて送信できます。ファイル送受信中はWindows通知に進捗率を表示します。正常終了・エラー・不正な引数も通知で表示します。

ファイルは1件100 MiB、送信合計1 GiBまでです。受信先に同名ファイルがある場合は、既存ファイルを上書きせず`file (1).ext`のように連番を付けて保存します。

## 開発・リリース

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

pushごとにGitHub ActionsがWindows releaseビルドを行い、`QShare.exe`をartifactとして保存します。`Cargo.toml` のパッケージバージョンを直前コミットより上げたpushでは、`v<package-version>`タグのGitHub Releaseを作成してEXEを添付します。それ以外のpushでは、`v<package-version>-build.<run-number>`形式のGitタグだけを作成します。
