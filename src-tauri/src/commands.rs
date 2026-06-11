use std::sync::Mutex;
use keyring::Entry as KeyringEntry;
use tauri::{AppHandle, Manager, State};

use crate::crypto;
use crate::models::{DataStore, Entry, FlatEntry};

const KEYRING_SERVICE: &str = "pwstore-tauri";
const KEYRING_ACCOUNT_KEY: &str = "google_account";
const KEYRING_PASSPHRASE_KEY: &str = "master_passphrase";

// アプリ起動中にメモリ上で保持するストア
pub struct AppState {
    pub store: Mutex<Option<DataStore>>,
}

impl AppState {
    pub fn new() -> Self {
        Self { store: Mutex::new(None) }
    }
}

// ---- 内部ヘルパー ----

fn data_file_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("data.enc"))
}

fn get_passphrase() -> Result<String, String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_PASSPHRASE_KEY)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "パスフレーズが見つかりません。初期化してください。".to_string())
}

fn persist(app: &AppHandle, store: &DataStore) -> Result<(), String> {
    let passphrase = get_passphrase()?;
    let json = serde_json::to_vec(store).map_err(|e| e.to_string())?;
    let encrypted = crypto::encrypt(&json, &passphrase)?;
    std::fs::write(data_file_path(app)?, encrypted).map_err(|e| e.to_string())
}

// ---- 初期化 ----

#[tauri::command]
pub fn is_initialized() -> bool {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_PASSPHRASE_KEY)
        .ok()
        .and_then(|e| e.get_password().ok())
        .is_some()
}

#[tauri::command]
pub fn save_credentials(google_account: String, passphrase: String) -> Result<(), String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_ACCOUNT_KEY)
        .map_err(|e| e.to_string())?
        .set_password(&google_account)
        .map_err(|e| e.to_string())?;
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_PASSPHRASE_KEY)
        .map_err(|e| e.to_string())?
        .set_password(&passphrase)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_google_account() -> Result<String, String> {
    KeyringEntry::new(KEYRING_SERVICE, KEYRING_ACCOUNT_KEY)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "Googleアカウントが見つかりません。".to_string())
}

// ---- ストア操作 ----

/// 起動時に呼ぶ。keyring からパスフレーズを取得してデータを復号しメモリに展開する
#[tauri::command]
pub fn unlock(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let passphrase = get_passphrase()?;
    let path = data_file_path(&app)?;

    let store = if path.exists() {
        let encrypted = std::fs::read(&path).map_err(|e| e.to_string())?;
        let json = crypto::decrypt(&encrypted, &passphrase)?;
        serde_json::from_slice(&json).map_err(|e| e.to_string())?
    } else {
        DataStore::new()
    };

    *state.store.lock().unwrap() = Some(store);
    Ok(())
}

// ---- エントリ操作 ----

#[tauri::command]
pub fn search_entries(keyword: String, state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let guard = state.store.lock().unwrap();
    let store = guard.as_ref().ok_or("ストアがロックされています")?;

    if keyword.is_empty() {
        return Ok(store.entries.clone());
    }

    let kw = keyword.to_lowercase();
    Ok(store.entries.iter()
        .filter(|e| {
            e.service_name.to_lowercase().contains(&kw)
                || e.keyword.to_lowercase().contains(&kw)
                || e.account.to_lowercase().contains(&kw)
        })
        .cloned()
        .collect())
}

/// 新規作成（id=0）または更新（id>0）を兼ねる
#[tauri::command]
pub fn upsert_entry(app: AppHandle, entry: Entry, state: State<'_, AppState>) -> Result<Entry, String> {
    let mut guard = state.store.lock().unwrap();
    let store = guard.as_mut().ok_or("ストアがロックされています")?;

    if entry.id > 0 {
        if let Some(existing) = store.entries.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
            persist(&app, store)?;
            return Ok(entry);
        }
    }

    // 新規
    let mut new_entry = entry;
    new_entry.id = store.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    store.entries.push(new_entry.clone());
    persist(&app, store)?;
    Ok(new_entry)
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, id: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.store.lock().unwrap();
    let store = guard.as_mut().ok_or("ストアがロックされています")?;
    store.entries.retain(|e| e.id != id);
    persist(&app, store)
}

// ---- インポート／エクスポート ----

#[tauri::command]
pub fn import_flat(
    app: AppHandle,
    entries: Vec<FlatEntry>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let mut guard = state.store.lock().unwrap();
    let store = guard.as_mut().ok_or("ストアがロックされています")?;

    let count = entries.len();
    for flat in entries {
        let entry: Entry = flat.into();
        match store.entries.iter_mut().find(|e| e.id == entry.id) {
            Some(existing) => *existing = entry,
            None => store.entries.push(entry),
        }
    }

    persist(&app, store)?;
    Ok(count)
}

#[tauri::command]
pub fn export_flat(state: State<'_, AppState>) -> Result<Vec<FlatEntry>, String> {
    let guard = state.store.lock().unwrap();
    let store = guard.as_ref().ok_or("ストアがロックされています")?;
    Ok(store.entries.iter().cloned().map(FlatEntry::from).collect())
}

// ---- OTP ----

/// OTP-URI から現在のコードと残り秒数を返す
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
