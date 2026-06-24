use std::sync::Mutex;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use keyring::Entry as KeyringEntry;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

const KEYRING_SERVICE: &str = "pwstore-tauri";
const KEYRING_REFRESH_TOKEN_KEY: &str = "google_refresh_token";
const CONFIG_FILE: &str = "config.json";

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPES: &str = "https://www.googleapis.com/auth/drive.file";

pub struct OAuthState {
    pub code_verifier: Mutex<Option<String>>,
}

impl OAuthState {
    pub fn new() -> Self {
        Self { code_verifier: Mutex::new(None) }
    }
}

// ---- PKCE ----

fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

// ---- 設定ファイル（非シークレット値） ----

fn config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(CONFIG_FILE))
}

fn read_config(app: &tauri::AppHandle) -> serde_json::Value {
    config_path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_config(app: &tauri::AppHandle, config: &serde_json::Value) -> Result<(), String> {
    let path = config_path(app)?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ---- Keyring ヘルパー（シークレット値のみ） ----

pub fn get_refresh_token() -> Result<String, String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_KEY)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "リフレッシュトークンが見つかりません。再認証してください。".to_string())
}

// ---- Tauriコマンド ----

#[tauri::command]
pub fn save_client_id(app: tauri::AppHandle, client_id: String) -> Result<(), String> {
    let mut config = read_config(&app);
    config["google_client_id"] = serde_json::Value::String(client_id);
    write_config(&app, &config)
}

#[tauri::command]
pub fn get_client_id(app: tauri::AppHandle) -> Result<String, String> {
    let config = read_config(&app);
    config["google_client_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| "クライアントIDが設定されていません".to_string())
}

/// デスクトップ: ループバックHTTPサーバーでOAuthコールバックを受け取る
#[cfg(desktop)]
#[tauri::command]
pub async fn start_oauth(app: tauri::AppHandle) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let client_id = get_client_id(app.clone())?;
    let verifier = generate_code_verifier();
    let challenge = code_challenge(&verifier);

    // ポート0を指定してOSにランダムポートを割り当てさせる
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{}", port);

    let auth_url = format!(
        "{url}?client_id={client_id}&redirect_uri={redirect}&response_type=code\
         &scope={scope}&code_challenge={challenge}&code_challenge_method=S256\
         &access_type=offline&prompt=consent",
        url = GOOGLE_AUTH_URL,
        client_id = urlencoding(client_id.clone()),
        redirect = urlencoding(redirect_uri.clone()),
        scope = urlencoding(SCOPES.to_string()),
        challenge = challenge,
    );

    app.opener()
        .open_url(auth_url, None::<&str>)
        .map_err(|e| e.to_string())?;

    // バックグラウンドでコールバックを待つ
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;

            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
            let request = String::from_utf8_lossy(&buf[..n]);

            // "GET /?code=xxx HTTP/1.1" の1行目からクエリを解析
            let first_line = request.lines().next().ok_or("不正なHTTPリクエスト")?;
            let path = first_line.split_whitespace().nth(1).ok_or("不正なHTTPリクエスト")?;
            let query = path.split('?').nth(1).unwrap_or("");
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query.as_bytes())
                    .into_owned()
                    .collect();

            if let Some(error) = params.get("error") {
                let html = html_page("認証エラー", &format!("エラー: {}", error));
                write_http_response(&mut stream, &html).await.ok();
                return Err(format!("Google認証エラー: {}", error));
            }

            let code = params.get("code").ok_or("認証コードが見つかりません")?.clone();

            let html = html_page("認証完了", "認証が完了しました。このタブを閉じてください。");
            write_http_response(&mut stream, &html).await.ok();

            // 認証コード → トークン交換
            let client = reqwest::Client::new();
            let res = client
                .post(GOOGLE_TOKEN_URL)
                .form(&[
                    ("code", code.as_str()),
                    ("client_id", client_id.as_str()),
                    ("redirect_uri", redirect_uri.as_str()),
                    ("code_verifier", verifier.as_str()),
                    ("grant_type", "authorization_code"),
                ])
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

            if let Some(err) = json.get("error") {
                return Err(format!("トークン取得エラー: {}", err));
            }

            let refresh_token = json["refresh_token"]
                .as_str()
                .ok_or("リフレッシュトークンを取得できませんでした。Google Cloudの設定を確認してください。")?;

            KeyringEntry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_KEY)
                .map_err(|e| e.to_string())?
                .set_password(refresh_token)
                .map_err(|e| e.to_string())
        }
        .await;

        match result {
            Ok(_) => { app.emit("oauth-complete", ()).ok(); }
            Err(e) => { app.emit("oauth-error", e).ok(); }
        }
    });

    Ok(())
}

