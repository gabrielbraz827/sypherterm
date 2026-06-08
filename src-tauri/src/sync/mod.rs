use crate::crypto::{export_vault_envelope, import_vault_envelope, VaultEnvelope, VaultError};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Runtime};
use uuid::Uuid;

const INDEX_FILE: &str = "sypherterm-sync-index.json";
const VERSION_PREFIX: &str = "vault";
const PROVIDER_VERSION: u8 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProviderConfig {
    pub provider_id: String,
    pub kind: SyncProviderKind,
    pub local_path: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncProviderKind {
    Local,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProviderStatus {
    pub provider_id: String,
    pub kind: String,
    pub state: String,
    pub root_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRequest {
    pub provider_id: String,
    pub direction: SyncDirection,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    Push,
    Pull,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncJobStatus {
    pub job_id: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncVersion {
    pub version_id: String,
    pub device_id: String,
    pub payload_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SyncIndex {
    version: u8,
    versions: Vec<SyncVersion>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

pub trait SyncProvider {
    fn test(&self) -> Result<SyncProviderStatus, SyncError>;
    fn list_versions(&self) -> Result<Vec<SyncVersion>, SyncError>;
    fn push<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        device_id: &str,
    ) -> Result<SyncJobStatus, SyncError>;
    fn pull<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        state: &AppState,
    ) -> Result<SyncJobStatus, SyncError>;
    fn bidirectional<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        state: &AppState,
        device_id: &str,
    ) -> Result<SyncJobStatus, SyncError>;
}

#[derive(Debug, Clone)]
pub struct LocalSyncProvider {
    provider_id: String,
    root: PathBuf,
}

impl LocalSyncProvider {
    fn from_config(config: SyncProviderConfig) -> Result<Self, SyncError> {
        let SyncProviderConfig {
            provider_id,
            kind,
            local_path,
            device_id,
        } = config;

        if kind != SyncProviderKind::Local {
            return Err(SyncError::new(
                "sync_failed",
                "only local sync provider is implemented in this MVP",
                true,
            ));
        }

        let _ = device_id;
        Self::new(provider_id, local_path)
    }

    fn from_provider_id(provider_id: String) -> Result<Self, SyncError> {
        Self::new(provider_id.clone(), provider_id)
    }

    fn new(provider_id: String, local_path: String) -> Result<Self, SyncError> {
        let local_path = local_path.trim();
        if local_path.is_empty() {
            return Err(SyncError::new(
                "sync_failed",
                "local sync path is required",
                true,
            ));
        }

        Ok(Self {
            provider_id,
            root: PathBuf::from(local_path),
        })
    }

    fn ensure_root(&self) -> Result<(), SyncError> {
        fs::create_dir_all(&self.root).map_err(SyncError::io)?;
        Ok(())
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(INDEX_FILE)
    }

    fn version_path(&self, version_id: &str) -> PathBuf {
        self.root
            .join(format!("{VERSION_PREFIX}-{version_id}.json"))
    }

    fn load_index(&self) -> Result<SyncIndex, SyncError> {
        self.ensure_root()?;
        let path = self.index_path();
        if !path.exists() {
            return Ok(SyncIndex {
                version: PROVIDER_VERSION,
                versions: Vec::new(),
            });
        }

        let bytes = fs::read(path).map_err(SyncError::io)?;
        let index = serde_json::from_slice::<SyncIndex>(&bytes)
            .map_err(|error| SyncError::failed(format!("sync index is invalid: {error}")))?;
        if index.version != PROVIDER_VERSION {
            return Err(SyncError::failed(format!(
                "unsupported sync index version {}",
                index.version
            )));
        }
        Ok(index)
    }

    fn save_index(&self, index: &SyncIndex) -> Result<(), SyncError> {
        self.ensure_root()?;
        let bytes = serde_json::to_vec_pretty(index).map_err(SyncError::json)?;
        fs::write(self.index_path(), bytes).map_err(SyncError::io)
    }

    fn latest_version(&self) -> Result<Option<SyncVersion>, SyncError> {
        let mut versions = self.load_index()?.versions;
        versions.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(versions.into_iter().next())
    }

    fn load_envelope(&self, version: &SyncVersion) -> Result<VaultEnvelope, SyncError> {
        let bytes = fs::read(self.version_path(&version.version_id)).map_err(SyncError::io)?;
        serde_json::from_slice::<VaultEnvelope>(&bytes).map_err(|error| {
            SyncError::failed(format!("synced vault envelope is invalid: {error}"))
        })
    }

    fn append_version(
        &self,
        envelope: &VaultEnvelope,
        device_id: &str,
    ) -> Result<SyncVersion, SyncError> {
        self.ensure_root()?;
        let payload = serde_json::to_vec_pretty(envelope).map_err(SyncError::json)?;
        let payload_hash = payload_hash(&payload);
        let mut index = self.load_index()?;

        if let Some(existing) = index
            .versions
            .iter()
            .find(|version| version.payload_hash == payload_hash)
        {
            return Ok(existing.clone());
        }

        let version = SyncVersion {
            version_id: format!("{}-{}", timestamp(), &payload_hash[..12]),
            device_id: device_id.to_string(),
            payload_hash,
            created_at: timestamp(),
        };
        fs::write(self.version_path(&version.version_id), payload).map_err(SyncError::io)?;
        index.versions.push(version.clone());
        self.save_index(&index)?;
        Ok(version)
    }
}

impl SyncProvider for LocalSyncProvider {
    fn test(&self) -> Result<SyncProviderStatus, SyncError> {
        self.ensure_root()?;
        let test_path = self
            .root
            .join(format!(".sypherterm-write-test-{}", Uuid::new_v4()));
        fs::write(&test_path, b"ok").map_err(SyncError::io)?;
        fs::remove_file(test_path).map_err(SyncError::io)?;

        Ok(SyncProviderStatus {
            provider_id: self.provider_id.clone(),
            kind: "local".to_string(),
            state: "ready".to_string(),
            root_path: self.root.display().to_string(),
        })
    }

    fn list_versions(&self) -> Result<Vec<SyncVersion>, SyncError> {
        let mut versions = self.load_index()?.versions;
        versions.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(versions)
    }

    fn push<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        device_id: &str,
    ) -> Result<SyncJobStatus, SyncError> {
        let envelope = export_vault_envelope(app)?;
        let payload = serde_json::to_vec_pretty(&envelope).map_err(SyncError::json)?;
        let local_hash = payload_hash(&payload);

        if let Some(latest) = self.latest_version()? {
            if latest.payload_hash != local_hash && latest.device_id != device_id {
                return Err(SyncError::conflict(latest));
            }
            if latest.payload_hash == local_hash {
                return Ok(job_status(
                    "up_to_date",
                    Some(latest),
                    Some("remote already has this vault"),
                ));
            }
        }

        let version = self.append_version(&envelope, device_id)?;
        Ok(job_status("pushed", Some(version), None))
    }

    fn pull<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        state: &AppState,
    ) -> Result<SyncJobStatus, SyncError> {
        let Some(latest) = self.latest_version()? else {
            return Ok(job_status("up_to_date", None, Some("no remote versions")));
        };
        let remote_envelope = self.load_envelope(&latest)?;

        match export_vault_envelope(app) {
            Ok(local_envelope) => {
                let local_payload =
                    serde_json::to_vec_pretty(&local_envelope).map_err(SyncError::json)?;
                let local_hash = payload_hash(&local_payload);
                if local_hash == latest.payload_hash {
                    Ok(job_status(
                        "up_to_date",
                        Some(latest),
                        Some("local vault is current"),
                    ))
                } else {
                    Err(SyncError::conflict(latest))
                }
            }
            Err(error) if error.code == "vault_locked" => {
                import_vault_envelope(app, state, remote_envelope)?;
                Ok(job_status("pulled", Some(latest), None))
            }
            Err(error) => Err(error.into()),
        }
    }

    fn bidirectional<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        state: &AppState,
        device_id: &str,
    ) -> Result<SyncJobStatus, SyncError> {
        match self.latest_version()? {
            None => self.push(app, device_id),
            Some(_) => match export_vault_envelope(app) {
                Ok(local_envelope) => {
                    let local_payload =
                        serde_json::to_vec_pretty(&local_envelope).map_err(SyncError::json)?;
                    let local_hash = payload_hash(&local_payload);
                    let latest = self.latest_version()?.expect("latest was already checked");
                    if local_hash == latest.payload_hash {
                        Ok(job_status("up_to_date", Some(latest), None))
                    } else {
                        Err(SyncError::conflict(latest))
                    }
                }
                Err(error) if error.code == "vault_locked" => self.pull(app, state),
                Err(error) => Err(error.into()),
            },
        }
    }
}

pub fn test_sync_provider(config: SyncProviderConfig) -> Result<SyncProviderStatus, SyncError> {
    LocalSyncProvider::from_config(config)?.test()
}

pub fn list_sync_versions(config: SyncProviderConfig) -> Result<Vec<SyncVersion>, SyncError> {
    LocalSyncProvider::from_config(config)?.list_versions()
}

pub fn trigger_sync<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    request: SyncRequest,
) -> Result<SyncJobStatus, SyncError> {
    let provider = LocalSyncProvider::from_provider_id(request.provider_id)?;
    let device_id = request.device_id.unwrap_or_else(default_device_id);

    match request.direction {
        SyncDirection::Push => provider.push(app, &device_id),
        SyncDirection::Pull => provider.pull(app, state),
        SyncDirection::Bidirectional => provider.bidirectional(app, state, &device_id),
    }
}

