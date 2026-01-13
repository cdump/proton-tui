//! Token storage for persisting authentication tokens between runs.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::auth::AuthResult;

/// Stored tokens that can be saved/loaded from disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub uid: String,
    pub access_token: String,
    pub refresh_token: String,
}

impl From<AuthResult> for StoredTokens {
    fn from(auth: AuthResult) -> Self {
        Self {
            uid: auth.uid,
            access_token: auth.access_token,
            refresh_token: auth.refresh_token,
        }
    }
}

/// Get the path to the tokens file
fn get_tokens_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("proton-tui");

    fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

    Ok(config_dir.join("tokens.json"))
}

/// Load tokens from disk
pub fn load_tokens() -> Result<Option<StoredTokens>> {
    let path = get_tokens_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path).context("Failed to read tokens file")?;

    let tokens: StoredTokens =
        serde_json::from_str(&contents).context("Failed to parse tokens file")?;

    Ok(Some(tokens))
}

/// Save tokens to disk
pub fn save_tokens(tokens: &StoredTokens) -> Result<()> {
    let path = get_tokens_path()?;

    let contents = serde_json::to_string_pretty(tokens).context("Failed to serialize tokens")?;

    fs::write(&path, contents).context("Failed to write tokens file")?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms).context("Failed to set tokens file permissions")?;
    }

    Ok(())
}

/// Delete saved tokens
pub fn delete_tokens() -> Result<()> {
    let path = get_tokens_path()?;

    if path.exists() {
        fs::remove_file(&path).context("Failed to delete tokens file")?;
    }

    Ok(())
}
