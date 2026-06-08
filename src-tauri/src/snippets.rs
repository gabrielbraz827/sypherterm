use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Runtime};
use uuid::Uuid;

use crate::crypto::{replace_vault_payload, VaultError};
use crate::state::{AppState, AppStateError};
use crate::storage::DeleteResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub version: u8,
    pub name: String,
    pub body: String,
    pub tags: Vec<String>,
    pub variables: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetSummary {
    pub id: String,
    pub version: u8,
    pub name: String,
    pub tags: Vec<String>,
    pub variables: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetDraft {
    pub id: Option<String>,
    pub name: String,
    pub body: String,
    pub tags: Option<Vec<String>>,
    pub variables: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetFilters {
    pub query: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VaultDocument {
    #[serde(default)]
    snippets: Vec<Snippet>,
}

impl From<&Snippet> for SnippetSummary {
    fn from(snippet: &Snippet) -> Self {
        Self {
            id: snippet.id.clone(),
            version: snippet.version,
            name: snippet.name.clone(),
            tags: snippet.tags.clone(),
            variables: snippet.variables.clone(),
            created_at: snippet.created_at.clone(),
            updated_at: snippet.updated_at.clone(),
        }
    }
}

pub fn list_snippets(
    state: &AppState,
    filters: Option<SnippetFilters>,
) -> Result<Vec<SnippetSummary>, SnippetError> {
    let document = read_document(state)?;
    let filters = filters.unwrap_or_default();
    let query = filters.query.map(|value| value.trim().to_lowercase());
    let tag = filters.tag.map(|value| value.trim().to_lowercase());

    Ok(document
        .snippets
        .iter()
        .filter(|snippet| {
            let matches_query = match query.as_ref() {
                Some(query) => {
                    query.is_empty()
                        || snippet.name.to_lowercase().contains(query)
                        || snippet
                            .tags
                            .iter()
                            .any(|snippet_tag| snippet_tag.to_lowercase().contains(query))
                }
                None => true,
            };
            let matches_tag = match tag.as_ref() {
                Some(tag) => {
                    tag.is_empty()
                        || snippet
                            .tags
                            .iter()
                            .any(|snippet_tag| snippet_tag.to_lowercase() == tag.as_str())
                }
                None => true,
            };

            matches_query && matches_tag
        })
        .map(SnippetSummary::from)
        .collect())
}

pub fn get_snippet(state: &AppState, id: String) -> Result<Snippet, SnippetError> {
    let document = read_document(state)?;
    document
        .snippets
        .into_iter()
        .find(|snippet| snippet.id == id)
        .ok_or_else(|| SnippetError::new("not_found", "snippet was not found", true))
}

pub fn save_snippet<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    draft: SnippetDraft,
) -> Result<Snippet, SnippetError> {
    let mut document = read_document(state)?;
    let name = validate_name(&draft.name)?;
    let body = validate_body(&draft.body)?;
    let tags = normalize_words(draft.tags.unwrap_or_default());
    let variables = match draft.variables {
        Some(variables) => normalize_words(variables),
        None => extract_variables(&body),
    };
    let now = timestamp();

    let snippet = if let Some(id) = draft.id.filter(|id| !id.trim().is_empty()) {
        let existing = document
            .snippets
            .iter_mut()
            .find(|snippet| snippet.id == id)
            .ok_or_else(|| SnippetError::new("not_found", "snippet was not found", true))?;
        existing.name = name;
        existing.body = body;
        existing.tags = tags;
        existing.variables = variables;
        existing.updated_at = now;
        existing.clone()
    } else {
        Snippet {
            id: Uuid::new_v4().to_string(),
            version: 1,
            name,
            body,
            tags,
            variables,
            created_at: now.clone(),
            updated_at: now,
        }
    };

    if !document
        .snippets
        .iter()
        .any(|stored| stored.id == snippet.id)
    {
        document.snippets.push(snippet.clone());
    }

    document
        .snippets
        .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    write_document(app, state, &document)?;

    Ok(snippet)
}

pub fn delete_snippet<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    id: String,
) -> Result<DeleteResult, SnippetError> {
    let mut document = read_document(state)?;
    let original_len = document.snippets.len();
    document.snippets.retain(|snippet| snippet.id != id);

    if document.snippets.len() == original_len {
        return Err(SnippetError::new(
            "not_found",
            "snippet was not found",
            true,
        ));
    }

    write_document(app, state, &document)?;
    Ok(DeleteResult { deleted: true })
}

fn read_document(state: &AppState) -> Result<VaultDocument, SnippetError> {
    let payload = state
        .vault_payload()?
        .ok_or_else(|| SnippetError::new("vault_locked", "vault is locked", true))?;
    serde_json::from_slice(&payload).map_err(SnippetError::from)
}

fn write_document<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    document: &VaultDocument,
) -> Result<(), SnippetError> {
    let payload = serde_json::to_vec(document)?;
    replace_vault_payload(app, state, payload)?;
    Ok(())
}

