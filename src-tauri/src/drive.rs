use tauri::{AppHandle, State};

use crate::commands::{self, AppState};
use crate::oauth;

const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DATA_FILE_NAME: &str = "data.enc";

// ---- 内部ヘルパー ----

async fn refresh_access_token(app: &AppHandle) -> Result<String, String> {
    let refresh_token = oauth::get_refresh_token(app)?;
    let client_id = oauth::get_client_id(app.clone())?;
    let client_secret = oauth::get_client_secret(app.clone())?;

    let client = reqwest::Client::new();
    let res = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

    if let Some(err) = json.get("error") {
        return Err(format!("トークンリフレッシュエラー: {}", err));
    }

    json["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "アクセストークンを取得できませんでした".to_string())
}

async fn find_file_id(access_token: &str) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(DRIVE_FILES_URL)
        .bearer_auth(access_token)
        .query(&[
            ("q", format!("name='{}' and trashed=false", DATA_FILE_NAME)),
            ("spaces", "drive".to_string()),
            ("fields", "files(id)".to_string()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

    if let Some(err) = json.get("error") {
        return Err(format!("Drive APIエラー: {}", err));
    }

    Ok(json["files"]
        .as_array()
        .and_then(|files| files.first())
        .and_then(|f| f["id"].as_str())
        .map(|s| s.to_string()))
}

// ---- Tauriコマンド ----

/// ローカルの data.enc を Google Drive にアップロード（存在すれば上書き）
#[tauri::command]
pub async fn drive_upload(app: AppHandle) -> Result<(), String> {
    let access_token = refresh_access_token(&app).await?;

    let data_path = commands::data_file_path(&app)?;
    if !data_path.exists() {
        return Err("アップロードするデータがありません".to_string());
    }
    let data = std::fs::read(&data_path).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();

    match find_file_id(&access_token).await? {
        Some(id) => {
            // 既存ファイルをメディアアップロードで更新
            let res = client
                .patch(format!("{}/{}", DRIVE_UPLOAD_URL, id))
                .query(&[("uploadType", "media")])
                .bearer_auth(&access_token)
                .header("Content-Type", "application/octet-stream")
                .body(data)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
            if let Some(err) = json.get("error") {
                return Err(format!("アップロードエラー: {}", err));
            }
        }
        None => {
            // 新規作成（multipart: JSON メタデータ + バイナリ）
            let boundary = "pwstore_drive_boundary";
            let metadata = format!("{{\"name\":\"{}\"}}", DATA_FILE_NAME);
            let mut body = format!(
                "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n\
                 {metadata}\r\n\
                 --{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .into_bytes();
            body.extend_from_slice(&data);
            body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());

            let res = client
                .post(DRIVE_UPLOAD_URL)
                .query(&[("uploadType", "multipart")])
                .bearer_auth(&access_token)
                .header(
                    "Content-Type",
                    format!("multipart/related; boundary={}", boundary),
                )
                .body(body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
            if let Some(err) = json.get("error") {
                return Err(format!("アップロードエラー: {}", err));
            }
        }
    }

    Ok(())
}

/// Google Drive から data.enc をダウンロードしてローカルに上書き・AppState を再読み込み
#[tauri::command]
pub async fn drive_download(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let access_token = refresh_access_token(&app).await?;

    let file_id = find_file_id(&access_token)
        .await?
        .ok_or("Driveにデータが見つかりません")?;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/{}", DRIVE_FILES_URL, file_id))
        .bearer_auth(&access_token)
        .query(&[("alt", "media")])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("ダウンロードエラー: HTTP {}", res.status()));
    }

    let data = res.bytes().await.map_err(|e| e.to_string())?;

    let data_path = commands::data_file_path(&app)?;
    std::fs::write(&data_path, &data).map_err(|e| e.to_string())?;

    // ダウンロードしたデータで AppState を再読み込み
    commands::do_unlock(&app, &state)?;

    Ok(())
}
