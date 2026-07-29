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

## Production ビルド(for Android)

Mac / Linux どちらでも実行できる。

### Android.1 前提ツールのインストール

[Android Studio](https://developer.android.com/studio) をインストールし、SDK Manager で以下を確認する：

- Android SDK（API 24 以上）
- Android NDK
- Android SDK Command-line Tools

**環境変数を設定**（`~/.zshrc` や `~/.bashrc` に追記）：

Mac:
```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/$(ls $ANDROID_HOME/ndk | tail -1)"
```

Linux:
```bash
export JAVA_HOME="$HOME/android-studio/jbr"
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/$(ls $ANDROID_HOME/ndk | tail -1)"
```

### Android.2 Rust の Android ターゲットを追加

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Android.3 Android プロジェクトの初期化（初回のみ）

```bash
npm run tauri android init
```

`src-tauri/gen/android/` が生成される。

### Android.4 ビルド

```bash
npm run tauri android build
```

成果物：
```
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
```

### Android.5 端末へのインストール

Android 端末の「開発者向けオプション」で USB デバッグを有効にして接続し：

```bash
adb install src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
```

## Production ビルド(for Android by Docker)

Dockerコンテナを使ってAndroid版のビルドを行う手順

### Android.1 イメージのビルド

```bash
docker build -f Dockerfile.android -t pwstore-android-builder .
```

### Android.2 ビルド

リポジトリ外の dot.env ファイルを --env-file として渡す

```
docker run --rm \
    -v "$(pwd):/workspace" \
    --env-file src-tauri/../pwstore-tauri-private/dot.env \
    pwstore-android-builder \
    bash -c "npm install && npm run tauri android build"
```

### Android. 3 成果物

```
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
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
