use tauri::{AppHandle, State};

use crate::crypto::{
    change_master_password as change_stored_master_password, create_vault as create_stored_vault,
    lock_vault as lock_stored_vault, unlock_vault as unlock_stored_vault,
    vault_status as stored_vault_status, ChangeMasterPasswordRequest, CreateVaultRequest,
    UnlockVaultRequest, VaultError, VaultStatus,
};
use crate::state::{AppState, AppStatus};
use crate::storage::{
    delete_profile as delete_stored_profile, get_preferences as get_stored_preferences,
    list_profiles as list_stored_profiles, save_preferences as save_stored_preferences,
    save_profile as save_stored_profile, ConnectionProfile, ConnectionProfileDraft,
    ConnectionProfileSummary, DeleteResult, StorageError, UserPreferences,
};

#[tauri::command]
pub fn get_app_status(app: AppHandle, state: State<'_, AppState>) -> Result<AppStatus, String> {
    let _ = stored_vault_status(&app, &state).map_err(|error| error.to_string())?;
    state.status().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CreateVaultRequest,
) -> Result<VaultStatus, VaultError> {
    create_stored_vault(&app, &state, request)
}

#[tauri::command]
pub fn unlock_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    request: UnlockVaultRequest,
) -> Result<VaultStatus, VaultError> {
    unlock_stored_vault(&app, &state, request)
}

#[tauri::command]
pub fn lock_vault(app: AppHandle, state: State<'_, AppState>) -> Result<VaultStatus, VaultError> {
    lock_stored_vault(&app, &state)
}

#[tauri::command]
pub fn change_master_password(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChangeMasterPasswordRequest,
) -> Result<VaultStatus, VaultError> {
    change_stored_master_password(&app, &state, request)
}

#[tauri::command]
pub fn list_profiles(app: AppHandle) -> Result<Vec<ConnectionProfileSummary>, StorageError> {
    list_stored_profiles(&app)
}

#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    profile: ConnectionProfileDraft,
) -> Result<ConnectionProfile, StorageError> {
    save_stored_profile(&app, profile)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, id: String) -> Result<DeleteResult, StorageError> {
    delete_stored_profile(&app, &id)
}

#[tauri::command]
pub fn get_preferences(app: AppHandle) -> Result<UserPreferences, StorageError> {
    get_stored_preferences(&app)
}

#[tauri::command]
pub fn save_preferences(
    app: AppHandle,
    preferences: UserPreferences,
) -> Result<UserPreferences, StorageError> {
    save_stored_preferences(&app, preferences)
}
