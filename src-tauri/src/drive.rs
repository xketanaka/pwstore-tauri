use tauri::{AppHandle, State};

use crate::commands::{self, AppState};
use crate::oauth;

const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DATA_FILE_NAME: &str = "data.enc";

// ---- 純粋ヘルパー（テスト可能） ----

/// トークンリフレッシュレスポンスから access_token を取り出す
pub(crate) fn parse_access_token(json: &serde_json::Value) -> Result<String, String> {
    if let Some(err) = json.get("error") {
        return Err(format!("トークンリフレッシュエラー: {}", err));
    }
    json["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "アクセストークンを取得できませんでした".to_string())
}

/// ファイル一覧レスポンスから最初のファイルIDを取り出す
pub(crate) fn parse_file_id(json: &serde_json::Value) -> Result<Option<String>, String> {
    if let Some(err) = json.get("error") {
        return Err(format!("Drive APIエラー: {}", err));
    }
    Ok(json["files"]
        .as_array()
        .and_then(|files| files.first())
        .and_then(|f| f["id"].as_str())
        .map(|s| s.to_string()))
}

/// multipart/related ボディを構築する（parent_id を指定するとフォルダ内に作成）
pub(crate) fn build_multipart_body(
    data: &[u8],
    file_name: &str,
    boundary: &str,
    parent_id: Option<&str>,
) -> Vec<u8> {
    let metadata = match parent_id {
        Some(pid) => format!("{{\"name\":\"{}\",\"parents\":[\"{}\"]}}", file_name, pid),
        None => format!("{{\"name\":\"{}\"}}", file_name),
    };
    let mut body = format!(
        "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n\
         {metadata}\r\n\
         --{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
    body
}

// ---- 変更検出ヘルパー ----

/// エントリ一覧の FNV-1a ハッシュ（変更検出用、非暗号）
pub(crate) fn entries_hash(entries: &[crate::models::Entry]) -> String {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|e| e.id);
    let json = serde_json::to_string(&sorted).unwrap_or_default();
    let h = json.bytes().fold(14695981039346656037u64, |acc, b| {
        acc.wrapping_mul(1099511628211).wrapping_add(b as u64)
    });
    format!("{:016x}", h)
}

// ---- HTTP ヘルパー ----

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

    parse_access_token(&res.json().await.map_err(|e| e.to_string())?)
}

/// "pwstore" フォルダを検索し、なければ作成して ID を返す
async fn find_or_create_folder(access_token: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    // 既存フォルダを検索
    let res = client
        .get(DRIVE_FILES_URL)
        .bearer_auth(access_token)
        .query(&[
            ("q", "name='pwstore' and mimeType='application/vnd.google-apps.folder' and 'root' in parents and trashed=false"),
            ("fields", "files(id)"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("フォルダ検索エラー: {}", err));
    }
    if let Some(id) = json["files"].as_array()
        .and_then(|f| f.first())
        .and_then(|f| f["id"].as_str())
    {
        return Ok(id.to_string());
    }

    // 存在しなければ作成
    let res = client
        .post(DRIVE_FILES_URL)
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "name": "pwstore",
            "mimeType": "application/vnd.google-apps.folder",
            "parents": ["root"]
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(format!("フォルダ作成エラー: {}", err));
    }
    json["id"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "フォルダIDを取得できませんでした".to_string())
}

