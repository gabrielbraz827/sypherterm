# 🗺️ Architectural & Stack Specification (SypherTerm)

This document establishes the technical architecture, stack selection, and core system design for **Sypher**. The primary engineering goals are ultra-low resource utilization, memory safety, and high-frequency data streaming without UI freezing.

---

## 1. Tech Stack Selection & Justification

### 🦀 Backend Runtime: Tauri + Rust
* **Decision:** Tauri (Rust) as the application core, managing the native OS windows, file system access, encryption engine, and SSH streaming.
* **Justification:** Eliminates the ~100MB+ memory overhead of bundled Chromium/Node.js runtimes found in Electron. It ensures native compilation across Windows, macOS, and Linux with a minimal binary size (~10-15MB) and native memory safety via Rust.

### ⚡ Frontend Framework: Svelte vs. React
* **Decision:** **Svelte** (via Vite + TailwindCSS).
* **Justification:** 1.  **High-Frequency Streaming:** A terminal emulator is essentially a high-frequency event stream. React’s Virtual DOM requires constant diffing and reconciliation cycles during heavy terminal outputs (e.g., running `htop` or printing large logs), creating massive runtime overhead. Svelte compiles down to precise, surgical, framework-free DOM manipulation, eliminating runtime reconciliation.
    2.  **WebView Performance:** Because Tauri relies on the system's native WebView (Webkit/WebView2), minimizing JavaScript execution overhead inside the webview layer is critical.
    3.  **Developer Experience:** Svelte features a highly intuitive reactive system with less boilerplate, offering a gentler learning curve for developers coming from typed backend languages like Rust.

---

## 2. High-Frequency Data Streaming Architecture

To completely avoid serialization/deserialization bottlenecks across the traditional Tauri IPC (Inter-Process Communication) bridge, Sypher splits data flow into two distinct channels:

1.  **The Control Plane (Tauri IPC Commands):** Used for low-frequency actions such as establishing a connection, modifying configurations, saving snippets, disconnecting, or authenticating.
2.  **The Data Plane (Local WebSocket Server):** The Rust backend boots a lightweight WebSocket server (via `tokio-tungstenite`) bound strictly to `localhost` on a dynamic/ephemeral port. `xterm.js` in the frontend connects directly to this local server using the browser-native `WebSocket` API. Raw SSH binary and text streams bypass the Tauri IPC bridge entirely, piping straight from the Rust SSH crate into the terminal interface.

---

## 3. Project Directory Structure

```text
sypherterm/
├── index.html                  # Entry point (Vanilla Svelte + Vite)
├── src/                        # Frontend Application (Svelte)
│   ├── assets/
│   ├── components/
│   │   ├── Sidebar.svelte
│   │   └── TerminalInstance.svelte
│   ├── lib/
│   │   ├── store.ts            # Svelte reactive stores for state
│   │   └── websocket.ts        # xterm.js ↔ Tauri WS plugin connector
│   ├── App.svelte
│   └── main.ts
└── src-tauri/                  # Backend Application (Rust)
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/
        ├── crypto/             # AES-256-GCM Encryption Layer
        │   └── mod.rs
        ├── ssh/                # Native SSH Engine (Russh / Thrussh)
        │   ├── client.rs
        │   └── mod.rs
        ├── sync/               # Cloud Sync Engine (S3, Drive API)
        │   ├── mod.rs
        │   └── s3.rs
        ├── ws/                 # Local WebSocket Server (tokio-tungstenite)
        │   └── mod.rs
        ├── commands.rs         # Tauri IPC Control Commands
        ├── lib.rs              # Plugin registration & app setup
        └── main.rs             # Application entrypoint

```

---

## 4. Tauri Plugins (Pre-built)

To accelerate development and reduce boilerplate, SypherTerm leverages official Tauri v2 plugins for common cross-platform concerns:

