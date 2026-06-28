use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

use crate::crypto;
use crate::models::{DataStore, Entry, FlatEntry};

pub struct AppState {
    pub store: Mutex<Option<DataStore>>,
    pub passphrase: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(None),
            passphrase: Mutex::new(None),
        }
    }
}

// ---- 内部ヘルパー（Tauri非依存・テスト可能） ----

pub fn filter_entries<'a>(entries: &'a [Entry], keyword: &str) -> Vec<&'a Entry> {
    if keyword.is_empty() {
        return entries.iter().collect();
    }
    let kw = keyword.to_lowercase();
    entries.iter()
        .filter(|e| {
            e.service_name.to_lowercase().contains(&kw)
                || e.keyword.to_lowercase().contains(&kw)
                || e.account.to_lowercase().contains(&kw)
        })
        .collect()
}

pub fn apply_upsert(entries: &mut Vec<Entry>, mut entry: Entry) -> Entry {
    if entry.id > 0 {
        if let Some(existing) = entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            return entry;
        }
    }
    entry.id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    entries.push(entry.clone());
    entry
}

pub fn apply_import(entries: &mut Vec<Entry>, flat_entries: Vec<FlatEntry>) -> usize {
    let count = flat_entries.len();
    for flat in flat_entries {
        let entry: Entry = flat.into();
        match entries.iter_mut().find(|e| e.id == entry.id) {
            Some(existing) => *existing = entry,
            None => entries.push(entry),
        }
    }
    count
}

// ---- Tauri依存ヘルパー ----

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn data_file_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(data_dir(app)?.join("data.enc"))
}


// ファイル難読化用の固定キー（セキュリティの本質ではなく、プレーンテキスト検索への引っかかりを防ぐ目的）
const SECRET_FILE_KEY: &str = "b7Qx2#mKpL9vRnYc4dEzWsA0fJhU6tGi";

