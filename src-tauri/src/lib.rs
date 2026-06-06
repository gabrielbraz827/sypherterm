mod commands;
mod crypto;
mod ssh;
mod state;
mod storage;
mod sync;
mod ws;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .setup(|app| {
            let app_state = app.state::<state::AppState>();
            app_state.set_data_plane_state(state::DataPlaneState::Starting)?;
            let stream_server = tauri::async_runtime::block_on(ws::StreamServer::start())?;
            app_state.set_data_plane_state(state::DataPlaneState::Running)?;
            app.manage(stream_server);
            Ok(())
        })
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
            commands::open_data_plane_session,
            commands::connect_ssh,
            commands::disconnect_session,
            commands::resize_session,
            commands::trigger_cloud_sync,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::list_tunnels,
            commands::list_session_tunnels,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
