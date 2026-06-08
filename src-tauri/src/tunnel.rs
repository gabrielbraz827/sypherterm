use crate::ssh::SshRegistry;
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{self, Duration};
use uuid::Uuid;

const TUNNEL_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelRequest {
    pub session_id: Option<String>,
    pub profile_id: Option<String>,
    pub mode: TunnelMode,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
    pub label: Option<String>,
    pub allow_external_bind: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TunnelMode {
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub tunnel_id: String,
    pub session_id: String,
    pub mode: TunnelMode,
    pub state: TunnelState,
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TunnelState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TunnelError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone)]
struct TunnelEntry {
    status: TunnelStatus,
    stop_tx: mpsc::UnboundedSender<()>,
}

#[derive(Debug, Clone, Default)]
pub struct TunnelRegistry {
    tunnels: Arc<Mutex<HashMap<String, TunnelEntry>>>,
}

impl TunnelRegistry {
    pub async fn start(
        &self,
        ssh: &SshRegistry,
        request: TunnelRequest,
    ) -> Result<TunnelStatus, TunnelError> {
        if request.mode != TunnelMode::Local {
            return Err(TunnelError::new(
                "unsupported_mode",
                "only local forwarding is implemented in this increment",
                true,
            ));
        }

        if request.profile_id.is_some() {
            return Err(TunnelError::new(
                "unsupported_mode",
                "starting tunnels from saved profiles is not implemented yet",
                true,
            ));
        }

        let session_id = required(request.session_id, "sessionId")?;
        if !ssh.has_session(&session_id).await {
            return Err(TunnelError::new(
                "not_found",
                "SSH session is not active",
                true,
            ));
        }

        let bind_host = normalize_bind_host(request.bind_host)?;
        validate_external_bind(&bind_host, request.allow_external_bind.unwrap_or(false))?;
        let target_host = required(request.target_host, "targetHost")?;
        let target_port = request
            .target_port
            .ok_or_else(|| TunnelError::new("invalid_bind", "targetPort is required", true))?;
        let listener = TcpListener::bind((bind_host.as_str(), request.bind_port))
            .await
            .map_err(TunnelError::from_bind_error)?;
        let local_addr = listener
            .local_addr()
            .map_err(TunnelError::from_bind_error)?;
        let tunnel_id = Uuid::new_v4().to_string();
        let (stop_tx, stop_rx) = mpsc::unbounded_channel();
        let status = TunnelStatus {
            tunnel_id: tunnel_id.clone(),
            session_id: session_id.clone(),
            mode: TunnelMode::Local,
            state: TunnelState::Running,
            bind_host: local_addr.ip().to_string(),
            bind_port: local_addr.port(),
            target_host: Some(target_host.clone()),
            target_port: Some(target_port),
            label: request.label,
            started_at: Some(timestamp()),
            last_error: None,
        };

        self.tunnels.lock().await.insert(
            tunnel_id.clone(),
            TunnelEntry {
                status: status.clone(),
                stop_tx,
            },
        );

        let registry = self.clone();
        let ssh = ssh.clone();
        tokio::spawn(async move {
            run_local_tunnel(
                registry,
                ssh,
                tunnel_id,
                session_id,
                target_host,
                target_port,
                listener,
                stop_rx,
            )
            .await;
        });

        Ok(status)
    }

    pub async fn stop(&self, tunnel_id: String) -> Result<TunnelStatus, TunnelError> {
        let entry = {
            let mut tunnels = self.tunnels.lock().await;
            let Some(entry) = tunnels.get_mut(&tunnel_id) else {
                return Err(TunnelError::new("not_found", "tunnel is not active", true));
            };
            entry.status.state = TunnelState::Stopping;
            entry.clone()
        };

        let _ = entry.stop_tx.send(());
        self.mark_stopped(&tunnel_id, None).await
    }

    pub async fn list(&self) -> Vec<TunnelStatus> {
        self.tunnels
            .lock()
            .await
            .values()
            .map(|entry| entry.status.clone())
            .collect()
    }

    pub async fn list_for_session(
        &self,
        session_id: String,
    ) -> Result<Vec<TunnelStatus>, TunnelError> {
        let tunnels = self
            .tunnels
            .lock()
            .await
            .values()
            .filter(|entry| entry.status.session_id == session_id)
            .map(|entry| entry.status.clone())
            .collect::<Vec<_>>();

        Ok(tunnels)
    }

    pub async fn stop_session_tunnels(&self, session_id: &str) {
        let tunnel_ids = self
            .tunnels
            .lock()
            .await
            .values()
            .filter(|entry| entry.status.session_id == session_id)
            .map(|entry| entry.status.tunnel_id.clone())
            .collect::<Vec<_>>();

        for tunnel_id in tunnel_ids {
            let _ = self.stop(tunnel_id).await;
        }
    }

    async fn mark_stopped(
        &self,
        tunnel_id: &str,
        last_error: Option<String>,
    ) -> Result<TunnelStatus, TunnelError> {
        let mut tunnels = self.tunnels.lock().await;
        let Some(entry) = tunnels.get_mut(tunnel_id) else {
            return Err(TunnelError::new("not_found", "tunnel is not active", true));
        };

        entry.status.state = if last_error.is_some() {
            TunnelState::Failed
        } else {
            TunnelState::Stopped
        };
        entry.status.last_error = last_error;
        Ok(entry.status.clone())
    }
}

