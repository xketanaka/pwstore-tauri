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
            // OTP
            commands::generate_otp,
            // ウィンドウ操作
            commands::resize_and_center,
            commands::quit,
            // Google OAuth
            oauth::start_oauth,
            oauth::handle_oauth_callback,
            // Google Drive 同期
            drive::drive_download,
            drive::drive_sync,
            drive::drive_force_download,
            drive::drive_force_upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
