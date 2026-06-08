use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
use zeroize::Zeroizing;

use crate::state::{AppState, VaultState};

const STORE_PATH: &str = "sypherterm.local.json";
const VAULT_KEY: &str = "vault";
const VAULT_VERSION: u8 = 1;
const INITIAL_VAULT_PAYLOAD: &[u8] = br#"{}"#;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultEnvelope {
    pub version: u8,
    pub kdf: KdfParams,
    pub cipher: CipherPayload,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KdfParams {
    pub algorithm: String,
    #[serde(rename = "memoryKiB")]
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub salt_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CipherPayload {
    pub algorithm: String,
    pub nonce_base64: String,
    pub ciphertext_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub state: VaultState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultRequest {
    pub master_password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockVaultRequest {
    pub master_password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeMasterPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

pub struct CryptoEngine;

impl CryptoEngine {
    pub fn encrypt_payload(
        data: &[u8],
        master_password: &str,
    ) -> Result<VaultEnvelope, VaultError> {
        encrypt_payload_with_created_at(data, master_password, timestamp())
    }

    #[cfg(test)]
    pub fn decrypt_payload(
        envelope: &VaultEnvelope,
        master_password: &str,
    ) -> Result<Vec<u8>, VaultError> {
        let (payload, _) = Self::decrypt_payload_and_key(envelope, master_password)?;
        Ok(payload)
    }

    pub fn decrypt_payload_and_key(
        envelope: &VaultEnvelope,
        master_password: &str,
    ) -> Result<(Vec<u8>, Zeroizing<[u8; KEY_LEN]>), VaultError> {
        validate_envelope(envelope)?;
        let salt = decode_base64(&envelope.kdf.salt_base64)?;
        let nonce = decode_base64(&envelope.cipher.nonce_base64)?;
        let ciphertext = decode_base64(&envelope.cipher.ciphertext_base64)?;
        validate_decoded_lengths(&salt, &nonce)?;
        let key = derive_key(master_password, &salt, &envelope.kdf)?;
        let cipher = Aes256Gcm::new_from_slice(&key[..])
            .map_err(|_| VaultError::crypto_error("failed to initialize AES-256-GCM cipher"))?;

        let payload = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| VaultError::invalid_password("master password is invalid"))?;

        Ok((payload, key))
    }
}

pub fn create_vault<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    request: CreateVaultRequest,
) -> Result<VaultStatus, VaultError> {
    let store = app.store(STORE_PATH)?;
    if store.get(VAULT_KEY).is_some() {
        return Err(VaultError::new(
            "vault_exists",
            "vault already exists",
            true,
        ));
    }

    validate_master_password(&request.master_password)?;
    let envelope = CryptoEngine::encrypt_payload(INITIAL_VAULT_PAYLOAD, &request.master_password)?;
    let key = key_for_envelope(&envelope, &request.master_password)?;
    store.set(VAULT_KEY, serde_json::to_value(&envelope)?);
    store.save()?;
    state.unlock_vault_secret(INITIAL_VAULT_PAYLOAD.to_vec(), key)?;

    Ok(VaultStatus {
        state: VaultState::Unlocked,
        version: Some(envelope.version),
    })
}

pub fn unlock_vault<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    request: UnlockVaultRequest,
) -> Result<VaultStatus, VaultError> {
    let envelope = load_envelope(app)?
        .ok_or_else(|| VaultError::new("vault_missing", "vault has not been created", true))?;
    let (payload, key) =
        CryptoEngine::decrypt_payload_and_key(&envelope, &request.master_password)?;
    state.unlock_vault_secret(payload, key)?;

    Ok(VaultStatus {
        state: VaultState::Unlocked,
        version: Some(envelope.version),
    })
}

pub fn lock_vault<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
) -> Result<VaultStatus, VaultError> {
    let envelope = load_envelope(app)?
        .ok_or_else(|| VaultError::new("vault_missing", "vault has not been created", true))?;
    state.lock_vault_payload()?;

    Ok(VaultStatus {
        state: VaultState::Locked,
        version: Some(envelope.version),
    })
}

