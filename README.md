# pwstore

Tauri v2 製のパーソナルパスワードマネージャ。Google Drive でデータを同期する。

## ビルドに必要なもの

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) / npm
- [Tauri CLI](https://tauri.app/start/prerequisites/)

## 初回セットアップ

### 1. OAuth 認証情報の準備

Google Cloud Console でプロジェクトを作成し、OAuth 2.0 クライアント ID（デスクトップアプリ用）を発行する。
スコープは `https://www.googleapis.com/auth/drive.file` のみ。

`src-tauri/.env.example` を `src-tauri/.env` にコピーして値を記入する：

```
GOOGLE_CLIENT_ID=XXXXXXXXXX.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=GOCSPX-XXXXXXXXXXXXXXXXXXXXXXXX
```

`.env` はビルド時に読み込まれ、バイナリに静的に埋め込まれる（実行時ファイルは不要）。
`.env` は `.gitignore` に含まれているため別途 private リポジトリで管理する。

### 2. ビルド・起動

```bash
npm install
npm run tauri dev   # 開発
npm run tauri build # リリースビルド
```

### 3. アプリの初期設定

初回起動時にマスターパスフレーズを設定し、Google アカウントで OAuth 認証を行う。
認証が完了するとアプリが使えるようになる。

## ランタイムのファイル構成

すべてのデータは Tauri の `app_data_dir` 以下に保存される。

| プラットフォーム | パス |
|---|---|
| macOS | `~/Library/Application Support/com.ketanaka.pwstore-tauri/` |
| Linux | `~/.local/share/com.ketanaka.pwstore-tauri/` |
| Windows | `%APPDATA%\com.ketanaka.pwstore-tauri\` |
| Android | アプリ内部ストレージ |

### ファイル一覧

| ファイル名 | 内容 | 暗号化 |
|---|---|---|
| `data.enc` | パスワードストア（JSON を AES-256-GCM で暗号化） | ✓ マスターパスフレーズで |
| `passphrase` | マスターパスフレーズ | ✓ 固定キーで難読化（`0600` 権限） |
| `refresh_token` | Google OAuth リフレッシュトークン | ✓ 固定キーで難読化（`0600` 権限） |
| `sync_hash` | 最終同期時の `data.enc` ハッシュ（競合検出用） | なし |

> `passphrase` と `refresh_token` の固定キー暗号化はプレーンテキスト検索への引っかかりを防ぐ目的。
> セキュリティの本質はマスターパスフレーズによる `data.enc` の暗号化にある。

`refresh_token` が存在する場合に初期設定済みとみなす。

## 暗号化仕様

- アルゴリズム: AES-256-GCM
- 鍵導出: scrypt (N=16384, r=8, p=1)
- フォーマット: `[salt(16B)][nonce(12B)][ciphertext+tag]`

## Google Drive 同期

Drive の `My Drive/pwstore/` フォルダ内の `data.enc` と同期する。
スコープは `drive.file`（このアプリが作成したファイルのみアクセス可能）。

### 競合検出ロジック

| ローカル変更 | Drive 変更 | 動作 |
|---|---|---|
| なし | なし | 何もしない |
| あり | なし | アップロード |
| なし | あり | ダウンロード |
| あり | あり | 競合ダイアログを表示 |

競合時は「ローカルを使用」「Drive を使用」「何もしない」の 3 択。

### 同期タイミング

| タイミング | コマンド |
|---|---|
| 起動時・管理画面遷移時 | `drive_download`（競合チェックあり） |
| 保存・カテゴリ変更後 | `drive_sync`（競合検出あり） |
| ⇄ ボタン | `drive_sync` |

## ツール

### `tools/decrypt.js` — 緊急復号

```bash
node tools/decrypt.js path/to/data.enc "マスターパスフレーズ"
```

`data.enc` を JSON に復号して標準出力に出力する。

### `tools/encrypt.js` — 移行用暗号化

```bash
node tools/encrypt.js path/to/entries.json "マスターパスフレーズ"
```

JSON 形式のエントリデータを `data.enc` に暗号化する。他のパスワードマネージャからの移行に使用する。

## 開発メモ

### Linux VM でのコンパイルエラー (SIGSEGV)

Linux 仮想マシン環境でコンパイル時に SIGSEGV が発生することがある。以下のオプションで回避できる：

```bash
CARGO_BUILD_JOBS=1 RUSTFLAGS="-C codegen-units=1" cargo test --lib
```
