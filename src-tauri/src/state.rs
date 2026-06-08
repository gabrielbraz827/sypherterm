use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VaultState {
    Missing,
    Locked,
    Unlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataPlaneState {
    Stopped,
    Starting,
    Running,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_version: String,
    pub vault: VaultState,
    pub active_sessions: usize,
    pub data_plane: DataPlaneState,
}

pub struct AppState {
    active_sessions: AtomicUsize,
    vault: Mutex<VaultState>,
    data_plane: Mutex<DataPlaneState>,
    vault_payload: Mutex<Option<Zeroizing<Vec<u8>>>>,
    vault_key: Mutex<Option<Zeroizing<[u8; 32]>>>,
}

#[derive(Debug)]
pub enum AppStateError {
    LockPoisoned(&'static str),
}

impl fmt::Display for AppStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LockPoisoned(resource) => {
                write!(formatter, "internal_error: {resource} state lock poisoned")
            }
        }
    }
}

impl std::error::Error for AppStateError {}

impl AppState {
    pub fn status(&self) -> Result<AppStatus, AppStateError> {
        Ok(AppStatus {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            vault: self.lock_value(&self.vault, "vault")?,
            active_sessions: self.active_sessions.load(Ordering::Relaxed),
            data_plane: self.lock_value(&self.data_plane, "data_plane")?,
        })
    }

    pub fn set_vault_state(&self, vault_state: VaultState) -> Result<(), AppStateError> {
        self.set_lock_value(&self.vault, vault_state, "vault")
    }

    #[cfg(test)]
    pub fn unlock_vault_payload(&self, payload: Vec<u8>) -> Result<(), AppStateError> {
        self.set_lock_value(&self.vault_key, None, "vault_key")?;
        self.set_lock_value(
            &self.vault_payload,
            Some(Zeroizing::new(payload)),
            "vault_payload",
        )?;
        self.set_vault_state(VaultState::Unlocked)
    }

    pub fn unlock_vault_secret(
        &self,
        payload: Vec<u8>,
        key: Zeroizing<[u8; 32]>,
    ) -> Result<(), AppStateError> {
        self.set_lock_value(
            &self.vault_payload,
            Some(Zeroizing::new(payload)),
            "vault_payload",
        )?;
        self.set_lock_value(&self.vault_key, Some(key), "vault_key")?;
        self.set_vault_state(VaultState::Unlocked)
    }

    pub fn update_vault_payload(&self, payload: Vec<u8>) -> Result<(), AppStateError> {
        self.set_lock_value(
            &self.vault_payload,
            Some(Zeroizing::new(payload)),
            "vault_payload",
        )
    }

    pub fn lock_vault_payload(&self) -> Result<(), AppStateError> {
        self.set_lock_value(&self.vault_payload, None, "vault_payload")?;
        self.set_lock_value(&self.vault_key, None, "vault_key")?;
        self.set_vault_state(VaultState::Locked)
    }

    pub fn set_data_plane_state(
        &self,
        data_plane_state: DataPlaneState,
    ) -> Result<(), AppStateError> {
        self.set_lock_value(&self.data_plane, data_plane_state, "data_plane")
    }

    #[allow(dead_code)]
    pub fn vault_payload(&self) -> Result<Option<Vec<u8>>, AppStateError> {
        self.vault_payload
            .lock()
            .map(|guard| guard.as_ref().map(|payload| payload.to_vec()))
            .map_err(|_: PoisonError<_>| AppStateError::LockPoisoned("vault_payload"))
    }

    pub fn vault_key(&self) -> Result<Option<Zeroizing<[u8; 32]>>, AppStateError> {
        self.vault_key
            .lock()
            .map(|guard| guard.as_ref().cloned())
            .map_err(|_: PoisonError<_>| AppStateError::LockPoisoned("vault_key"))
    }

    fn lock_value<T: Copy>(
        &self,
        mutex: &Mutex<T>,
        resource: &'static str,
    ) -> Result<T, AppStateError> {
        mutex
            .lock()
            .map(|guard| *guard)
            .map_err(|_: PoisonError<_>| AppStateError::LockPoisoned(resource))
    }

    fn set_lock_value<T>(
        &self,
        mutex: &Mutex<T>,
        value: T,
        resource: &'static str,
    ) -> Result<(), AppStateError> {
        mutex
            .lock()
            .map(|mut guard| *guard = value)
            .map_err(|_: PoisonError<_>| AppStateError::LockPoisoned(resource))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_sessions: AtomicUsize::new(0),
            vault: Mutex::new(VaultState::Missing),
            data_plane: Mutex::new(DataPlaneState::Stopped),
            vault_payload: Mutex::new(None),
            vault_key: Mutex::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, DataPlaneState, VaultState};

    #[test]
    fn default_status_matches_initial_app_state() {
        let state = AppState::default();
        let status = state
            .status()
            .expect("default app state should be readable");

        assert_eq!(status.app_version, env!("CARGO_PKG_VERSION"));
        assert!(matches!(status.vault, VaultState::Missing));
        assert_eq!(status.active_sessions, 0);
        assert!(matches!(status.data_plane, DataPlaneState::Stopped));
    }
}
