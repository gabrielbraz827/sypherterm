use futures_util::{SinkExt, StreamExt};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const TOKEN_BYTES: usize = 32;
const SESSION_TTL: Duration = Duration::from_secs(60);
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPlaneSession {
    pub session_id: String,
    pub ws_url: String,
    pub auth_token: String,
    pub expires_at: String,
}

#[derive(Debug)]
pub enum DataPlaneClientMessage {
    Binary(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Close,
}

#[derive(Debug)]
pub enum DataPlaneServerMessage {
    Binary(Vec<u8>),
    Status {
        state: String,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

#[derive(Debug)]
pub enum StreamServerError {
    Bind(std::io::Error),
    LocalAddress(std::io::Error),
}

impl fmt::Display for StreamServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(error) => write!(formatter, "failed to bind Data Plane: {error}"),
            Self::LocalAddress(error) => {
                write!(
                    formatter,
                    "failed to read Data Plane local address: {error}"
                )
            }
        }
    }
}

impl std::error::Error for StreamServerError {}

#[derive(Debug, Clone)]
pub struct StreamServer {
    addr: SocketAddr,
    pending: Arc<Mutex<HashMap<String, PendingSession>>>,
}

#[derive(Debug)]
struct PendingSession {
    session_id: String,
    expires_at: Instant,
    client_tx: mpsc::UnboundedSender<DataPlaneClientMessage>,
    server_rx: mpsc::UnboundedReceiver<DataPlaneServerMessage>,
}

#[derive(Debug, Deserialize)]
struct AuthFrame {
    #[serde(default)]
    event: Option<String>,
    #[serde(default, rename = "type")]
    frame_type: Option<String>,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResizeFrame {
    cols: u16,
    rows: u16,
}

impl StreamServer {
    pub async fn start() -> Result<Self, StreamServerError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(StreamServerError::Bind)?;
        let addr = listener
            .local_addr()
            .map_err(StreamServerError::LocalAddress)?;
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let server = Self {
            addr,
            pending: Arc::clone(&pending),
        };

        tokio::spawn(async move {
            accept_loop(listener, pending).await;
        });

        Ok(server)
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    pub async fn register_session(
        &self,
    ) -> (
        DataPlaneSession,
        mpsc::UnboundedReceiver<DataPlaneClientMessage>,
        mpsc::UnboundedSender<DataPlaneServerMessage>,
    ) {
        let session_id = Uuid::new_v4().to_string();
        self.register_session_with_id(session_id).await
    }

    pub async fn register_session_with_id(
        &self,
        session_id: String,
    ) -> (
        DataPlaneSession,
        mpsc::UnboundedReceiver<DataPlaneClientMessage>,
        mpsc::UnboundedSender<DataPlaneServerMessage>,
    ) {
        let auth_token = generate_token();
        let expires_at = Instant::now() + SESSION_TTL;
        let (client_tx, client_rx) = mpsc::unbounded_channel();
        let (server_tx, server_rx) = mpsc::unbounded_channel();

        let mut pending = self.pending.lock().await;
        cleanup_expired(&mut pending);
        pending.insert(
            auth_token.clone(),
            PendingSession {
                session_id: session_id.clone(),
                expires_at,
                client_tx,
                server_rx,
            },
        );
        drop(pending);

        (
            DataPlaneSession {
                session_id,
                ws_url: self.ws_url(),
                auth_token,
                expires_at: unix_timestamp_after(SESSION_TTL),
            },
            client_rx,
            server_tx,
        )
    }

    async fn consume_token(&self, token: &str) -> Option<PendingSession> {
        let mut pending = self.pending.lock().await;
        cleanup_expired(&mut pending);
        pending.remove(token)
    }

    #[cfg(test)]
    async fn pending_count(&self) -> usize {
        let pending = self.pending.lock().await;
        pending.len()
    }
}

async fn accept_loop(listener: TcpListener, pending: Arc<Mutex<HashMap<String, PendingSession>>>) {
    while let Ok((stream, _)) = listener.accept().await {
        let server = StreamServer {
            addr: listener
                .local_addr()
                .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 0))),
            pending: Arc::clone(&pending),
        };

        tokio::spawn(async move {
            if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                handle_connection(ws_stream, server).await;
            }
        });
    }
}