/// 指定フォルダ内の data.enc を検索してファイル ID を返す
async fn find_file_id(access_token: &str, folder_id: &str) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(DRIVE_FILES_URL)
        .bearer_auth(access_token)
        .query(&[
            ("q", format!("name='{}' and '{}' in parents and trashed=false", DATA_FILE_NAME, folder_id)),
            ("spaces", "drive".to_string()),
            ("fields", "files(id)".to_string()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    parse_file_id(&res.json().await.map_err(|e| e.to_string())?)
}

/// Drive からファイルの生バイトを取得する
async fn fetch_raw(access_token: &str, file_id: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/{}", DRIVE_FILES_URL, file_id))
        .bearer_auth(access_token)
        .query(&[("alt", "media")])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("ダウンロードエラー: HTTP {}", res.status()));
    }
    Ok(res.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

/// ローカルの data.enc を Drive にアップロードする（file_id があれば更新、なければ folder_id 内に新規作成）
async fn upload_to_drive(
    access_token: &str,
    data: Vec<u8>,
    file_id: Option<&str>,
    folder_id: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let json: serde_json::Value = match file_id {
        Some(id) => {
            let res = client
                .patch(format!("{}/{}", DRIVE_UPLOAD_URL, id))
                .query(&[("uploadType", "media")])
                .bearer_auth(access_token)
                .header("Content-Type", "application/octet-stream")
                .body(data)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            res.json().await.map_err(|e| e.to_string())?
        }
        None => {
            let boundary = "pwstore_drive_boundary";
            let body = build_multipart_body(&data, DATA_FILE_NAME, boundary, Some(folder_id));
            let res = client
                .post(DRIVE_UPLOAD_URL)
                .query(&[("uploadType", "multipart")])
                .bearer_auth(access_token)
                .header("Content-Type", format!("multipart/related; boundary={}", boundary))
                .body(body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            res.json().await.map_err(|e| e.to_string())?
        }
    };
    if let Some(err) = json.get("error") {
        return Err(format!("アップロードエラー: {}", err));
    }
    Ok(())
}

// ---- Tauriコマンド ----

/// 起動時ダウンロード: Drive のデータでローカルを上書きし sync_hash を記録
#[tauri::command]
pub async fn drive_download(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let access_token = refresh_access_token(&app).await?;
    let folder_id = find_or_create_folder(&access_token).await?;
    let file_id = find_file_id(&access_token, &folder_id)
        .await?
        .ok_or("Driveにデータが見つかりません")?;

    let data = fetch_raw(&access_token, &file_id).await?;
    std::fs::write(commands::data_file_path(&app)?, &data).map_err(|e| e.to_string())?;
    commands::do_unlock(&app, &state)?;

    let hash = {
        let guard = state.store.lock().unwrap();
        guard.as_ref().map(|s| entries_hash(&s.entries)).unwrap_or_default()
    };
    let _ = commands::save_secret(&app, "sync_hash", &hash);
    Ok(())
}

/// 同期: ダウンロード→競合チェック→アップロード
#[tauri::command]
pub async fn drive_sync(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let access_token = refresh_access_token(&app).await?;

    let local_hash = {
        let guard = state.store.lock().unwrap();
        let store = guard.as_ref().ok_or("ストアがロックされています")?;
        entries_hash(&store.entries)
    };
    let last_hash = commands::load_secret(&app, "sync_hash").ok();
    let folder_id = find_or_create_folder(&access_token).await?;
    let file_id = find_file_id(&access_token, &folder_id).await?;

    match file_id {
        None => {
            // Drive にファイルなし → そのままアップロード
            let data = std::fs::read(commands::data_file_path(&app)?).map_err(|e| e.to_string())?;
            upload_to_drive(&access_token, data, None, &folder_id).await?;
            commands::save_secret(&app, "sync_hash", &local_hash)?;
        }
        Some(ref id) => {
            let drive_raw = fetch_raw(&access_token, id).await?;

            // Drive データを復号してハッシュ計算
            let drive_hash = {
                let guard = state.passphrase.lock().unwrap();
                let passphrase = guard.as_ref().ok_or("パスフレーズが設定されていません")?;
                let json = crate::crypto::decrypt(&drive_raw, passphrase)?;
                let store: crate::models::DataStore =
                    serde_json::from_slice(&json).map_err(|e| e.to_string())?;
                entries_hash(&store.entries)
            };

            if drive_hash == local_hash {
                // 差分なし → ハッシュだけ保存して終了
                commands::save_secret(&app, "sync_hash", &local_hash)?;
                return Ok(());
            }

            match last_hash.as_deref() {
                Some(last) => {
                    let drive_changed = drive_hash != last;
                    let local_changed = local_hash != last;

                    if drive_changed && local_changed {
                        return Err(
                            "競合が検出されました。他のデバイスでも変更が行われています。".to_string(),
                        );
                    }

                    if drive_changed {
                        // Drive だけ変更 → ダウンロードしてローカル更新
                        std::fs::write(commands::data_file_path(&app)?, &drive_raw)
                            .map_err(|e| e.to_string())?;
                        commands::do_unlock(&app, &state)?;
                        commands::save_secret(&app, "sync_hash", &drive_hash)?;
                        return Ok(());
                    }

                    // ローカルだけ変更 → アップロード
                    let data = std::fs::read(commands::data_file_path(&app)?)
                        .map_err(|e| e.to_string())?;
                    upload_to_drive(&access_token, data, Some(id), &folder_id).await?;
                    commands::save_secret(&app, "sync_hash", &local_hash)?;
                }
                None => {
                    // 同期履歴なし（初回同期）
                    let local_empty = {
                        let guard = state.store.lock().unwrap();
                        guard.as_ref().map(|s| s.entries.is_empty()).unwrap_or(true)
                    };
                    if local_empty {
                        // ローカルが空 → Drive データをダウンロード
                        std::fs::write(commands::data_file_path(&app)?, &drive_raw)
                            .map_err(|e| e.to_string())?;
                        commands::do_unlock(&app, &state)?;
                        commands::save_secret(&app, "sync_hash", &drive_hash)?;
                    } else {
                        return Err(
                            "競合が検出されました。Driveとローカルにそれぞれデータがあります。".to_string(),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

// ---- テスト ----

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_access_token ----

    #[test]
    fn parse_access_token_returns_token_on_success() {
        let json = serde_json::json!({ "access_token": "ya29.abc123", "token_type": "Bearer" });
        assert_eq!(parse_access_token(&json).unwrap(), "ya29.abc123");
    }

    #[test]
    fn parse_access_token_returns_err_on_error_field() {
        let json = serde_json::json!({ "error": "invalid_grant", "error_description": "Token expired" });
        let err = parse_access_token(&json).unwrap_err();
        assert!(err.contains("トークンリフレッシュエラー"));
        assert!(err.contains("invalid_grant"));
    }

    #[test]
    fn parse_access_token_returns_err_when_field_missing() {
        let json = serde_json::json!({ "token_type": "Bearer" });
        assert!(parse_access_token(&json).is_err());
    }

    // ---- parse_file_id ----

    #[test]
    fn parse_file_id_returns_id_when_file_exists() {
        let json = serde_json::json!({ "files": [{ "id": "file_abc123" }] });
        assert_eq!(parse_file_id(&json).unwrap(), Some("file_abc123".to_string()));
    }

    #[test]
    fn parse_file_id_returns_none_when_files_empty() {
        let json = serde_json::json!({ "files": [] });
        assert_eq!(parse_file_id(&json).unwrap(), None);
    }

    #[test]
    fn parse_file_id_returns_first_when_multiple_files() {
        let json = serde_json::json!({
            "files": [{ "id": "first_id" }, { "id": "second_id" }]
        });
        assert_eq!(parse_file_id(&json).unwrap(), Some("first_id".to_string()));
    }

    #[test]
    fn parse_file_id_returns_err_on_error_field() {
        let json = serde_json::json!({ "error": { "code": 401, "message": "Unauthorized" } });
        let err = parse_file_id(&json).unwrap_err();
        assert!(err.contains("Drive APIエラー"));
    }

    // ---- build_multipart_body ----

    #[test]
    fn build_multipart_body_contains_filename() {
        let body = build_multipart_body(b"binary_data", "data.enc", "boundary123", None);
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"name\":\"data.enc\""));
    }

    #[test]
    fn build_multipart_body_contains_data() {
        let data = b"hello world";
        let body = build_multipart_body(data, "data.enc", "boundary123", None);
        assert!(body.windows(data.len()).any(|w| w == data));
    }

    #[test]
    fn build_multipart_body_uses_boundary() {
        let body = build_multipart_body(b"x", "f", "my_boundary", None);
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("--my_boundary\r\n"));
        assert!(text.contains("--my_boundary--"));
    }

    #[test]
    fn build_multipart_body_has_correct_content_types() {
        let body = build_multipart_body(b"x", "f", "b", None);
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("Content-Type: application/json; charset=UTF-8"));
        assert!(text.contains("Content-Type: application/octet-stream"));
    }

    #[test]
    fn build_multipart_body_includes_parent_id_when_given() {
        let body = build_multipart_body(b"x", "data.enc", "b", Some("folder_xyz"));
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"parents\":[\"folder_xyz\"]"));
    }

    #[test]
    fn build_multipart_body_no_parents_when_none() {
        let body = build_multipart_body(b"x", "data.enc", "b", None);
        let text = String::from_utf8_lossy(&body);
        assert!(!text.contains("parents"));
    }
}