fn job_status(state: &str, version: Option<SyncVersion>, message: Option<&str>) -> SyncJobStatus {
    SyncJobStatus {
        job_id: Uuid::new_v4().to_string(),
        state: state.to_string(),
        version_id: version.as_ref().map(|version| version.version_id.clone()),
        payload_hash: version.as_ref().map(|version| version.payload_hash.clone()),
        message: message.map(str::to_string),
    }
}

fn payload_hash(payload: &[u8]) -> String {
    let hash = Sha256::digest(payload);
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn default_device_id() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local-device".to_string())
}

impl SyncError {
    fn new(code: &'static str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }

    fn failed(message: impl Into<String>) -> Self {
        Self::new("sync_failed", message, true)
    }

    fn conflict(version: SyncVersion) -> Self {
        Self::new(
            "conflict_detected",
            format!(
                "remote version {} from device {} has different payload hash {}",
                version.version_id, version.device_id, version.payload_hash
            ),
            true,
        )
    }

    fn io(error: std::io::Error) -> Self {
        Self::new(
            "sync_failed",
            format!("local sync provider failed: {error}"),
            true,
        )
    }

    fn json(error: serde_json::Error) -> Self {
        Self::new(
            "sync_failed",
            format!("sync payload is invalid: {error}"),
            true,
        )
    }
}

impl fmt::Display for SyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl From<VaultError> for SyncError {
    fn from(error: VaultError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        job_status, payload_hash, LocalSyncProvider, SyncError, SyncProvider, SyncVersion,
    };