async fn handle_connection(
    mut ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    server: StreamServer,
) {
    let auth_message = match timeout(AUTH_TIMEOUT, ws_stream.next()).await {
        Ok(Some(Ok(message))) => message,
        _ => {
            let _ = send_error(&mut ws_stream, "auth_timeout", "authentication timed out").await;
            return;
        }
    };

    let Some(token) = parse_auth_token(auth_message) else {
        let _ = send_error(
            &mut ws_stream,
            "auth_required",
            "first frame must authenticate",
        )
        .await;
        return;
    };

    let Some(mut pending_session) = server.consume_token(&token).await else {
        let _ = send_error(&mut ws_stream, "invalid_token", "invalid or expired token").await;
        return;
    };

    let _ = send_json(
        &mut ws_stream,
        serde_json::json!({
            "event": "status",
            "state": "connected",
            "sessionId": pending_session.session_id,
        }),
    )
    .await;
    loop {
        tokio::select! {
            Some(server_message) = pending_session.server_rx.recv() => {
                if send_server_message(&mut ws_stream, server_message).await.is_err() {
                    break;
                }
            }
            Some(message) = ws_stream.next() => {
                match message {
                    Ok(Message::Text(text)) => {
                        if handle_text_message(&mut ws_stream, &pending_session.client_tx, &text).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Binary(bytes)) => {
                        let _ = pending_session.client_tx.send(DataPlaneClientMessage::Binary(bytes.to_vec()));
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
            else => break,
        }
    }

    let _ = pending_session
        .client_tx
        .send(DataPlaneClientMessage::Close);
}

fn parse_auth_token(message: Message) -> Option<String> {
    let Message::Text(text) = message else {
        return None;
    };

    let frame = serde_json::from_str::<AuthFrame>(&text).ok()?;
    let auth_event =
        frame.event.as_deref() == Some("auth") || frame.frame_type.as_deref() == Some("auth");
    if !auth_event && (frame.event.is_some() || frame.frame_type.is_some()) {
        return None;
    }

    Some(frame.token)
}

async fn send_error(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    code: &str,
    message: &str,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    send_json(
        ws_stream,
        serde_json::json!({
            "event": "error",
            "code": code,
            "message": message,
            "recoverable": true,
        }),
    )
    .await
}

async fn send_server_message(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    message: DataPlaneServerMessage,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    match message {
        DataPlaneServerMessage::Binary(bytes) => {
            ws_stream.send(Message::Binary(bytes.into())).await
        }
        DataPlaneServerMessage::Status { state } => {
            send_json(
                ws_stream,
                serde_json::json!({
                    "event": "status",
                    "state": state,
                }),
            )
            .await
        }
        DataPlaneServerMessage::Error {
            code,
            message,
            recoverable,
        } => {
            send_json(
                ws_stream,
                serde_json::json!({
                    "event": "error",
                    "code": code,
                    "message": message,
                    "recoverable": recoverable,
                }),
            )
            .await
        }
    }
}

async fn handle_text_message(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    client_tx: &mpsc::UnboundedSender<DataPlaneClientMessage>,
    text: &str,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Ok(());
    };

    match value.get("event").and_then(|event| event.as_str()) {
        Some("heartbeat") => {
            send_json(
                ws_stream,
                serde_json::json!({
                    "event": "heartbeat",
                    "ts": timestamp(),
                }),
            )
            .await
        }
        Some("resize") => {
            if let Ok(resize) = serde_json::from_value::<ResizeFrame>(value) {
                let _ = client_tx.send(DataPlaneClientMessage::Resize {
                    cols: resize.cols,
                    rows: resize.rows,
                });
            }
            Ok(())
        }
        Some("input") => {
            if let Some(data) = value.get("data").and_then(|data| data.as_str()) {
                let _ = client_tx.send(DataPlaneClientMessage::Binary(data.as_bytes().to_vec()));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn send_json(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    value: serde_json::Value,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    ws_stream
        .send(Message::Text(value.to_string().into()))
        .await
}

fn cleanup_expired(pending: &mut HashMap<String, PendingSession>) {
    let now = Instant::now();
    pending.retain(|_, session| session.expires_at > now);
}

fn generate_token() -> String {
    let mut bytes = [0_u8; TOKEN_BYTES];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn unix_timestamp_after(duration: Duration) -> String {
    SystemTime::now()
        .checked_add(duration)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(timestamp)
}

#[cfg(test)]
mod tests {
    use super::{parse_auth_token, StreamServer};
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn parses_auth_frame_with_token_only() {
        let token = parse_auth_token(Message::Text(r#"{"token":"abc"}"#.into()));

        assert_eq!(token.as_deref(), Some("abc"));
    }

    #[test]
    fn rejects_non_auth_typed_frame() {
        let token = parse_auth_token(Message::Text(r#"{"event":"input","token":"abc"}"#.into()));

        assert!(token.is_none());
    }

    #[tokio::test]
    async fn registers_unique_pending_session() {
        let server = StreamServer::start()
            .await
            .expect("stream server should start");
        let (first, _, _) = server.register_session().await;
        let (second, _, _) = server.register_session().await;

        assert_ne!(first.session_id, second.session_id);
        assert_ne!(first.auth_token, second.auth_token);
        assert_eq!(server.pending_count().await, 2);
    }

    #[tokio::test]
    async fn websocket_requires_valid_first_auth_frame() {
        let server = StreamServer::start()
            .await
            .expect("stream server should start");
        let (session, _, _) = server.register_session().await;
        let (mut socket, _) = connect_async(&session.ws_url)
            .await
            .expect("websocket should connect");

        socket
            .send(Message::Text(
                format!(r#"{{"event":"auth","token":"{}"}}"#, session.auth_token).into(),
            ))
            .await
            .expect("auth frame should send");
        let response = socket
            .next()
            .await
            .expect("status frame should arrive")
            .expect("status frame should be valid");

        assert!(response
            .into_text()
            .expect("status should be text")
            .contains(r#""state":"connected""#));
        assert_eq!(server.pending_count().await, 0);
    }

    #[tokio::test]
    async fn websocket_rejects_invalid_token() {
        let server = StreamServer::start()
            .await
            .expect("stream server should start");
        let (mut socket, _) = connect_async(server.ws_url())
            .await
            .expect("websocket should connect");

        socket
            .send(Message::Text(
                r#"{"event":"auth","token":"not-valid"}"#.into(),
            ))
            .await
            .expect("auth frame should send");
        let response = socket
            .next()
            .await
            .expect("error frame should arrive")
            .expect("error frame should be valid");

        assert!(response
            .into_text()
            .expect("error should be text")
            .contains(r#""code":"invalid_token""#));
    }
}
