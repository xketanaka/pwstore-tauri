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

## Production ビルド(for Mac)

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

## Production ビルド(for Android by Docker)

Dockerコンテナを使ってAndroid版のビルドを行う手順

### Android.1 イメージのビルド

```bash
docker build -f Dockerfile.android -t pwstore-android-builder .
```

### Android.2 ビルド

リポジトリ外の dot.env ファイル、keystore ファイルを指定する

```
docker run --rm \
    -v "$(pwd):/workspace" \
    -v "../pwstore-tauri-private/pwstore-release.keystore:/keystore/pwstore-release.keystore:ro" \
    --env-file ../pwstore-tauri-private/dot.env \
    pwstore-android-builder \
    bash -c "npm install && npm run tauri android build"
```

### Android.3 成果物

```
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
```

### Android.4 端末へのインストール

Android 端末の「開発者向けオプション」で USB デバッグを有効にして接続し：

```bash
adb install path/to/app-universal-release-unsigned.apk
```


## テスト

### TypeScript (vitest)

```bash
npm test           # 全テストを1回実行
npm run test:watch # ファイル変更を監視して自動再実行
```

テストファイルは `src/**/*.test.ts` に配置する。

### Rust (cargo test)

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```