| Plugin | Crate | npm Package | Purpose |
|---|---|---|---|
| **Store** | `tauri-plugin-store` | `@tauri-apps/plugin-store` | Persistent key-value storage for user configs, SSH profiles, session state |
| **WebSocket** | `tauri-plugin-websocket` | `@tauri-apps/plugin-websocket` | Auxiliary WebSocket client (cloud sync, real-time collaboration). Not used for the Data Plane. |
| **OS Info** | `tauri-plugin-os` | `@tauri-apps/plugin-os` | Detect platform, arch, and locale for OS-specific behavior |
| **Notification** | `tauri-plugin-notification` | `@tauri-apps/plugin-notification` | Native desktop notifications (e.g., connection dropped, sync complete) |
| **Clipboard** | `tauri-plugin-clipboard-manager` | `@tauri-apps/plugin-clipboard-manager` | Read/write system clipboard from the terminal |
| **Opener** | `tauri-plugin-opener` | `@tauri-apps/plugin-opener` | Open URLs/files in the default system application |

> **Note:** `tauri-plugin-websocket` is a WebSocket **client** only — it connects to external servers but cannot host one. The Data Plane requires a local WebSocket **server** (see `src-tauri/src/ws/` below), which is implemented manually with `tokio-tungstenite`. The plugin remains useful for auxiliary connections (e.g., cloud sync WebSocket endpoints, collaboration features).

---

## 5. Core Rust Module Skeletons

### `src-tauri/src/ws/mod.rs`

*Local WebSocket server for the Data Plane. Binds to `localhost` on a dynamic port; `xterm.js` connects to receive SSH streams.*

```rust
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct StreamServer {
    port: u16,
}

impl StreamServer {
    pub fn new() -> Self {
        // Initialize on port 0 to let the OS assign a random free port dynamically
        Self { port: 0 }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let bound_addr = listener.local_addr()?;
        println!("WebSocket Data Plane listening on: ws://{}", bound_addr);

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    if let Ok(mut ws_stream) = accept_async(stream).await {
                        println!("Frontend connected to Data Plane.");
                        // Handle splitting incoming/outgoing SSH streams to xterm.js here
                    }
                });
            }
        });

        Ok(())
    }
}
```

### `src-tauri/src/ssh/mod.rs`

*Manages active asynchronous SSH sessions. WebSocket streaming is handled by `tauri-plugin-websocket`.*

```rust
pub struct SshSession {
    pub host: String,
    pub username: String,
    // Add russh client handles here
}

impl SshSession {
    pub async fn connect(host: &str, user: &str) -> Result<Self, String> {
        // Authenticate and establish channel using underlying SSH crate
        Ok(Self {
            host: host.to_string(),
            username: user.to_string(),
        })
    }

    pub async fn pipe_to_stream(&self, ws_tx: serde_json::Value) {
        // Async loop channeling bytes from SSH stdout directly into the WebSocket
    }
}

```

### `src-tauri/src/crypto/mod.rs`

*Local Zero-Knowledge encryption engine wrapper.*

```rust
pub struct CryptoEngine;

impl CryptoEngine {
    /// Encrypts local profile data using AES-256-GCM before syncing to the cloud
    pub fn encrypt_payload(data: &[u8], master_password: &str) -> Result<Vec<u8>, String> {
        // 1. Derive encryption key via Argon2id from master_password
        // 2. Encrypt bytes using AES-256-GCM with secure nonce
        // 3. Return payload ready for export/sync
        Ok(vec![])
    }

    /// Decrypts cloud payloads locally
    pub fn decrypt_payload(encrypted_data: &[u8], master_password: &str) -> Result<Vec<u8>, String> {
        Ok(vec![])
    }
}

```

### `src-tauri/src/commands.rs`

*Tauri Control Plane IPC commands. Uses `tauri-plugin-store` for config persistence.*

```rust
use tauri::State;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn connect_ssh(host: String, username: String) -> Result<String, String> {
    println!("Control Plane: Initiating connection to {}", host);
    // 1. Call SshSession::connect
    // 2. Register session in Tauri AppState
    // 3. Return acknowledgment to Svelte UI
    Ok(format!("Connected to {}", host))
}

#[tauri::command]
pub async fn trigger_cloud_sync(provider: String) -> Result<(), String> {
    println!("Control Plane: Triggering encrypted sync to {}", provider);
    // Call sync module routines
    Ok(())
}

#[tauri::command]
pub async fn save_config(app: tauri::AppHandle, key: String, value: serde_json::Value) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set(key, value).map_err(|e| e.to_string())
}
```

### `src-tauri/src/lib.rs`

*Application entry point — registers all Tauri plugins and IPC commands.*

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            commands::connect_ssh,
            commands::trigger_cloud_sync,
            commands::save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
