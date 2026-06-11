pub mod commands;
pub mod crypto;
pub mod models;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::is_initialized,
            commands::save_credentials,
            commands::get_google_account,
            commands::unlock,
            commands::search_entries,
            commands::upsert_entry,
            commands::delete_entry,
            commands::import_flat,
            commands::export_flat,
            commands::generate_otp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
