use tauri::{AppHandle, State};

use crate::crypto::{
    change_master_password as change_stored_master_password, create_vault as create_stored_vault,
    lock_vault as lock_stored_vault, unlock_vault as unlock_stored_vault,
    vault_status as stored_vault_status, ChangeMasterPasswordRequest, CreateVaultRequest,
    UnlockVaultRequest, VaultError, VaultStatus,
};
use crate::ssh::{
    ConnectSshRequest, ConnectSshResponse, SessionResizeRequest, SessionStatus, SshError,
    SshRegistry,
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

impl From<SshError> for CommandError {
    fn from(error: SshError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
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
pub async fn get_app_status(
    app: AppHandle,
    state: State<'_, AppState>,
    ssh: State<'_, SshRegistry>,
) -> Result<AppStatus, CommandError> {
    let _ = stored_vault_status(&app, &state)?;
    let mut status = state.status()?;
    status.active_sessions = ssh.active_count().await;
    Ok(status)
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
    let (session, mut client_rx, server_tx) = server.register_session().await;
    tokio::spawn(async move {
        while let Some(message) = client_rx.recv().await {
            match message {
                crate::ws::DataPlaneClientMessage::Binary(bytes) => {
                    let _ = server_tx.send(crate::ws::DataPlaneServerMessage::Binary(bytes));
                }
                crate::ws::DataPlaneClientMessage::Resize { .. } => {}
                crate::ws::DataPlaneClientMessage::Close => break,
            }
        }
    });
    Ok(session)
}

#[tauri::command]
pub async fn connect_ssh(
    server: State<'_, StreamServer>,
    ssh: State<'_, SshRegistry>,
    request: ConnectSshRequest,
) -> Result<ConnectSshResponse, CommandError> {
    ssh.connect(&server, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn disconnect_session(
    ssh: State<'_, SshRegistry>,
    session_id: String,
) -> Result<SessionStatus, CommandError> {
    ssh.disconnect(&session_id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn resize_session(
    ssh: State<'_, SshRegistry>,
    request: SessionResizeRequest,
) -> Result<SessionStatus, CommandError> {
    ssh.resize(request).await.map_err(Into::into)
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
