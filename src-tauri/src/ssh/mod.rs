use crate::ws::{DataPlaneClientMessage, DataPlaneServerMessage, DataPlaneSession, StreamServer};
use russh::client;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, Disconnect};
use russh_sftp::client::SftpSession;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

const DEFAULT_SSH_PORT: u16 = 22;
const MIN_TERMINAL_COLS: u16 = 20;
const MIN_TERMINAL_ROWS: u16 = 5;
const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectSshRequest {
    pub profile_id: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub credential_ref: Option<String>,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub passphrase: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectSshResponse {
    pub session_id: String,
    pub ws_url: String,
    pub auth_token: String,
}

impl From<DataPlaneSession> for ConnectSshResponse {
    fn from(session: DataPlaneSession) -> Self {
        Self {
            session_id: session.session_id,
            ws_url: session.ws_url,
            auth_token: session.auth_token,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResizeRequest {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub session_id: String,
    pub state: SessionLifecycle,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionLifecycle {
    Connecting,
    Connected,
    Closing,
    Closed,
    Failed,
}

impl fmt::Display for SessionLifecycle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Closing => "closing",
            Self::Closed => "closed",
            Self::Failed => "failed",
        };
        formatter.write_str(state)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl SshError {
    fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }

    fn validation(message: impl Into<String>) -> Self {
        Self::new("validation_error", message, true)
    }

    fn invalid_size(message: impl Into<String>) -> Self {
        Self::new("invalid_size", message, true)
    }

    fn not_found(session_id: &str) -> Self {
        Self::new(
            "not_found",
            format!("session {session_id} is not active"),
            true,
        )
    }

    fn auth_failed() -> Self {
        Self::new("auth_failed", "SSH authentication failed", true)
    }

    fn network(message: impl Into<String>) -> Self {
        Self::new("network_error", message, true)
    }

    fn vault_locked() -> Self {
        Self::new(
            "vault_locked",
            "stored SSH credentials are not available in the unlocked vault yet",
            true,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct SshRegistry {
    sessions: Arc<Mutex<HashMap<String, SshSessionEntry>>>,
}

#[derive(Debug, Clone)]
struct SshSessionEntry {
    state: SessionLifecycle,
    control_tx: mpsc::UnboundedSender<SshControl>,
    target: NormalizedConnectRequest,
}

#[derive(Debug)]
enum SshControl {
    Resize { cols: u16, rows: u16 },
    Disconnect,
}

#[derive(Debug, Clone)]
struct NormalizedConnectRequest {
    host: String,
    port: u16,
    username: String,
    auth: SshAuth,
    cols: u16,
    rows: u16,
}

#[derive(Debug, Clone)]
enum SshAuth {
    Password(String),
    PrivateKey {
        path: String,
        passphrase: Option<String>,
    },
}

struct AcceptAnyServerKey;

impl client::Handler for AcceptAnyServerKey {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl SshRegistry {
    pub async fn active_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    pub async fn connect(
        &self,
        stream_server: &StreamServer,
        request: ConnectSshRequest,
    ) -> Result<ConnectSshResponse, SshError> {
        let target = normalize_connect_request(request)?;
        let session_id = Uuid::new_v4().to_string();
        let mut ssh_handle = connect_client(&target).await?;
        authenticate_client(&mut ssh_handle, &target).await?;
        let channel = ssh_handle
            .channel_open_session()
            .await
            .map_err(map_russh_error)?;
        channel
            .request_pty(
                false,
                "xterm-256color",
                target.cols.into(),
                target.rows.into(),
                0,
                0,
                &[],
            )
            .await
            .map_err(map_russh_error)?;
        channel
            .request_shell(false)
            .await
            .map_err(map_russh_error)?;

        let (data_plane_session, client_rx, server_tx) = stream_server
            .register_session_with_id(session_id.clone())
            .await;
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        self.insert_session(
            &session_id,
            SessionLifecycle::Connected,
            control_tx,
            target.clone(),
        )
        .await;

        let registry = self.clone();
        let task_session_id = session_id.clone();
        tokio::spawn(async move {
            bridge_session(
                task_session_id,
                registry,
                ssh_handle,
                channel,
                client_rx,
                server_tx,
                control_rx,
            )
            .await;
        });

        Ok(data_plane_session.into())
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<SessionStatus, SshError> {
        let Some(entry) = self.set_state(session_id, SessionLifecycle::Closing).await else {
            return Err(SshError::not_found(session_id));
        };

        entry
            .control_tx
            .send(SshControl::Disconnect)
            .map_err(|_| SshError::not_found(session_id))?;

        Ok(SessionStatus {
            session_id: session_id.to_string(),
            state: SessionLifecycle::Closing,
        })
    }

    pub async fn resize(&self, request: SessionResizeRequest) -> Result<SessionStatus, SshError> {
        validate_terminal_size(request.cols, request.rows)?;
        let Some(entry) = self.sessions.lock().await.get(&request.session_id).cloned() else {
            return Err(SshError::not_found(&request.session_id));
        };

        entry
            .control_tx
            .send(SshControl::Resize {
                cols: request.cols,
                rows: request.rows,
            })
            .map_err(|_| SshError::not_found(&request.session_id))?;

        Ok(SessionStatus {
            session_id: request.session_id,
            state: entry.state,
        })
    }

    pub async fn open_sftp(&self, session_id: &str) -> Result<SftpSession, SshError> {
        let Some(entry) = self.sessions.lock().await.get(session_id).cloned() else {
            return Err(SshError::not_found(session_id));
        };
        if entry.state != SessionLifecycle::Connected {
            return Err(SshError::new(
                "session_unavailable",
                "SSH session is not connected",
                true,
            ));
        }

        let mut ssh_handle = connect_client(&entry.target).await?;
        authenticate_client(&mut ssh_handle, &entry.target).await?;
        let channel = ssh_handle
            .channel_open_session()
            .await
            .map_err(map_russh_error)?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(map_russh_error)?;

        SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| SshError::new("sftp_error", error.to_string(), true))
    }

    pub async fn open_direct_tcpip(
        &self,
        session_id: &str,
        host_to_connect: &str,
        port_to_connect: u16,
        originator_address: &str,
        originator_port: u16,
    ) -> Result<russh::Channel<client::Msg>, SshError> {
        let Some(entry) = self.sessions.lock().await.get(session_id).cloned() else {
            return Err(SshError::not_found(session_id));
        };
        if entry.state != SessionLifecycle::Connected {
            return Err(SshError::new(
                "session_unavailable",
                "SSH session is not connected",
                true,
            ));
        }

        let mut ssh_handle = connect_client(&entry.target).await?;
        authenticate_client(&mut ssh_handle, &entry.target).await?;
        ssh_handle
            .channel_open_direct_tcpip(
                host_to_connect,
                u32::from(port_to_connect),
                originator_address,
                u32::from(originator_port),
            )
            .await
            .map_err(map_russh_error)
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }

    async fn insert_session(
        &self,
        session_id: &str,
        state: SessionLifecycle,
        control_tx: mpsc::UnboundedSender<SshControl>,
        target: NormalizedConnectRequest,
    ) {
        self.sessions.lock().await.insert(
            session_id.to_string(),
            SshSessionEntry {
                state,
                control_tx,
                target,
            },
        );
    }

    async fn set_state(
        &self,
        session_id: &str,
        state: SessionLifecycle,
    ) -> Option<SshSessionEntry> {
        let mut sessions = self.sessions.lock().await;
        let entry = sessions.get_mut(session_id)?;
        entry.state = state;
        Some(entry.clone())
    }

    async fn remove_session(&self, session_id: &str) {
        self.sessions.lock().await.remove(session_id);
    }
}

async fn connect_client(
    target: &NormalizedConnectRequest,
) -> Result<client::Handle<AcceptAnyServerKey>, SshError> {
    let config = client::Config {
        inactivity_timeout: None,
        ..Default::default()
    };
    let address = format!("{}:{}", target.host, target.port);

    client::connect(Arc::new(config), address, AcceptAnyServerKey)
        .await
        .map_err(map_russh_error)
}

async fn authenticate_client(
    ssh_handle: &mut client::Handle<AcceptAnyServerKey>,
    target: &NormalizedConnectRequest,
) -> Result<(), SshError> {
    let authenticated = match &target.auth {
        SshAuth::Password(password) => ssh_handle
            .authenticate_password(&target.username, password)
            .await
            .map_err(map_russh_error)?,
        SshAuth::PrivateKey { path, passphrase } => {
            let private_key = load_secret_key(path, passphrase.as_deref()).map_err(|_| {
                SshError::new("auth_failed", "SSH private key could not be loaded", true)
            })?;
            let key = PrivateKeyWithHashAlg::new(
                Arc::new(private_key),
                ssh_handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(map_russh_error)?
                    .flatten(),
            );
            ssh_handle
                .authenticate_publickey(&target.username, key)
                .await
                .map_err(map_russh_error)?
        }
    };

    if authenticated.success() {
        Ok(())
    } else {
        Err(SshError::auth_failed())
    }
}

async fn bridge_session(
    session_id: String,
    registry: SshRegistry,
    ssh_handle: client::Handle<AcceptAnyServerKey>,
    mut channel: russh::Channel<client::Msg>,
    mut client_rx: mpsc::UnboundedReceiver<DataPlaneClientMessage>,
    server_tx: mpsc::UnboundedSender<DataPlaneServerMessage>,
    mut control_rx: mpsc::UnboundedReceiver<SshControl>,
) {
    let _ = server_tx.send(DataPlaneServerMessage::Status {
        state: SessionLifecycle::Connected.to_string(),
    });

    loop {
        tokio::select! {
            Some(control) = control_rx.recv() => {
                match control {
                    SshControl::Resize { cols, rows } => {
                        if let Err(error) = channel.window_change(cols.into(), rows.into(), 0, 0).await {
                            let _ = server_tx.send(DataPlaneServerMessage::Error {
                                code: "network_error".to_string(),
                                message: error.to_string(),
                                recoverable: true,
                            });
                        }
                    }
                    SshControl::Disconnect => break,
                }
            }
            Some(message) = client_rx.recv() => {
                match message {
                    DataPlaneClientMessage::Binary(bytes) => {
                        if let Err(error) = channel.data_bytes(bytes).await {
                            let _ = server_tx.send(DataPlaneServerMessage::Error {
                                code: "network_error".to_string(),
                                message: error.to_string(),
                                recoverable: true,
                            });
                            break;
                        }
                    }
                    DataPlaneClientMessage::Resize { cols, rows } => {
                        if validate_terminal_size(cols, rows).is_ok() {
                            let _ = channel.window_change(cols.into(), rows.into(), 0, 0).await;
                        }
                    }
                    DataPlaneClientMessage::Close => break,
                }
            }
            Some(message) = channel.wait() => {
                match message {
                    ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                        let _ = server_tx.send(DataPlaneServerMessage::Binary(data.to_vec()));
                    }
                    ChannelMsg::ExitStatus { .. } | ChannelMsg::Eof | ChannelMsg::Close => break,
                    _ => {}
                }
            }
            else => break,
        }
    }

    let _ = channel.close().await;
    let _ = ssh_handle
        .disconnect(Disconnect::ByApplication, "session closed", "en")
        .await;
    registry.remove_session(&session_id).await;
    let _ = server_tx.send(DataPlaneServerMessage::Status {
        state: SessionLifecycle::Closed.to_string(),
    });
}

fn normalize_connect_request(
    request: ConnectSshRequest,
) -> Result<NormalizedConnectRequest, SshError> {
    let _profile_id = request.profile_id;
    let host = required_non_empty(request.host, "host")?;
    let username = required_non_empty(request.username, "username")?;
    let port = request.port.unwrap_or(DEFAULT_SSH_PORT);
    validate_terminal_size(request.cols, request.rows)?;

    let auth = match (request.password, request.private_key_path) {
        (Some(password), _) if !password.is_empty() => SshAuth::Password(password),
        (_, Some(path)) if !path.trim().is_empty() => SshAuth::PrivateKey {
            path,
            passphrase: request.passphrase,
        },
        _ if request.credential_ref.is_some() => return Err(SshError::vault_locked()),
        _ => {
            return Err(SshError::validation(
                "password or privateKeyPath is required for SSH authentication",
            ))
        }
    };

    Ok(NormalizedConnectRequest {
        host,
        port,
        username,
        auth,
        cols: request.cols,
        rows: request.rows,
    })
}

fn required_non_empty(value: Option<String>, field: &str) -> Result<String, SshError> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SshError::validation(format!("{field} is required")))
}

fn validate_terminal_size(cols: u16, rows: u16) -> Result<(), SshError> {
    if !(MIN_TERMINAL_COLS..=MAX_TERMINAL_COLS).contains(&cols) {
        return Err(SshError::invalid_size(format!(
            "cols must be between {MIN_TERMINAL_COLS} and {MAX_TERMINAL_COLS}"
        )));
    }
    if !(MIN_TERMINAL_ROWS..=MAX_TERMINAL_ROWS).contains(&rows) {
        return Err(SshError::invalid_size(format!(
            "rows must be between {MIN_TERMINAL_ROWS} and {MAX_TERMINAL_ROWS}"
        )));
    }
    Ok(())
}

fn map_russh_error(error: russh::Error) -> SshError {
    match error {
        russh::Error::ConnectionTimeout | russh::Error::KeepaliveTimeout => {
            SshError::new("host_unreachable", "SSH host is unreachable", true)
        }
        russh::Error::NotAuthenticated | russh::Error::UnsupportedAuthMethod => {
            SshError::auth_failed()
        }
        _ => SshError::network(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_connect_request, validate_terminal_size, ConnectSshRequest, SshAuth};

    fn valid_request() -> ConnectSshRequest {
        ConnectSshRequest {
            profile_id: None,
            host: Some("example.com".to_string()),
            port: Some(22),
            username: Some("alice".to_string()),
            credential_ref: None,
            password: Some("secret".to_string()),
            private_key_path: None,
            passphrase: None,
            cols: 80,
            rows: 24,
        }
    }

    #[test]
    fn normalizes_valid_password_request() {
        let target =
            normalize_connect_request(valid_request()).expect("valid request should normalize");

        assert_eq!(target.host, "example.com");
        assert_eq!(target.port, 22);
        assert_eq!(target.username, "alice");
        assert!(matches!(target.auth, SshAuth::Password(_)));
    }

    #[test]
    fn rejects_missing_auth_material() {
        let mut request = valid_request();
        request.password = None;

        let error = normalize_connect_request(request).expect_err("auth should be required");

        assert_eq!(error.code, "validation_error");
    }

    #[test]
    fn rejects_invalid_terminal_size() {
        let error = validate_terminal_size(10, 24).expect_err("cols should be bounded");

        assert_eq!(error.code, "invalid_size");
    }
}
