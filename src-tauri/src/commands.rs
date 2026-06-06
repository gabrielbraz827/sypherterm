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
use crate::ws::{DataPlaneSession, StreamServer, StreamServerError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl CommandError {
    fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }

    fn not_implemented(feature: &str) -> Self {
        Self::new(
            "not_implemented",
            format!("{feature} is defined in the Control Plane but not implemented yet"),
            true,
        )
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message, true)
    }
}

impl From<VaultError> for CommandError {
    fn from(error: VaultError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

impl From<StorageError> for CommandError {
    fn from(error: StorageError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

impl From<crate::state::AppStateError> for CommandError {
    fn from(error: crate::state::AppStateError) -> Self {
        Self::new("internal_error", error.to_string(), false)
    }
}

impl From<StreamServerError> for CommandError {
    fn from(error: StreamServerError) -> Self {
        Self::new("data_plane_error", error.to_string(), false)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectSshRequest {
    #[allow(dead_code)]
    pub profile_id: Option<String>,
    #[allow(dead_code)]
    pub host: Option<String>,
    #[allow(dead_code)]
    pub port: Option<u16>,
    #[allow(dead_code)]
    pub username: Option<String>,
    #[allow(dead_code)]
    pub credential_ref: Option<String>,
    #[allow(dead_code)]
    pub cols: u16,
    #[allow(dead_code)]
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectSshResponse {
    pub session_id: String,
    pub ws_url: String,
    pub auth_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResizeRequest {
    #[allow(dead_code)]
    pub session_id: String,
    #[allow(dead_code)]
    pub cols: u16,
    #[allow(dead_code)]
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub session_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRequest {
    #[allow(dead_code)]
    pub provider_id: String,
    #[allow(dead_code)]
    pub direction: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncJobStatus {
    pub job_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelRequest {
    #[allow(dead_code)]
    pub session_id: Option<String>,
    #[allow(dead_code)]
    pub profile_id: Option<String>,
    #[allow(dead_code)]
    pub mode: String,
    #[allow(dead_code)]
    pub bind_host: String,
    #[allow(dead_code)]
    pub bind_port: u16,
    #[allow(dead_code)]
    pub target_host: Option<String>,
    #[allow(dead_code)]
    pub target_port: Option<u16>,
    #[allow(dead_code)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub tunnel_id: String,
    pub session_id: String,
    pub mode: String,
    pub state: String,
    pub bind_host: String,
    pub bind_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[tauri::command]
pub fn get_app_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppStatus, CommandError> {
    let _ = stored_vault_status(&app, &state)?;
    state.status().map_err(Into::into)
}

#[tauri::command]
pub fn create_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CreateVaultRequest,
) -> Result<VaultStatus, CommandError> {
    create_stored_vault(&app, &state, request).map_err(Into::into)
}

#[tauri::command]
pub fn unlock_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    request: UnlockVaultRequest,
) -> Result<VaultStatus, CommandError> {
    unlock_stored_vault(&app, &state, request).map_err(Into::into)
}

#[tauri::command]
pub fn lock_vault(app: AppHandle, state: State<'_, AppState>) -> Result<VaultStatus, CommandError> {
    lock_stored_vault(&app, &state).map_err(Into::into)
}

#[tauri::command]
pub fn change_master_password(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChangeMasterPasswordRequest,
) -> Result<VaultStatus, CommandError> {
    change_stored_master_password(&app, &state, request).map_err(Into::into)
}

#[tauri::command]
pub fn list_profiles(app: AppHandle) -> Result<Vec<ConnectionProfileSummary>, CommandError> {
    list_stored_profiles(&app).map_err(Into::into)
}

#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    profile: ConnectionProfileDraft,
) -> Result<ConnectionProfile, CommandError> {
    save_stored_profile(&app, profile).map_err(Into::into)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, id: String) -> Result<DeleteResult, CommandError> {
    delete_stored_profile(&app, &id).map_err(Into::into)
}

#[tauri::command]
pub fn get_preferences(app: AppHandle) -> Result<UserPreferences, CommandError> {
    get_stored_preferences(&app).map_err(Into::into)
}

#[tauri::command]
pub fn save_preferences(
    app: AppHandle,
    preferences: UserPreferences,
) -> Result<UserPreferences, CommandError> {
    save_stored_preferences(&app, preferences).map_err(Into::into)
}

#[tauri::command]
pub async fn open_data_plane_session(
    server: State<'_, StreamServer>,
) -> Result<DataPlaneSession, CommandError> {
    Ok(server.register_session().await)
}

#[tauri::command]
pub fn connect_ssh(request: ConnectSshRequest) -> Result<ConnectSshResponse, CommandError> {
    let _ = request;
    Err(CommandError::not_implemented("SSH connections"))
}

#[tauri::command]
pub fn disconnect_session(session_id: String) -> Result<SessionStatus, CommandError> {
    Err(CommandError::not_found(format!(
        "session {session_id} is not active"
    )))
}

#[tauri::command]
pub fn resize_session(request: SessionResizeRequest) -> Result<SessionStatus, CommandError> {
    Err(CommandError::not_found(format!(
        "session {} is not active",
        request.session_id
    )))
}

#[tauri::command]
pub fn trigger_cloud_sync(request: SyncRequest) -> Result<SyncJobStatus, CommandError> {
    let _ = request;
    Err(CommandError::not_implemented("cloud sync"))
}

#[tauri::command]
pub fn start_tunnel(request: TunnelRequest) -> Result<TunnelStatus, CommandError> {
    let _ = request;
    Err(CommandError::new(
        "unsupported_mode",
        "tunnels are defined in the Control Plane but not implemented yet",
        true,
    ))
}

#[tauri::command]
pub fn stop_tunnel(tunnel_id: String) -> Result<TunnelStatus, CommandError> {
    Err(CommandError::not_found(format!(
        "tunnel {tunnel_id} is not active"
    )))
}

#[tauri::command]
pub fn list_tunnels() -> Result<Vec<TunnelStatus>, CommandError> {
    Ok(Vec::new())
}

#[tauri::command]
pub fn list_session_tunnels(session_id: String) -> Result<Vec<TunnelStatus>, CommandError> {
    let _ = session_id;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::CommandError;
    use crate::storage::StorageError;

    #[test]
    fn command_error_serializes_as_contract() {
        let error = CommandError::new("validation_error", "bad input", true);
        let value = serde_json::to_value(error).expect("command error should serialize");

        assert_eq!(value["code"], "validation_error");
        assert_eq!(value["message"], "bad input");
        assert_eq!(value["recoverable"], true);
    }

    #[test]
    fn storage_error_converts_to_command_error() {
        let storage_error = StorageError {
            code: "store_unavailable",
            message: "store failed".to_string(),
            recoverable: true,
        };
        let command_error = CommandError::from(storage_error);

        assert_eq!(command_error.code, "store_unavailable");
        assert_eq!(command_error.message, "store failed");
        assert!(command_error.recoverable);
    }
}