pub fn change_master_password<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    request: ChangeMasterPasswordRequest,
) -> Result<VaultStatus, VaultError> {
    validate_master_password(&request.new_password)?;
    let envelope = load_envelope(app)?
        .ok_or_else(|| VaultError::new("vault_missing", "vault has not been created", true))?;
    let (payload, _) = CryptoEngine::decrypt_payload_and_key(&envelope, &request.current_password)?;
    let updated_envelope =
        encrypt_payload_with_created_at(&payload, &request.new_password, envelope.created_at)?;
    let key = key_for_envelope(&updated_envelope, &request.new_password)?;
    let store = app.store(STORE_PATH)?;
    store.set(VAULT_KEY, serde_json::to_value(&updated_envelope)?);
    store.save()?;
    state.unlock_vault_secret(payload, key)?;

    Ok(VaultStatus {
        state: VaultState::Unlocked,
        version: Some(updated_envelope.version),
    })
}

pub fn vault_status<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
) -> Result<VaultStatus, VaultError> {
    let envelope = load_envelope(app)?;
    match (envelope, state.status()?.vault) {
        (Some(envelope), VaultState::Unlocked) => Ok(VaultStatus {
            state: VaultState::Unlocked,
            version: Some(envelope.version),
        }),
        (Some(envelope), _) => {
            state.set_vault_state(VaultState::Locked)?;
            Ok(VaultStatus {
                state: VaultState::Locked,
                version: Some(envelope.version),
            })
        }
        (None, _) => {
            state.set_vault_state(VaultState::Missing)?;
            Ok(VaultStatus {
                state: VaultState::Missing,
                version: None,
            })
        }
    }
}

pub fn export_vault_envelope<R: Runtime>(app: &AppHandle<R>) -> Result<VaultEnvelope, VaultError> {
    load_envelope(app)?
        .ok_or_else(|| VaultError::new("vault_locked", "vault has not been created", true))
}

pub fn import_vault_envelope<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    envelope: VaultEnvelope,
) -> Result<VaultStatus, VaultError> {
    validate_envelope(&envelope)?;
    let store = app.store(STORE_PATH)?;
    store.set(VAULT_KEY, serde_json::to_value(&envelope)?);
    store.save()?;
    state.lock_vault_payload()?;

    Ok(VaultStatus {
        state: VaultState::Locked,
        version: Some(envelope.version),
    })
}

pub fn replace_vault_payload<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    payload: Vec<u8>,
) -> Result<VaultStatus, VaultError> {
    let envelope = load_envelope(app)?
        .ok_or_else(|| VaultError::new("vault_missing", "vault has not been created", true))?;
    let key = state
        .vault_key()?
        .ok_or_else(|| VaultError::new("vault_locked", "vault is locked", true))?;
    let updated_envelope = encrypt_payload_with_key(&payload, &key, &envelope)?;
    let store = app.store(STORE_PATH)?;
    store.set(VAULT_KEY, serde_json::to_value(&updated_envelope)?);
    store.save()?;
    state.update_vault_payload(payload)?;

    Ok(VaultStatus {
        state: VaultState::Unlocked,
        version: Some(updated_envelope.version),
    })
}

fn encrypt_payload_with_created_at(
    data: &[u8],
    master_password: &str,
    created_at: String,
) -> Result<VaultEnvelope, VaultError> {
    validate_master_password(master_password)?;

    let mut salt = [0_u8; SALT_LEN];
    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);

    let kdf = KdfParams {
        algorithm: "argon2id".to_string(),
        memory_kib: ARGON2_MEMORY_KIB,
        iterations: ARGON2_ITERATIONS,
        parallelism: ARGON2_PARALLELISM,
        salt_base64: STANDARD.encode(salt),
    };
    let key = derive_key(master_password, &salt, &kdf)?;
    let cipher = Aes256Gcm::new_from_slice(&key[..])
        .map_err(|_| VaultError::crypto_error("failed to initialize AES-256-GCM cipher"))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), data)
        .map_err(|_| VaultError::crypto_error("failed to encrypt vault payload"))?;
    let now = timestamp();

    Ok(VaultEnvelope {
        version: VAULT_VERSION,
        kdf,
        cipher: CipherPayload {
            algorithm: "aes-256-gcm".to_string(),
            nonce_base64: STANDARD.encode(nonce),
            ciphertext_base64: STANDARD.encode(ciphertext),
        },
        created_at,
        updated_at: now,
    })
}

