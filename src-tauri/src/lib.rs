pub mod commands;
pub mod crypto;
pub mod drive;
pub mod models;
pub mod oauth;

use commands::AppState;
use oauth::OAuthState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::new())
        .manage(OAuthState::new())
        .invoke_handler(tauri::generate_handler![
            // 認証情報
            commands::is_initialized,
            commands::save_credentials,
            commands::unlock,
            // エントリ操作
            commands::search_entries,
            commands::upsert_entry,
            commands::delete_entry,
            // カテゴリ
            commands::get_categories,
            commands::set_categories,
            // インポート／エクスポート
            commands::import_flat,
            commands::export_flat,
            // OTP
            commands::generate_otp,
            // Google OAuth
            oauth::save_client_id,
            oauth::get_client_id,
            oauth::save_client_secret,
            oauth::get_client_secret,
            oauth::start_oauth,
            oauth::handle_oauth_callback,
            // Google Drive 同期
            drive::drive_upload,
            drive::drive_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