/// モバイル: deep-linkで認証を開始する
#[cfg(mobile)]
#[tauri::command]
pub fn start_oauth(
    app: tauri::AppHandle,
    oauth_state: tauri::State<'_, OAuthState>,
) -> Result<(), String> {
    const MOBILE_REDIRECT_URI: &str = "pwstore://oauth/callback";

    let client_id = get_client_id(app.clone())?;
    let verifier = generate_code_verifier();
    let challenge = code_challenge(&verifier);

    *oauth_state.code_verifier.lock().unwrap() = Some(verifier);

    let auth_url = format!(
        "{url}?client_id={client_id}&redirect_uri={redirect}&response_type=code\
         &scope={scope}&code_challenge={challenge}&code_challenge_method=S256\
         &access_type=offline&prompt=consent",
        url = GOOGLE_AUTH_URL,
        client_id = urlencoding(client_id),
        redirect = urlencoding(MOBILE_REDIRECT_URI.to_string()),
        scope = urlencoding(SCOPES.to_string()),
        challenge = challenge,
    );

    app.opener()
        .open_url(auth_url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// モバイル専用: deep-linkコールバックURLを処理してトークンを保存し、イベントを発行する
#[tauri::command]
pub async fn handle_oauth_callback(
    app: tauri::AppHandle,
    url: String,
    oauth_state: tauri::State<'_, OAuthState>,
) -> Result<(), String> {
    const MOBILE_REDIRECT_URI: &str = "pwstore://oauth/callback";

    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;

    if let Some((_, msg)) = parsed.query_pairs().find(|(k, _)| k == "error") {
        let err = format!("Google認証エラー: {}", msg);
        app.emit("oauth-error", &err).ok();
        return Err(err);
    }

    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or("認証コードが見つかりません")?;

    let verifier = oauth_state
        .code_verifier
        .lock()
        .unwrap()
        .take()
        .ok_or("OAuthセッションが見つかりません。もう一度試してください。")?;

    let client_id = get_client_id(app.clone())?;

    let client = reqwest::Client::new();
    let res = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code.as_str()),
            ("client_id", &client_id),
            ("redirect_uri", MOBILE_REDIRECT_URI),
            ("code_verifier", &verifier),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

    if let Some(err) = json.get("error") {
        let msg = format!("トークン取得エラー: {}", err);
        app.emit("oauth-error", &msg).ok();
        return Err(msg);
    }

    let refresh_token = json["refresh_token"]
        .as_str()
        .ok_or("リフレッシュトークンを取得できませんでした。Google Cloudの設定を確認してください。")?;

    KeyringEntry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_KEY)
        .map_err(|e| e.to_string())?
        .set_password(refresh_token)
        .map_err(|e| e.to_string())?;

    app.emit("oauth-complete", ()).ok();
    Ok(())
}

// ---- ユーティリティ ----

fn urlencoding(s: String) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn html_page(title: &str, message: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{title}</title></head>\
         <body style=\"font-family:sans-serif;text-align:center;padding:40px\">\
         <h2>{title}</h2><p>{message}</p></body></html>"
    )
}

#[cfg(desktop)]
async fn write_http_response(
    stream: &mut tokio::net::TcpStream,
    body: &str,
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await.map_err(|e| e.to_string())
}