    #[test]
    fn payload_hash_is_stable_for_same_bytes() {
        assert_eq!(payload_hash(b"vault"), payload_hash(b"vault"));
        assert_ne!(payload_hash(b"vault"), payload_hash(b"other"));
    }

    #[test]
    fn conflict_error_exposes_remote_version_without_payload() {
        let error = SyncError::conflict(SyncVersion {
            version_id: "v1".to_string(),
            device_id: "device-a".to_string(),
            payload_hash: "abc".to_string(),
            created_at: "1".to_string(),
        });

        assert_eq!(error.code, "conflict_detected");
        assert!(error.message.contains("device-a"));
        assert!(!error.message.contains("ciphertext"));
    }

    #[test]
    fn job_status_includes_hash_metadata() {
        let status = job_status(
            "pushed",
            Some(SyncVersion {
                version_id: "v1".to_string(),
                device_id: "device-a".to_string(),
                payload_hash: "abc".to_string(),
                created_at: "1".to_string(),
            }),
            None,
        );

        assert_eq!(status.state, "pushed");
        assert_eq!(status.version_id.as_deref(), Some("v1"));
        assert_eq!(status.payload_hash.as_deref(), Some("abc"));
    }

    #[test]
    fn sync_error_serializes_as_command_error_shape() {
        let value = serde_json::to_value(SyncError::failed("nope")).expect("error serializes");

        assert_eq!(value["code"], "sync_failed");
        assert_eq!(value["recoverable"], true);
    }

    #[test]
    fn local_provider_test_creates_writable_root() {
        let root =
            std::env::temp_dir().join(format!("sypherterm-sync-test-{}", uuid::Uuid::new_v4()));
        let provider =
            LocalSyncProvider::new("local-test".to_string(), root.display().to_string()).unwrap();

        let status = provider.test().expect("provider root should be writable");

        assert_eq!(status.state, "ready");
        assert!(root.exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
