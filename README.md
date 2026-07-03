# pwstore

## ビルドに必要なもの

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) / npm
- [Tauri CLI](https://tauri.app/start/prerequisites/)

## 初回セットアップ

### SETUP.1 OAuth 認証情報の準備

Google Cloud Console でプロジェクトを作成し、OAuth 2.0 クライアント ID（デスクトップアプリ用）を発行する。
スコープは `https://www.googleapis.com/auth/drive`。

発行した値は `src-tauri/.env` に記入する：


### SETUP.2 ビルド・起動

```bash
npm install
npm run tauri dev   # 開発
npm run tauri build # リリースビルド
```

## Mac 向け Production ビルド

### BUILD.1 ツールのインストール

```bash
# Xcode Command Line Tools
xcode-select --install

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Node.js (v18以上)
## コマンドは割愛
```

### BUILD.2 リポジトリのセットアップ

```bash
npm install
```

### BUILD.3 `.env` を配置

```bash
cp src-tauri/.env.example src-tauri/.env
# エディタで GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET / SECRET_FILE_KEY を記入
```

### BUILD.4 ビルド

```bash
npm run tauri build
```

### BUILD.5 成果物

```
src-tauri/target/release/bundle/macos/pwstore-tauri.app
src-tauri/target/release/bundle/dmg/pwstore-tauri_0.1.0_aarch64.dmg  # Apple Silicon
src-tauri/target/release/bundle/dmg/pwstore-tauri_0.1.0_x64.dmg      # Intel
```

dmg を実行してインストールする

### BUILD.6 初回起動時の Gatekeeper 回避

署名なしのため「開発元を確認できません」と表示される。
Finder で右クリック →「開く」を選択するか、以下のコマンドを実行して回避する

```bash
# quarantine 属性を外す
xattr -d com.apple.quarantine /Applications/pwstore-tauri.app
```
