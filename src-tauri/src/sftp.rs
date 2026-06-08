use crate::ssh::{SshError, SshRegistry};
use russh_sftp::client::error::Error as RusshSftpError;
use russh_sftp::protocol::{FileType, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use uuid::Uuid;

const TRANSFER_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpPathRequest {
    pub session_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferRequest {
    pub session_id: String,
    pub remote_path: String,
    pub local_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpRenameRequest {
    pub session_id: String,
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpDeleteRequest {
    pub session_id: String,
    pub path: String,
    pub kind: Option<RemoteEntryKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpCancelRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDirEntry {
    pub name: String,
    pub path: String,
    pub kind: RemoteEntryKind,
    pub size: u64,
    pub permissions: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteEntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Download,
    Upload,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferJob {
    pub job_id: String,
    pub session_id: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub remote_path: String,
    pub local_path: String,
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferState {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SftpError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SftpRegistry {
    jobs: Arc<Mutex<HashMap<String, TransferJob>>>,
    cancellations: Arc<Mutex<HashSet<String>>>,
}

impl SftpRegistry {
    pub async fn list_dir(
        &self,
        ssh: &SshRegistry,
        request: SftpPathRequest,
    ) -> Result<Vec<RemoteDirEntry>, SftpError> {
        let path = normalize_required_path(request.path, "remote path")?;
        let sftp = ssh.open_sftp(&request.session_id).await?;
        let mut entries = sftp
            .read_dir(path)
            .await
            .map_err(SftpError::from_russh_sftp)?
            .map(RemoteDirEntry::from)
            .collect::<Vec<_>>();

        sort_entries(&mut entries);
        Ok(entries)
    }

    pub async fn mkdir(
        &self,
        ssh: &SshRegistry,
        request: SftpPathRequest,
    ) -> Result<RemoteDirEntry, SftpError> {
        let path = normalize_required_path(request.path, "remote path")?;
        let sftp = ssh.open_sftp(&request.session_id).await?;
        sftp.create_dir(path.clone())
            .await
            .map_err(SftpError::from_russh_sftp)?;
        Ok(remote_operation_entry(path, RemoteEntryKind::Directory))
    }

    pub async fn rename(
        &self,
        ssh: &SshRegistry,
        request: SftpRenameRequest,
    ) -> Result<RemoteDirEntry, SftpError> {
        let old_path = normalize_required_path(request.old_path, "old remote path")?;
        let new_path = normalize_required_path(request.new_path, "new remote path")?;
        let sftp = ssh.open_sftp(&request.session_id).await?;
        sftp.rename(old_path, new_path.clone())
            .await
            .map_err(SftpError::from_russh_sftp)?;
        Ok(remote_operation_entry(new_path, RemoteEntryKind::Other))
    }

    pub async fn delete(
        &self,
        ssh: &SshRegistry,
        request: SftpDeleteRequest,
    ) -> Result<RemoteDirEntry, SftpError> {
        let path = normalize_required_path(request.path, "remote path")?;
        let kind = request.kind.unwrap_or(RemoteEntryKind::File);
        let sftp = ssh.open_sftp(&request.session_id).await?;
        match kind {
            RemoteEntryKind::Directory => sftp.remove_dir(path.clone()).await,
            RemoteEntryKind::File | RemoteEntryKind::Symlink | RemoteEntryKind::Other => {
                sftp.remove_file(path.clone()).await
            }
        }
        .map_err(SftpError::from_russh_sftp)?;

        Ok(remote_operation_entry(path, kind))
    }

    pub async fn download(
        &self,
        ssh: &SshRegistry,
        request: SftpTransferRequest,
    ) -> Result<TransferJob, SftpError> {
        let remote_path = normalize_required_path(request.remote_path, "remote path")?;
        let local_path = normalize_required_path(request.local_path, "local path")?;
        let mut job = self
            .start_job(
                request.session_id.clone(),
                TransferDirection::Download,
                remote_path.clone(),
                local_path.clone(),
                None,
            )
            .await;
        let fallback_job = job.clone();

        let result = async {
            let sftp = ssh.open_sftp(&request.session_id).await?;
            let metadata = sftp
                .metadata(remote_path.clone())
                .await
                .map_err(SftpError::from_russh_sftp)?;
            job.total_bytes = Some(metadata.len());
            self.update_job(&job).await;

            let mut remote_file = sftp
                .open(remote_path)
                .await
                .map_err(SftpError::from_russh_sftp)?;
            let mut local_file = fs::File::create(local_path).await.map_err(SftpError::io)?;
            let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];

            loop {
                if self.is_cancelled(&job.job_id).await {
                    job.state = TransferState::Cancelled;
                    job.message = Some("transfer cancelled".to_string());
                    self.update_job(&job).await;
                    return Ok(job.clone());
                }

                let bytes_read = remote_file.read(&mut buffer).await.map_err(SftpError::io)?;
                if bytes_read == 0 {
                    break;
                }
                local_file
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(SftpError::io)?;
                job.bytes_transferred += bytes_read as u64;
                self.update_job(&job).await;
            }

            local_file.flush().await.map_err(SftpError::io)?;
            job.state = TransferState::Completed;
            self.update_job(&job).await;
            Ok(job.clone())
        }
        .await;

        self.finish_result(fallback_job, result).await
    }

    pub async fn upload(
        &self,
        ssh: &SshRegistry,
        request: SftpTransferRequest,
    ) -> Result<TransferJob, SftpError> {
        let remote_path = normalize_required_path(request.remote_path, "remote path")?;
        let local_path = normalize_required_path(request.local_path, "local path")?;
        let total_bytes = fs::metadata(&local_path)
            .await
            .ok()
            .map(|metadata| metadata.len());
        let mut job = self
            .start_job(
                request.session_id.clone(),
                TransferDirection::Upload,
                remote_path.clone(),
                local_path.clone(),
                total_bytes,
            )
            .await;
        let fallback_job = job.clone();

        let result = async {
            let sftp = ssh.open_sftp(&request.session_id).await?;
            let mut local_file = fs::File::open(local_path).await.map_err(SftpError::io)?;
            let mut remote_file = sftp
                .create(remote_path)
                .await
                .map_err(SftpError::from_russh_sftp)?;
            let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];

            loop {
                if self.is_cancelled(&job.job_id).await {
                    job.state = TransferState::Cancelled;
                    job.message = Some("transfer cancelled".to_string());
                    self.update_job(&job).await;
                    return Ok(job.clone());
                }

                let bytes_read = local_file.read(&mut buffer).await.map_err(SftpError::io)?;
                if bytes_read == 0 {
                    break;
                }
                remote_file
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(SftpError::io)?;
                job.bytes_transferred += bytes_read as u64;
                self.update_job(&job).await;
            }

            remote_file.flush().await.map_err(SftpError::io)?;
            job.state = TransferState::Completed;
            self.update_job(&job).await;
            Ok(job.clone())
        }
        .await;

        self.finish_result(fallback_job, result).await
    }

    pub async fn cancel_transfer(
        &self,
        request: SftpCancelRequest,
    ) -> Result<TransferJob, SftpError> {
        self.cancellations
            .lock()
            .await
            .insert(request.job_id.clone());

        let mut jobs = self.jobs.lock().await;
        let Some(job) = jobs.get_mut(&request.job_id) else {
            return Err(SftpError::new(
                "not_found",
                "transfer job is not active",
                true,
            ));
        };

        if job.state == TransferState::Running {
            job.state = TransferState::Cancelled;
            job.message = Some("transfer cancellation requested".to_string());
        }

        Ok(job.clone())
    }

    async fn start_job(
        &self,
        session_id: String,
        direction: TransferDirection,
        remote_path: String,
        local_path: String,
        total_bytes: Option<u64>,
    ) -> TransferJob {
        let job = TransferJob {
            job_id: Uuid::new_v4().to_string(),
            session_id,
            direction,
            state: TransferState::Running,
            remote_path,
            local_path,
            bytes_transferred: 0,
            total_bytes,
            message: None,
        };
        self.jobs
            .lock()
            .await
            .insert(job.job_id.clone(), job.clone());
        job
    }

    async fn update_job(&self, job: &TransferJob) {
        self.jobs
            .lock()
            .await
            .insert(job.job_id.clone(), job.clone());
    }

    async fn is_cancelled(&self, job_id: &str) -> bool {
        self.cancellations.lock().await.contains(job_id)
    }

    async fn finish_result(
        &self,
        mut job: TransferJob,
        result: Result<TransferJob, SftpError>,
    ) -> Result<TransferJob, SftpError> {
        match result {
            Ok(job) => {
                self.cancellations.lock().await.remove(&job.job_id);
                Ok(job)
            }
            Err(error) => {
                job.state = TransferState::Failed;
                job.message = Some(error.message.clone());
                self.update_job(&job).await;
                self.cancellations.lock().await.remove(&job.job_id);
                Err(error)
            }
        }
    }
}

impl RemoteDirEntry {
    fn from(entry: russh_sftp::client::fs::DirEntry) -> Self {
        let metadata = entry.metadata();
        Self {
            name: entry.file_name(),
            path: entry.path(),
            kind: RemoteEntryKind::from(metadata.file_type()),
            size: metadata.len(),
            permissions: metadata
                .permissions
                .map(|_| metadata.permissions().to_string()),
            modified_at: metadata.mtime.map(|mtime| mtime.to_string()),
        }
    }
}

impl From<FileType> for RemoteEntryKind {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Dir => Self::Directory,
            FileType::File => Self::File,
            FileType::Symlink => Self::Symlink,
            FileType::Other => Self::Other,
        }
    }
}

impl From<SshError> for SftpError {
    fn from(error: SshError) -> Self {
        Self::new(leak_code(error.code), error.message, error.recoverable)
    }
}

impl SftpError {
    fn new(code: &'static str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }

    fn from_russh_sftp(error: RusshSftpError) -> Self {
        match error {
            RusshSftpError::Status(status) => match status.status_code {
                StatusCode::NoSuchFile => Self::new("not_found", "remote path was not found", true),
                StatusCode::PermissionDenied => {
                    Self::new("permission_denied", "remote permission denied", true)
                }
                StatusCode::OpUnsupported => Self::new(
                    "unsupported_mode",
                    "remote SFTP operation is unsupported",
                    true,
                ),
                _ => Self::new("sftp_error", status.status_code.to_string(), true),
            },
            RusshSftpError::IO(message) => Self::new("io_error", message, true),
            RusshSftpError::Timeout => Self::new("network_error", "SFTP request timed out", true),
            RusshSftpError::Limited(message) | RusshSftpError::UnexpectedBehavior(message) => {
                Self::new("sftp_error", message, true)
            }
            RusshSftpError::UnexpectedPacket => {
                Self::new("sftp_error", "unexpected SFTP packet", true)
            }
        }
    }

    fn io(error: std::io::Error) -> Self {
        Self::new("io_error", error.to_string(), true)
    }
}

impl fmt::Display for SftpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

fn normalize_required_path(path: String, label: &str) -> Result<String, SftpError> {
    let normalized = path.trim().to_string();
    if normalized.is_empty() {
        return Err(SftpError::new(
            "validation_error",
            format!("{label} is required"),
            true,
        ));
    }
    Ok(normalized)
}

fn remote_operation_entry(path: String, kind: RemoteEntryKind) -> RemoteDirEntry {
    let name = path
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(&path)
        .to_string();

    RemoteDirEntry {
        name,
        path,
        kind,
        size: 0,
        permissions: None,
        modified_at: None,
    }
}

fn sort_entries(entries: &mut [RemoteDirEntry]) {
    entries.sort_by(|left, right| {
        let left_group = if left.kind == RemoteEntryKind::Directory {
            0
        } else {
            1
        };
        let right_group = if right.kind == RemoteEntryKind::Directory {
            0
        } else {
            1
        };

        left_group
            .cmp(&right_group)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

fn leak_code(code: String) -> &'static str {
    Box::leak(code.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_required_path, remote_operation_entry, sort_entries, RemoteDirEntry,
        RemoteEntryKind,
    };

    #[test]
    fn remote_entries_sort_directories_first_then_by_name() {
        let mut entries = vec![
            entry("z.txt", RemoteEntryKind::File),
            entry("src", RemoteEntryKind::Directory),
            entry("a.txt", RemoteEntryKind::File),
        ];

        sort_entries(&mut entries);

        assert_eq!(entries[0].name, "src");
        assert_eq!(entries[1].name, "a.txt");
        assert_eq!(entries[2].name, "z.txt");
    }

    #[test]
    fn required_path_rejects_empty_values() {
        let error = normalize_required_path("  ".to_string(), "remote path").unwrap_err();

        assert_eq!(error.code, "validation_error");
    }

    #[test]
    fn operation_entry_derives_name_from_path() {
        let entry = remote_operation_entry("/var/www/app".to_string(), RemoteEntryKind::Directory);

        assert_eq!(entry.name, "app");
        assert_eq!(entry.kind, RemoteEntryKind::Directory);
    }

    fn entry(name: &str, kind: RemoteEntryKind) -> RemoteDirEntry {
        RemoteDirEntry {
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            kind,
            size: 0,
            permissions: None,
            modified_at: None,
        }
    }
}