fn encrypt_payload_with_key(
    data: &[u8],
    key: &[u8; KEY_LEN],
    existing: &VaultEnvelope,
) -> Result<VaultEnvelope, VaultError> {
    validate_envelope(existing)?;
    let salt = decode_base64(&existing.kdf.salt_base64)?;
    validate_salt_length(&salt)?;

    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| VaultError::crypto_error("failed to initialize AES-256-GCM cipher"))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), data)
        .map_err(|_| VaultError::crypto_error("failed to encrypt vault payload"))?;

    Ok(VaultEnvelope {
        version: VAULT_VERSION,
        kdf: existing.kdf.clone(),
        cipher: CipherPayload {
            algorithm: "aes-256-gcm".to_string(),
            nonce_base64: STANDARD.encode(nonce),
            ciphertext_base64: STANDARD.encode(ciphertext),
        },
        created_at: existing.created_at.clone(),
        updated_at: timestamp(),
    })
}

fn key_for_envelope(
    envelope: &VaultEnvelope,
    master_password: &str,
) -> Result<Zeroizing<[u8; KEY_LEN]>, VaultError> {
    validate_envelope(envelope)?;
    let salt = decode_base64(&envelope.kdf.salt_base64)?;
    validate_salt_length(&salt)?;
    derive_key(master_password, &salt, &envelope.kdf)
}

