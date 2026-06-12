use std::sync::Mutex;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use keyring::Entry as KeyringEntry;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tauri_plugin_opener::OpenerExt;

const KEYRING_SERVICE: &str = "pwstore-tauri";
const KEYRING_CLIENT_ID_KEY: &str = "google_client_id";
const KEYRING_REFRESH_TOKEN_KEY: &str = "google_refresh_token";

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REDIRECT_URI: &str = "pwstore://oauth/callback";
const SCOPES: &str = "https://www.googleapis.com/auth/drive.file";

// OAuthフロー中にcode_verifierをメモリ上で保持する
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

// ---- keyring ヘルパー ----

pub fn get_refresh_token() -> Result<String, String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_KEY)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "リフレッシュトークンが見つかりません。再認証してください。".to_string())
}

// ---- Tauriコマンド ----

#[tauri::command]
pub fn save_client_id(client_id: String) -> Result<(), String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_CLIENT_ID_KEY)
        .map_err(|e| e.to_string())?
        .set_password(&client_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_client_id() -> Result<String, String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_CLIENT_ID_KEY)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "クライアントIDが設定されていません".to_string())
}

/// PKCE code_verifier を生成してAppStateに保存し、ブラウザでGoogle認証画面を開く
#[tauri::command]
pub fn start_oauth(
    app: tauri::AppHandle,
    oauth_state: tauri::State<'_, OAuthState>,
) -> Result<(), String> {
    let client_id = get_client_id()?;
    let verifier = generate_code_verifier();
    let challenge = code_challenge(&verifier);

    *oauth_state.code_verifier.lock().unwrap() = Some(verifier);

    let auth_url = format!(
        "{url}?client_id={client_id}&redirect_uri={redirect}&response_type=code\
         &scope={scope}&code_challenge={challenge}&code_challenge_method=S256\
         &access_type=offline&prompt=consent",
        url = GOOGLE_AUTH_URL,
        client_id = urlencoding(client_id),
        redirect = urlencoding(REDIRECT_URI.to_string()),
        scope = urlencoding(SCOPES.to_string()),
        challenge = challenge,
    );

    app.opener()
        .open_url(auth_url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// deep-link で受け取ったコールバック URL を処理し、トークンを keyring に保存する
#[tauri::command]
pub async fn handle_oauth_callback(
    url: String,
    oauth_state: tauri::State<'_, OAuthState>,
) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;

    // エラーチェック
    if let Some((_, msg)) = parsed.query_pairs().find(|(k, _)| k == "error") {
        return Err(format!("Google認証エラー: {}", msg));
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

    let client_id = get_client_id()?;

    // 認証コード → トークン交換
    let client = reqwest::Client::new();
    let res = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code.as_str()),
            ("client_id", &client_id),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", &verifier),
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

fn urlencoding(s: String) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
