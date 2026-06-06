mod commands;
mod crypto;
mod ssh;
mod state;
mod storage;
mod sync;
mod ws;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::create_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::change_master_password,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::get_preferences,
            commands::save_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
