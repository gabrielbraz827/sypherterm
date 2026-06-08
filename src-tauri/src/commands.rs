use tauri::{AppHandle, State};

use crate::crypto::{
    change_master_password as change_stored_master_password, create_vault as create_stored_vault,
    lock_vault as lock_stored_vault, unlock_vault as unlock_stored_vault,
    vault_status as stored_vault_status, ChangeMasterPasswordRequest, CreateVaultRequest,
    UnlockVaultRequest, VaultError, VaultStatus,
};
use crate::sftp::{
    RemoteDirEntry, SftpCancelRequest, SftpDeleteRequest, SftpError, SftpPathRequest, SftpRegistry,
    SftpRenameRequest, SftpTransferRequest, TransferJob,
};
use crate::snippets::{
    delete_snippet as delete_stored_snippet, get_snippet as get_stored_snippet,
    list_snippets as list_stored_snippets, save_snippet as save_stored_snippet, Snippet,
    SnippetDraft, SnippetError, SnippetFilters, SnippetSummary,
};
use crate::ssh::{
    ConnectSshRequest, ConnectSshResponse, SessionResizeRequest, SessionStatus, SshError,
    SshRegistry,
};
use crate::state::{AppState, AppStatus};
use crate::storage::{
    delete_profile as delete_stored_profile, duplicate_profile as duplicate_stored_profile,
    get_preferences as get_stored_preferences, list_profiles as list_stored_profiles,
    mark_profile_used, save_preferences as save_stored_preferences,
    save_profile as save_stored_profile, ConnectionProfile, ConnectionProfileDraft,
    ConnectionProfileSummary, DeleteResult, ProfileListFilters, StorageError, UserPreferences,
};
use crate::sync::{
    list_sync_versions as list_provider_sync_versions,
    test_sync_provider as test_configured_sync_provider, trigger_sync as trigger_configured_sync,
    SyncError, SyncJobStatus, SyncProviderConfig, SyncProviderStatus, SyncRequest, SyncVersion,
};
use crate::tunnel::{TunnelError, TunnelRegistry, TunnelRequest, TunnelStatus};
use crate::ws::{DataPlaneSession, StreamServer, StreamServerError};
use serde::Serialize;

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

impl From<SyncError> for CommandError {
    fn from(error: SyncError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

impl From<SftpError> for CommandError {
    fn from(error: SftpError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

impl From<TunnelError> for CommandError {
    fn from(error: TunnelError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

impl From<SnippetError> for CommandError {
    fn from(error: SnippetError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
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
pub fn list_profiles(
    app: AppHandle,
    filters: Option<ProfileListFilters>,
) -> Result<Vec<ConnectionProfileSummary>, CommandError> {
    list_stored_profiles(&app, filters).map_err(Into::into)
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
pub fn duplicate_profile(app: AppHandle, id: String) -> Result<ConnectionProfile, CommandError> {
    duplicate_stored_profile(&app, &id).map_err(Into::into)
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
pub fn list_snippets(
    state: State<'_, AppState>,
    filters: Option<SnippetFilters>,
) -> Result<Vec<SnippetSummary>, CommandError> {
    list_stored_snippets(&state, filters).map_err(Into::into)
}

#[tauri::command]
pub fn get_snippet(state: State<'_, AppState>, id: String) -> Result<Snippet, CommandError> {
    get_stored_snippet(&state, id).map_err(Into::into)
}

#[tauri::command]
pub fn save_snippet(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: SnippetDraft,
) -> Result<Snippet, CommandError> {
    save_stored_snippet(&app, &state, draft).map_err(Into::into)
}

#[tauri::command]
pub fn delete_snippet(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<DeleteResult, CommandError> {
    delete_stored_snippet(&app, &state, id).map_err(Into::into)
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
    app: AppHandle,
    server: State<'_, StreamServer>,
    ssh: State<'_, SshRegistry>,
    request: ConnectSshRequest,
) -> Result<ConnectSshResponse, CommandError> {
    let profile_id = request.profile_id.clone();
    let response = ssh.connect(&server, request).await?;
    if let Some(profile_id) = profile_id {
        let _ = mark_profile_used(&app, &profile_id);
    }
    Ok(response)
}

#[tauri::command]
pub async fn disconnect_session(
    ssh: State<'_, SshRegistry>,
    tunnels: State<'_, TunnelRegistry>,
    session_id: String,
) -> Result<SessionStatus, CommandError> {
    tunnels.stop_session_tunnels(&session_id).await;
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
pub async fn sftp_list_dir(
    ssh: State<'_, SshRegistry>,
    sftp: State<'_, SftpRegistry>,
    request: SftpPathRequest,
) -> Result<Vec<RemoteDirEntry>, CommandError> {
    sftp.list_dir(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn sftp_download(
    ssh: State<'_, SshRegistry>,
    sftp: State<'_, SftpRegistry>,
    request: SftpTransferRequest,
) -> Result<TransferJob, CommandError> {
    sftp.download(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn sftp_upload(
    ssh: State<'_, SshRegistry>,
    sftp: State<'_, SftpRegistry>,
    request: SftpTransferRequest,
) -> Result<TransferJob, CommandError> {
    sftp.upload(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn sftp_cancel_transfer(
    sftp: State<'_, SftpRegistry>,
    request: SftpCancelRequest,
) -> Result<TransferJob, CommandError> {
    sftp.cancel_transfer(request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn sftp_mkdir(
    ssh: State<'_, SshRegistry>,
    sftp: State<'_, SftpRegistry>,
    request: SftpPathRequest,
) -> Result<RemoteDirEntry, CommandError> {
    sftp.mkdir(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn sftp_rename(
    ssh: State<'_, SshRegistry>,
    sftp: State<'_, SftpRegistry>,
    request: SftpRenameRequest,
) -> Result<RemoteDirEntry, CommandError> {
    sftp.rename(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn sftp_delete(
    ssh: State<'_, SshRegistry>,
    sftp: State<'_, SftpRegistry>,
    request: SftpDeleteRequest,
) -> Result<RemoteDirEntry, CommandError> {
    sftp.delete(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub fn test_sync_provider(config: SyncProviderConfig) -> Result<SyncProviderStatus, CommandError> {
    test_configured_sync_provider(config).map_err(Into::into)
}

#[tauri::command]
pub fn trigger_cloud_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SyncRequest,
) -> Result<SyncJobStatus, CommandError> {
    trigger_configured_sync(&app, &state, request).map_err(Into::into)
}

#[tauri::command]
pub fn list_sync_versions(config: SyncProviderConfig) -> Result<Vec<SyncVersion>, CommandError> {
    list_provider_sync_versions(config).map_err(Into::into)
}

#[tauri::command]
pub async fn start_tunnel(
    ssh: State<'_, SshRegistry>,
    tunnels: State<'_, TunnelRegistry>,
    request: TunnelRequest,
) -> Result<TunnelStatus, CommandError> {
    tunnels.start(&ssh, request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn stop_tunnel(
    tunnels: State<'_, TunnelRegistry>,
    tunnel_id: String,
) -> Result<TunnelStatus, CommandError> {
    tunnels.stop(tunnel_id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn list_tunnels(
    tunnels: State<'_, TunnelRegistry>,
) -> Result<Vec<TunnelStatus>, CommandError> {
    Ok(tunnels.list().await)
}

#[tauri::command]
pub async fn list_session_tunnels(
    tunnels: State<'_, TunnelRegistry>,
    session_id: String,
) -> Result<Vec<TunnelStatus>, CommandError> {
    tunnels
        .list_for_session(session_id)
        .await
        .map_err(Into::into)
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