async fn run_local_tunnel(
    registry: TunnelRegistry,
    ssh: SshRegistry,
    tunnel_id: String,
    session_id: String,
    target_host: String,
    target_port: u16,
    listener: TcpListener,
    mut stop_rx: mpsc::UnboundedReceiver<()>,
) {
    let mut session_check = time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            _ = session_check.tick() => {
                if !ssh.has_session(&session_id).await {
                    break;
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer_addr)) => {
                        let ssh = ssh.clone();
                        let session_id = session_id.clone();
                        let target_host = target_host.clone();
                        tokio::spawn(async move {
                            let _ = proxy_local_connection(
                                ssh,
                                session_id,
                                target_host,
                                target_port,
                                stream,
                                peer_addr,
                            )
                            .await;
                        });
                    }
                    Err(error) => {
                        let _ = registry
                            .mark_stopped(&tunnel_id, Some(error.to_string()))
                            .await;
                        return;
                    }
                }
            }
        }
    }

    let _ = registry.mark_stopped(&tunnel_id, None).await;
}

async fn proxy_local_connection(
    ssh: SshRegistry,
    session_id: String,
    target_host: String,
    target_port: u16,
    local_stream: TcpStream,
    peer_addr: SocketAddr,
) -> Result<(), TunnelError> {
    let mut channel = ssh
        .open_direct_tcpip(
            &session_id,
            &target_host,
            target_port,
            &peer_addr.ip().to_string(),
            peer_addr.port(),
        )
        .await
        .map_err(TunnelError::from_ssh_error)?;
    let (mut local_reader, mut local_writer) = local_stream.into_split();
    let mut buffer = vec![0; TUNNEL_BUFFER_SIZE];

    loop {
        tokio::select! {
            read = local_reader.read(&mut buffer) => {
                let bytes_read = read.map_err(TunnelError::io)?;
                if bytes_read == 0 {
                    let _ = channel.eof().await;
                    break;
                }
                channel
                    .data_bytes(buffer[..bytes_read].to_vec())
                    .await
                    .map_err(TunnelError::from_russh_error)?;
            }
            message = channel.wait() => {
                let Some(message) = message else {
                    break;
                };
                match message {
                    ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                        local_writer
                            .write_all(&data)
                            .await
                            .map_err(TunnelError::io)?;
                    }
                    ChannelMsg::Eof | ChannelMsg::Close | ChannelMsg::ExitStatus { .. } => break,
                    _ => {}
                }
            }
        }
    }

    let _ = channel.close().await;
    Ok(())
}

impl TunnelError {
    fn new(code: &'static str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }

    fn from_bind_error(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::AddrInUse => {
                Self::new("port_in_use", "local bind port is already in use", true)
            }
            std::io::ErrorKind::PermissionDenied => Self::new(
                "permission_denied",
                "permission denied while binding local port",
                true,
            ),
            _ => Self::new("invalid_bind", error.to_string(), true),
        }
    }

    fn from_ssh_error(error: crate::ssh::SshError) -> Self {
        Self::new("ssh_error", error.message, error.recoverable)
    }

    fn from_russh_error(error: russh::Error) -> Self {
        Self::new("ssh_error", error.to_string(), true)
    }

    fn io(error: std::io::Error) -> Self {
        Self::new("io_error", error.to_string(), true)
    }
}

impl fmt::Display for TunnelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

fn required(value: Option<String>, field: &str) -> Result<String, TunnelError> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| TunnelError::new("invalid_bind", format!("{field} is required"), true))
}

fn normalize_bind_host(bind_host: String) -> Result<String, TunnelError> {
    let bind_host = bind_host.trim();
    if bind_host.is_empty() {
        return Ok("127.0.0.1".to_string());
    }

    bind_host
        .parse::<IpAddr>()
        .map(|address| address.to_string())
        .map_err(|_| TunnelError::new("invalid_bind", "bindHost must be an IP address", true))
}

fn validate_external_bind(bind_host: &str, allow_external_bind: bool) -> Result<(), TunnelError> {
    let address = bind_host
        .parse::<IpAddr>()
        .map_err(|_| TunnelError::new("invalid_bind", "bindHost must be an IP address", true))?;
    if !address.is_loopback() && !allow_external_bind {
        return Err(TunnelError::new(
            "invalid_bind",
            "external bind requires explicit confirmation",
            true,
        ));
    }

    Ok(())
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::{normalize_bind_host, required, validate_external_bind};

    #[test]
    fn empty_bind_host_defaults_to_loopback() {
        assert_eq!(
            normalize_bind_host(" ".to_string()).expect("default bind host"),
            "127.0.0.1"
        );
    }

    #[test]
    fn external_bind_requires_explicit_confirmation() {
        let error = validate_external_bind("0.0.0.0", false).expect_err("external bind blocked");

        assert_eq!(error.code, "invalid_bind");
    }

    #[test]
    fn external_bind_accepts_confirmation() {
        validate_external_bind("0.0.0.0", true).expect("external bind confirmed");
    }

    #[test]
    fn required_rejects_empty_values() {
        let error = required(Some(" ".to_string()), "sessionId").expect_err("empty rejected");

        assert_eq!(error.code, "invalid_bind");
    }
}