fn validate_name(name: &str) -> Result<String, SnippetError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(SnippetError::validation("snippet name is required"));
    }
    if name.len() > 120 {
        return Err(SnippetError::validation(
            "snippet name must be 120 characters or less",
        ));
    }

    Ok(name.to_string())
}

fn validate_body(body: &str) -> Result<String, SnippetError> {
    if body.trim().is_empty() {
        return Err(SnippetError::validation("snippet body is required"));
    }
    if body.len() > 65_536 {
        return Err(SnippetError::validation(
            "snippet body must be 65536 characters or less",
        ));
    }

    Ok(body.to_string())
}

fn normalize_words(words: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for word in words {
        let word = word.trim().to_lowercase();
        if word.is_empty() || !seen.insert(word.clone()) {
            continue;
        }
        normalized.push(word);
    }

    normalized.sort();
    normalized
}

fn extract_variables(body: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let bytes = body.as_bytes();
    let mut index = 0;

    while index + 3 < bytes.len() {
        if bytes[index] != b'{' || bytes[index + 1] != b'{' {
            index += 1;
            continue;
        }

        let start = index + 2;
        let mut end = start;
        while end + 1 < bytes.len() && (bytes[end] != b'}' || bytes[end + 1] != b'}') {
            end += 1;
        }

        if end + 1 >= bytes.len() {
            break;
        }

        let candidate = body[start..end].trim();
        if is_variable_name(candidate) {
            variables.push(candidate.to_lowercase());
        }
        index = end + 2;
    }

    normalize_words(variables)
}

fn is_variable_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

impl SnippetError {
    fn new(code: &'static str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }

    fn validation(message: impl Into<String>) -> Self {
        Self::new("validation_error", message, true)
    }
}

impl fmt::Display for SnippetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SnippetError {}

impl From<AppStateError> for SnippetError {
    fn from(error: AppStateError) -> Self {
        Self::new("internal_error", error.to_string(), false)
    }
}

impl From<VaultError> for SnippetError {
    fn from(error: VaultError) -> Self {
        Self::new(error.code, error.message, error.recoverable)
    }
}

impl From<serde_json::Error> for SnippetError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("vault_payload_error", error.to_string(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_variables, list_snippets, normalize_words, Snippet, SnippetFilters};
    use crate::state::AppState;

    #[test]
    fn extracts_template_variables() {
        let variables = extract_variables("ssh {{host}} -l {{ user_name }} && echo {{HOST}}");

        assert_eq!(variables, vec!["host", "user_name"]);
    }

    #[test]
    fn ignores_invalid_template_variables() {
        let variables = extract_variables("echo {{ }} {{user.name}} {{valid-name}}");

        assert_eq!(variables, vec!["valid-name"]);
    }

    #[test]
    fn normalizes_tags_and_variables() {
        let words = normalize_words(vec![
            " SSH ".to_string(),
            "deploy".to_string(),
            "ssh".to_string(),
            "".to_string(),
        ]);

        assert_eq!(words, vec!["deploy", "ssh"]);
    }

    #[test]
    fn list_requires_unlocked_vault() {
        let state = AppState::default();
        let error = list_snippets(&state, None).expect_err("locked vault should fail");

        assert_eq!(error.code, "vault_locked");
    }

    #[test]
    fn filters_snippets_by_name_and_tag() {
        let state = AppState::default();
        let document = serde_json::json!({
            "snippets": [
                snippet_json("one", "Deploy API", ["deploy", "prod"]),
                snippet_json("two", "List logs", ["logs"])
            ]
        });
        state
            .unlock_vault_payload(serde_json::to_vec(&document).expect("document should encode"))
            .expect("vault should unlock for tests");

        let results = list_snippets(
            &state,
            Some(SnippetFilters {
                query: Some("deploy".to_string()),
                tag: Some("prod".to_string()),
            }),
        )
        .expect("list should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "one");
    }

    fn snippet_json<const N: usize>(id: &str, name: &str, tags: [&str; N]) -> serde_json::Value {
        serde_json::to_value(Snippet {
            id: id.to_string(),
            version: 1,
            name: name.to_string(),
            body: "echo ok".to_string(),
            tags: tags.into_iter().map(str::to_string).collect(),
            variables: Vec::new(),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        })
        .expect("snippet should encode")
    }
}