fn derive_key(
    master_password: &str,
    salt: &[u8],
    kdf: &KdfParams,
) -> Result<Zeroizing<[u8; KEY_LEN]>, VaultError> {
    if kdf.algorithm != "argon2id" {
        return Err(VaultError::crypto_error("unsupported vault KDF"));
    }

    let params = Params::new(kdf.memory_kib, kdf.iterations, kdf.parallelism, None)
        .map_err(|_| VaultError::crypto_error("invalid Argon2id parameters"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    argon2
        .hash_password_into(master_password.as_bytes(), salt, key.as_mut())
        .map_err(|_| VaultError::crypto_error("failed to derive vault key"))?;
    Ok(key)
}

fn validate_envelope(envelope: &VaultEnvelope) -> Result<(), VaultError> {
    if envelope.version != VAULT_VERSION {
        return Err(VaultError::crypto_error(format!(
            "unsupported vault version {}",
            envelope.version
        )));
    }

    if envelope.cipher.algorithm != "aes-256-gcm" {
        return Err(VaultError::crypto_error("unsupported vault cipher"));
    }

    Ok(())
}

fn validate_decoded_lengths(salt: &[u8], nonce: &[u8]) -> Result<(), VaultError> {
    validate_salt_length(salt)?;

    if nonce.len() != NONCE_LEN {
        return Err(VaultError::crypto_error("vault nonce has invalid length"));
    }

    Ok(())
}

fn validate_salt_length(salt: &[u8]) -> Result<(), VaultError> {
    if salt.len() != SALT_LEN {
        return Err(VaultError::crypto_error("vault salt has invalid length"));
    }

    Ok(())
}

fn validate_master_password(master_password: &str) -> Result<(), VaultError> {
    if master_password.len() < 12 {
        return Err(VaultError::new(
            "weak_password",
            "master password must be at least 12 characters",
            true,
        ));
    }

    Ok(())
}

fn load_envelope<R: Runtime>(app: &AppHandle<R>) -> Result<Option<VaultEnvelope>, VaultError> {
    let store = app.store(STORE_PATH)?;
    store
        .get(VAULT_KEY)
        .map(serde_json::from_value::<VaultEnvelope>)
        .transpose()
        .map_err(Into::into)
}

fn decode_base64(value: &str) -> Result<Vec<u8>, VaultError> {
    STANDARD
        .decode(value)
        .map_err(|_| VaultError::crypto_error("vault envelope contains invalid base64"))
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

impl VaultError {
    fn new(code: &'static str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }

    fn crypto_error(message: impl Into<String>) -> Self {
        Self::new("crypto_error", message, false)
    }

    fn invalid_password(message: impl Into<String>) -> Self {
        Self::new("invalid_password", message, true)
    }
}

impl fmt::Display for VaultError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl From<serde_json::Error> for VaultError {
    fn from(error: serde_json::Error) -> Self {
        Self::crypto_error(format!("vault envelope is invalid: {error}"))
    }
}

impl From<tauri_plugin_store::Error> for VaultError {
    fn from(error: tauri_plugin_store::Error) -> Self {
        Self::new(
            "crypto_error",
            format!("vault store is unavailable: {error}"),
            true,
        )
    }
}

impl From<crate::state::AppStateError> for VaultError {
    fn from(error: crate::state::AppStateError) -> Self {
        Self::crypto_error(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{CryptoEngine, VaultEnvelope};

    const PASSWORD: &str = "correct horse battery staple";

    #[test]
    fn encrypts_and_decrypts_payload() {
        let payload = br#"{"secret":"value"}"#;
        let envelope =
            CryptoEngine::encrypt_payload(payload, PASSWORD).expect("payload should encrypt");
        let decrypted =
            CryptoEngine::decrypt_payload(&envelope, PASSWORD).expect("payload should decrypt");

        assert_eq!(decrypted, payload);
    }

    #[test]
    fn rejects_invalid_password() {
        let envelope =
            CryptoEngine::encrypt_payload(b"secret", PASSWORD).expect("payload should encrypt");
        let error = CryptoEngine::decrypt_payload(&envelope, "wrong horse battery staple")
            .expect_err("wrong password should fail");

        assert_eq!(error.code, "invalid_password");
    }

    #[test]
    fn encrypting_same_payload_twice_uses_unique_nonce_and_ciphertext() {
        let first =
            CryptoEngine::encrypt_payload(b"secret", PASSWORD).expect("first encrypt should work");
        let second =
            CryptoEngine::encrypt_payload(b"secret", PASSWORD).expect("second encrypt should work");

        assert_ne!(first.cipher.nonce_base64, second.cipher.nonce_base64);
        assert_ne!(
            first.cipher.ciphertext_base64,
            second.cipher.ciphertext_base64
        );
    }

    #[test]
    fn rejects_unknown_vault_version() {
        let mut envelope =
            CryptoEngine::encrypt_payload(b"secret", PASSWORD).expect("payload should encrypt");
        envelope.version = 99;

        let error = CryptoEngine::decrypt_payload(&envelope, PASSWORD)
            .expect_err("unknown version should fail");

        assert_eq!(error.code, "crypto_error");
        assert!(error.message.contains("version"));
    }

    #[test]
    fn rejects_weak_master_password() {
        let error = CryptoEngine::encrypt_payload(b"secret", "short")
            .expect_err("weak password should fail");

        assert_eq!(error.code, "weak_password");
    }

    #[test]
    fn envelope_serializes_with_camel_case_fields() {
        let envelope =
            CryptoEngine::encrypt_payload(b"secret", PASSWORD).expect("payload should encrypt");
        let value = serde_json::to_value(&envelope).expect("envelope should serialize");

        assert!(value.get("createdAt").is_some());
        assert!(value["kdf"].get("memoryKiB").is_some());
        assert_round_trips(envelope);
    }

    fn assert_round_trips(envelope: VaultEnvelope) {
        let value = serde_json::to_value(&envelope).expect("envelope should serialize");
        let decoded: VaultEnvelope =
            serde_json::from_value(value).expect("envelope should deserialize");

        assert_eq!(decoded, envelope);
    }
}