/// 秘密情報をAES暗号化してファイルに保存する（passphrase / refresh_token など）
pub fn save_secret(app: &AppHandle, name: &str, value: &str) -> Result<(), String> {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let path = data_dir(app)?.join(name);
    let encrypted = crypto::encrypt(value.as_bytes(), SECRET_FILE_KEY)?;
    std::fs::write(&path, encrypted).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 秘密情報ファイルを復号して返す
pub fn load_secret(app: &AppHandle, name: &str) -> Result<String, String> {
    let path = data_dir(app)?.join(name);
    let encrypted = std::fs::read(&path)
        .map_err(|_| format!("{name} が見つかりません"))?;
    let bytes = crypto::decrypt(&encrypted, SECRET_FILE_KEY)
        .map_err(|_| format!("{name} の読み込みに失敗しました"))?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn save_passphrase_file(app: &AppHandle, passphrase: &str) -> Result<(), String> {
    save_secret(app, "passphrase", passphrase)
}

fn load_passphrase_file(app: &AppHandle) -> Result<String, String> {
    load_secret(app, "passphrase")
        .map_err(|_| "パスフレーズが見つかりません。初期化してください。".to_string())
}

fn persist(app: &AppHandle, store: &DataStore, state: &AppState) -> Result<(), String> {
    let guard = state.passphrase.lock().unwrap();
    let passphrase = guard.as_ref().ok_or("セッションが無効です。再起動してください。")?;
    let json = serde_json::to_vec(store).map_err(|e| e.to_string())?;
    let encrypted = crypto::encrypt(&json, passphrase)?;
    std::fs::write(data_file_path(app)?, encrypted).map_err(|e| e.to_string())
}

// ---- Tauriコマンド ----

/// config.json に client_id が設定済みなら初期化済みとみなす
#[tauri::command]
pub fn is_initialized(app: AppHandle) -> bool {
    let Ok(dir) = app.path().app_config_dir() else { return false };
    let Ok(s) = std::fs::read_to_string(dir.join("config.json")) else { return false };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) else { return false };
    v["google_client_id"].as_str().is_some_and(|s| !s.is_empty())
}

/// パスフレーズをファイルに保存し、セッションにも保持する
#[tauri::command]
pub fn save_credentials(
    app: AppHandle,
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    save_passphrase_file(&app, &passphrase)?;
    *state.passphrase.lock().unwrap() = Some(passphrase);
    Ok(())
}

/// ファイルからパスフレーズを読んでデータを復号しセッションに保持する
#[tauri::command]
pub fn unlock(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let passphrase = load_passphrase_file(&app)?;
    let path = data_file_path(&app)?;
    let store = if path.exists() {
        let encrypted = std::fs::read(&path).map_err(|e| e.to_string())?;
        let json = crypto::decrypt(&encrypted, &passphrase)?;
        serde_json::from_slice(&json).map_err(|e| e.to_string())?
    } else {
        DataStore::new()
    };
    *state.passphrase.lock().unwrap() = Some(passphrase);
    *state.store.lock().unwrap() = Some(store);
    Ok(())
}

#[tauri::command]
pub fn search_entries(keyword: String, state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let guard = state.store.lock().unwrap();
    let store = guard.as_ref().ok_or("ストアがロックされています")?;
    Ok(filter_entries(&store.entries, &keyword).into_iter().cloned().collect())
}

#[tauri::command]
pub fn upsert_entry(app: AppHandle, entry: Entry, state: State<'_, AppState>) -> Result<Entry, String> {
    let mut guard = state.store.lock().unwrap();
    let store = guard.as_mut().ok_or("ストアがロックされています")?;
    let saved = apply_upsert(&mut store.entries, entry);
    persist(&app, store, &state)?;
    Ok(saved)
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, id: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.store.lock().unwrap();
    let store = guard.as_mut().ok_or("ストアがロックされています")?;
    store.entries.retain(|e| e.id != id);
    persist(&app, store, &state)
}

#[tauri::command]
pub fn import_flat(
    app: AppHandle,
    entries: Vec<FlatEntry>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let mut guard = state.store.lock().unwrap();
    let store = guard.as_mut().ok_or("ストアがロックされています")?;
    let count = apply_import(&mut store.entries, entries);
    persist(&app, store, &state)?;
    Ok(count)
}

#[tauri::command]
pub fn export_flat(state: State<'_, AppState>) -> Result<Vec<FlatEntry>, String> {
    let guard = state.store.lock().unwrap();
    let store = guard.as_ref().ok_or("ストアがロックされています")?;
    Ok(store.entries.iter().cloned().map(FlatEntry::from).collect())
}

#[tauri::command]
pub fn generate_otp(otp_uri: String) -> Result<(String, u64), String> {
    let totp = totp_rs::TOTP::from_url(&otp_uri).map_err(|e| e.to_string())?;
    let code = totp.generate_current().map_err(|e| e.to_string())?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let remaining = totp.step - (secs % totp.step);
    Ok((code, remaining))
}

// ---- テスト ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Entry, FlatEntry};

    fn make_entry(id: u32, service: &str, account: &str, keyword: &str) -> Entry {
        Entry {
            id,
            service_name: service.to_string(),
            account: account.to_string(),
            password: "pass".to_string(),
            url: None,
            keyword: keyword.to_string(),
            category: "test".to_string(),
            otp_uri: None,
            notes: None,
            status: 1,
            extra_fields: vec![],
        }
    }

    // --- filter_entries ---

    #[test]
    fn filter_empty_keyword_returns_all() {
        let entries = vec![
            make_entry(1, "AWS", "alice", "cloud"),
            make_entry(2, "Google", "bob", "search"),
        ];
        assert_eq!(filter_entries(&entries, "").len(), 2);
    }

    #[test]
    fn filter_matches_service_name() {
        let entries = vec![
            make_entry(1, "AWS", "alice", "cloud"),
            make_entry(2, "Google", "bob", "search"),
        ];
        let result = filter_entries(&entries, "aws");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].service_name, "AWS");
    }

    #[test]
    fn filter_matches_keyword_field() {
        let entries = vec![
            make_entry(1, "AWS", "alice", "cloud infra"),
            make_entry(2, "Google", "bob", "search mail"),
        ];
        let result = filter_entries(&entries, "infra");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn filter_matches_account() {
        let entries = vec![
            make_entry(1, "AWS", "alice@example.com", ""),
            make_entry(2, "Google", "bob@example.com", ""),
        ];
        let result = filter_entries(&entries, "alice");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn filter_is_case_insensitive() {
        let entries = vec![make_entry(1, "GitHub", "dev", "")];
        assert_eq!(filter_entries(&entries, "GITHUB").len(), 1);
        assert_eq!(filter_entries(&entries, "github").len(), 1);
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let entries = vec![make_entry(1, "AWS", "alice", "cloud")];
        assert_eq!(filter_entries(&entries, "zzz").len(), 0);
    }

    // --- apply_upsert ---

    #[test]
    fn upsert_new_entry_assigns_id() {
        let mut entries = vec![make_entry(5, "AWS", "alice", "")];
        let new = make_entry(0, "Google", "bob", "");
        let saved = apply_upsert(&mut entries, new);
        assert_eq!(saved.id, 6);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn upsert_first_entry_gets_id_1() {
        let mut entries = vec![];
        let new = make_entry(0, "AWS", "alice", "");
        let saved = apply_upsert(&mut entries, new);
        assert_eq!(saved.id, 1);
    }

    #[test]
    fn upsert_updates_existing_entry() {
        let mut entries = vec![make_entry(1, "AWS", "alice", "")];
        let mut updated = make_entry(1, "AWS", "alice-updated", "");
        updated.password = "newpass".to_string();
        apply_upsert(&mut entries, updated);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].account, "alice-updated");
    }

    #[test]
    fn upsert_nonexistent_id_creates_new() {
        let mut entries = vec![make_entry(1, "AWS", "alice", "")];
        let new = make_entry(99, "Google", "bob", "");
        let saved = apply_upsert(&mut entries, new);
        assert_eq!(saved.id, 2); // 99 は存在しないので新規扱い→ max+1
        assert_eq!(entries.len(), 2);
    }

    // --- apply_import ---

    #[test]
    fn import_adds_new_entries() {
        let mut entries = vec![];
        let flat = FlatEntry {
            id: 10,
            service_name: "AWS".to_string(),
            account: "alice".to_string(),
            password: "pass".to_string(),
            status: 1,
            keyword: "cloud".to_string(),
            category: "biz".to_string(),
            extra1_key_name: None, extra1_value: None, extra1_encrypted: None,
            extra2_key_name: None, extra2_value: None, extra2_encrypted: None,
            extra3_key_name: None, extra3_value: None, extra3_encrypted: None,
        };
        let count = apply_import(&mut entries, vec![flat]);
        assert_eq!(count, 1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, 10);
    }

    #[test]
    fn import_overwrites_existing_id() {
        let mut entries = vec![make_entry(10, "AWS", "alice", "")];
        let flat = FlatEntry {
            id: 10,
            service_name: "AWS".to_string(),
            account: "alice-new".to_string(),
            password: "pass".to_string(),
            status: 1,
            keyword: "".to_string(),
            category: "".to_string(),
            extra1_key_name: None, extra1_value: None, extra1_encrypted: None,
            extra2_key_name: None, extra2_value: None, extra2_encrypted: None,
            extra3_key_name: None, extra3_value: None, extra3_encrypted: None,
        };
        apply_import(&mut entries, vec![flat]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].account, "alice-new");
    }

    // --- generate_otp ---

    #[test]
    fn generate_otp_valid_uri_returns_6_digit_code() {
        // RFC 6238 テストベクタ: secret = "12345678901234567890" (20バイト = 160bit)
        let uri = "otpauth://totp/Example:alice@example.com?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=Example";
        let (code, remaining) = generate_otp(uri.to_string()).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        assert!((1..=30).contains(&remaining));
    }

    #[test]
    fn generate_otp_invalid_uri_returns_error() {
        let result = generate_otp("not-a-valid-uri".to_string());
        assert!(result.is_err());
    }
}
