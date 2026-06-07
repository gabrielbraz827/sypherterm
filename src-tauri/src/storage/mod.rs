use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const STORE_PATH: &str = "sypherterm.local.json";
const PROFILES_KEY: &str = "profiles";
const PREFERENCES_KEY: &str = "preferences";
const MODEL_VERSION: u8 = 1;
const DEFAULT_FONT_FAMILY: &str = "JetBrains Mono, Cascadia Code, ui-monospace, monospace";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub version: u8,
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfileDraft {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: Option<String>,
    pub group_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfileSummary {
    pub id: String,
    pub version: u8,
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    pub has_credential: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileListFilters {
    pub query: Option<String>,
    pub group_id: Option<String>,
    pub tag: Option<String>,
    #[serde(default)]
    pub recent_first: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Block,
    Bar,
    Underline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub version: u8,
    pub theme: ThemePreference,
    pub font_family: String,
    pub font_size: u8,
    pub cursor_style: CursorStyle,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

impl StorageError {
    fn validation(message: impl Into<String>) -> Self {
        Self {
            code: "validation_error",
            message: message.into(),
            recoverable: true,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: message.into(),
            recoverable: true,
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "store_unavailable",
            message: message.into(),
            recoverable: true,
        }
    }

    fn corrupt(message: impl Into<String>) -> Self {
        Self {
            code: "store_corrupt",
            message: message.into(),
            recoverable: false,
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        Self::corrupt(format!("local store contains invalid data: {error}"))
    }
}

impl From<tauri_plugin_store::Error> for StorageError {
    fn from(error: tauri_plugin_store::Error) -> Self {
        Self::unavailable(format!("local store is unavailable: {error}"))
    }
}

impl From<ConnectionProfile> for ConnectionProfileSummary {
    fn from(profile: ConnectionProfile) -> Self {
        Self {
            id: profile.id,
            version: profile.version,
            name: profile.name,
            host: profile.host,
            port: profile.port,
            username: profile.username,
            group_id: profile.group_id,
            tags: profile.tags,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
            last_used_at: profile.last_used_at,
            has_credential: profile.credential_ref.is_some(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            version: MODEL_VERSION,
            theme: ThemePreference::System,
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            font_size: 13,
            cursor_style: CursorStyle::Block,
        }
    }
}

impl UserPreferences {
    fn validate(&self) -> Result<(), StorageError> {
        if self.version != MODEL_VERSION {
            return Err(StorageError::validation(format!(
                "unsupported preferences version {}",
                self.version
            )));
        }

        if self.font_family.trim().is_empty() {
            return Err(StorageError::validation("font family is required"));
        }

        if !(8..=32).contains(&self.font_size) {
            return Err(StorageError::validation(
                "font size must be between 8 and 32",
            ));
        }

        Ok(())
    }
}

impl ConnectionProfileDraft {
    fn into_profile(
        self,
        existing: Option<&ConnectionProfile>,
    ) -> Result<ConnectionProfile, StorageError> {
        let now = timestamp();
        let id = normalize_optional(self.id).unwrap_or_else(|| Uuid::new_v4().to_string());
        let name = normalize_required(self.name, "profile name")?;
        let host = normalize_host(self.host)?;
        let port = normalize_port(self.port)?;

        Ok(ConnectionProfile {
            id,
            version: MODEL_VERSION,
            name,
            host,
            port,
            username: normalize_optional(self.username),
            group_id: normalize_optional(self.group_id),
            tags: normalize_tags(self.tags.unwrap_or_default())?,
            credential_ref: normalize_optional(self.credential_ref)
                .or_else(|| existing.and_then(|profile| profile.credential_ref.clone())),
            created_at: existing
                .map(|profile| profile.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            last_used_at: existing.and_then(|profile| profile.last_used_at.clone()),
        })
    }
}

pub fn list_profiles<R: Runtime>(
    app: &AppHandle<R>,
    filters: Option<ProfileListFilters>,
) -> Result<Vec<ConnectionProfileSummary>, StorageError> {
    let filters = filters.unwrap_or_default();
    let query = normalize_optional(filters.query).map(|query| query.to_lowercase());
    let group_id = normalize_optional(filters.group_id);
    let tag = normalize_optional(filters.tag);
    let mut profiles = load_profiles(app)?;

    profiles.retain(|profile| profile_matches_filters(profile, &query, &group_id, &tag));

    if filters.recent_first {
        profiles.sort_by(|left, right| {
            right
                .last_used_at
                .cmp(&left.last_used_at)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
    } else {
        profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    }

    Ok(profiles
        .into_iter()
        .map(ConnectionProfileSummary::from)
        .collect())
}

pub fn save_profile<R: Runtime>(
    app: &AppHandle<R>,
    draft: ConnectionProfileDraft,
) -> Result<ConnectionProfile, StorageError> {
    let mut profiles = load_profiles(app)?;
    let existing_index = draft
        .id
        .as_deref()
        .and_then(|id| profiles.iter().position(|profile| profile.id == id));
    let existing = existing_index.and_then(|index| profiles.get(index));
    let profile = draft.into_profile(existing)?;

    if let Some(index) = existing_index {
        profiles[index] = profile.clone();
    } else {
        profiles.push(profile.clone());
    }

    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    save_profiles(app, &profiles)?;
    Ok(profile)
}

pub fn delete_profile<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<DeleteResult, StorageError> {
    let mut profiles = load_profiles(app)?;
    let original_len = profiles.len();
    profiles.retain(|profile| profile.id != id);

    if profiles.len() == original_len {
        return Err(StorageError::not_found("profile not found"));
    }

    save_profiles(app, &profiles)?;
    Ok(DeleteResult { deleted: true })
}

pub fn duplicate_profile<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<ConnectionProfile, StorageError> {
    let mut profiles = load_profiles(app)?;
    let Some(source) = profiles.iter().find(|profile| profile.id == id) else {
        return Err(StorageError::not_found("profile not found"));
    };
    let now = timestamp();
    let mut duplicate = source.clone();
    duplicate.id = Uuid::new_v4().to_string();
    duplicate.name = unique_copy_name(&profiles, &source.name);
    duplicate.created_at = now.clone();
    duplicate.updated_at = now;
    duplicate.last_used_at = None;

    profiles.push(duplicate.clone());
    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    save_profiles(app, &profiles)?;
    Ok(duplicate)
}

pub fn mark_profile_used<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<ConnectionProfileSummary, StorageError> {
    let mut profiles = load_profiles(app)?;
    let Some(profile) = profiles.iter_mut().find(|profile| profile.id == id) else {
        return Err(StorageError::not_found("profile not found"));
    };

    profile.last_used_at = Some(timestamp());
    let summary = ConnectionProfileSummary::from(profile.clone());
    save_profiles(app, &profiles)?;
    Ok(summary)
}

pub fn get_preferences<R: Runtime>(app: &AppHandle<R>) -> Result<UserPreferences, StorageError> {
    let store = app.store(STORE_PATH)?;
    match store.get(PREFERENCES_KEY) {
        Some(value) => {
            let preferences = serde_json::from_value::<UserPreferences>(value)?;
            preferences.validate()?;
            Ok(preferences)
        }
        None => Ok(UserPreferences::default()),
    }
}

pub fn save_preferences<R: Runtime>(
    app: &AppHandle<R>,
    preferences: UserPreferences,
) -> Result<UserPreferences, StorageError> {
    preferences.validate()?;
    let store = app.store(STORE_PATH)?;
    store.set(PREFERENCES_KEY, serde_json::to_value(&preferences)?);
    store.save()?;
    Ok(preferences)
}

fn load_profiles<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<ConnectionProfile>, StorageError> {
    let store = app.store(STORE_PATH)?;
    match store.get(PROFILES_KEY) {
        Some(value) => {
            let profiles = serde_json::from_value::<Vec<ConnectionProfile>>(value)?;
            validate_loaded_profiles(&profiles)?;
            Ok(profiles)
        }
        None => Ok(Vec::new()),
    }
}

fn save_profiles<R: Runtime>(
    app: &AppHandle<R>,
    profiles: &[ConnectionProfile],
) -> Result<(), StorageError> {
    let store = app.store(STORE_PATH)?;
    store.set(PROFILES_KEY, serde_json::to_value(profiles)?);
    store.save()?;
    Ok(())
}

fn validate_loaded_profiles(profiles: &[ConnectionProfile]) -> Result<(), StorageError> {
    for profile in profiles {
        if profile.version != MODEL_VERSION {
            return Err(StorageError::corrupt(format!(
                "unsupported profile version {}",
                profile.version
            )));
        }

        normalize_required(profile.name.clone(), "profile name")?;
        normalize_host(profile.host.clone())?;
        normalize_port(i64::from(profile.port))?;
    }

    Ok(())
}

fn profile_matches_filters(
    profile: &ConnectionProfile,
    query: &Option<String>,
    group_id: &Option<String>,
    tag: &Option<String>,
) -> bool {
    if let Some(group_id) = group_id {
        if profile.group_id.as_ref() != Some(group_id) {
            return false;
        }
    }

    if let Some(tag) = tag {
        if !profile.tags.iter().any(|profile_tag| profile_tag == tag) {
            return false;
        }
    }

    let Some(query) = query else {
        return true;
    };

    profile.name.to_lowercase().contains(query)
        || profile.host.to_lowercase().contains(query)
        || profile
            .username
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || profile
            .group_id
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || profile
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

fn unique_copy_name(profiles: &[ConnectionProfile], source_name: &str) -> String {
    let base = format!("{source_name} copy");
    if profiles.iter().all(|profile| profile.name != base) {
        return base;
    }

    for suffix in 2.. {
        let candidate = format!("{base} {suffix}");
        if profiles.iter().all(|profile| profile.name != candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded suffix search should always return")
}

fn normalize_required(value: String, field_name: &str) -> Result<String, StorageError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(StorageError::validation(format!(
            "{field_name} is required"
        )));
    }
    Ok(value)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_host(value: String) -> Result<String, StorageError> {
    let host = normalize_required(value, "host")?;
    if host.chars().any(char::is_whitespace) {
        return Err(StorageError::validation("host cannot contain whitespace"));
    }
    Ok(host)
}

fn normalize_port(value: i64) -> Result<u16, StorageError> {
    if !(1..=65_535).contains(&value) {
        return Err(StorageError::validation("port must be between 1 and 65535"));
    }
    Ok(value as u16)
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>, StorageError> {
    let mut normalized = BTreeSet::new();

    for tag in tags {
        let tag = tag.trim().to_string();
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 32 {
            return Err(StorageError::validation(
                "tags must be 32 characters or fewer",
            ));
        }
        normalized.insert(tag);
    }

    Ok(normalized.into_iter().collect())
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_tags, profile_matches_filters, unique_copy_name, ConnectionProfile,
        ConnectionProfileDraft, CursorStyle, StorageError, ThemePreference, UserPreferences,
    };

    fn valid_draft() -> ConnectionProfileDraft {
        ConnectionProfileDraft {
            id: None,
            name: " Production ".to_string(),
            host: " example.com ".to_string(),
            port: 22,
            username: Some(" deploy ".to_string()),
            group_id: None,
            tags: Some(vec![
                "linux".to_string(),
                " prod ".to_string(),
                "linux".to_string(),
            ]),
            credential_ref: Some(" vault-key ".to_string()),
        }
    }

    #[test]
    fn validates_and_normalizes_profile_draft() {
        let profile = valid_draft()
            .into_profile(None)
            .expect("valid draft should produce a profile");

        assert_eq!(profile.version, 1);
        assert_eq!(profile.name, "Production");
        assert_eq!(profile.host, "example.com");
        assert_eq!(profile.port, 22);
        assert_eq!(profile.username.as_deref(), Some("deploy"));
        assert_eq!(profile.tags, vec!["linux".to_string(), "prod".to_string()]);
        assert_eq!(profile.credential_ref.as_deref(), Some("vault-key"));
    }

    #[test]
    fn rejects_invalid_port() {
        let mut draft = valid_draft();
        draft.port = 70_000;

        let error = draft
            .into_profile(None)
            .expect_err("invalid port should be rejected");

        assert_eq!(error.code, "validation_error");
        assert!(error.message.contains("port"));
    }

    #[test]
    fn rejects_host_with_whitespace() {
        let mut draft = valid_draft();
        draft.host = "bad host".to_string();

        let error = draft
            .into_profile(None)
            .expect_err("host with whitespace should be rejected");

        assert_eq!(error.code, "validation_error");
    }

    #[test]
    fn preferences_default_to_safe_values() {
        let preferences = UserPreferences::default();

        assert_eq!(preferences.version, 1);
        assert_eq!(preferences.theme, ThemePreference::System);
        assert_eq!(preferences.font_size, 13);
        assert_eq!(preferences.cursor_style, CursorStyle::Block);
    }

    #[test]
    fn preferences_reject_small_font_size() {
        let preferences = UserPreferences {
            font_size: 7,
            ..UserPreferences::default()
        };

        let error = preferences
            .validate()
            .expect_err("small font size should be rejected");

        assert_eq!(error.code, "validation_error");
    }

    #[test]
    fn tags_are_trimmed_deduplicated_and_sorted() -> Result<(), StorageError> {
        let tags = normalize_tags(vec![
            " zeta ".to_string(),
            "alpha".to_string(),
            "zeta".to_string(),
        ])?;

        assert_eq!(tags, vec!["alpha".to_string(), "zeta".to_string()]);
        Ok(())
    }

    #[test]
    fn editing_profile_preserves_existing_credential_ref_when_omitted() {
        let existing = valid_draft()
            .into_profile(None)
            .expect("valid draft should produce profile");
        let mut draft = valid_draft();
        draft.id = Some(existing.id.clone());
        draft.credential_ref = None;

        let updated = draft
            .into_profile(Some(&existing))
            .expect("valid edit should preserve credential ref");

        assert_eq!(updated.credential_ref.as_deref(), Some("vault-key"));
    }

    #[test]
    fn profile_filters_match_query_group_and_tag() {
        let profile = valid_draft()
            .into_profile(None)
            .expect("valid draft should produce profile");
        let query = Some("prod".to_string());
        let group = None;
        let tag = Some("linux".to_string());

        assert!(profile_matches_filters(&profile, &query, &group, &tag));
    }

    #[test]
    fn copy_name_does_not_collide_with_existing_profiles() {
        let profile = ConnectionProfile {
            id: "one".to_string(),
            version: 1,
            name: "Production".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: None,
            group_id: None,
            tags: Vec::new(),
            credential_ref: None,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            last_used_at: None,
        };
        let mut copy = profile.clone();
        copy.id = "two".to_string();
        copy.name = "Production copy".to_string();

        assert_eq!(
            unique_copy_name(&[profile, copy], "Production"),
            "Production copy 2"
        );
    }
}
