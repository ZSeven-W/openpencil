//! Reuse the user's current agy login without sharing its mutable app data.

use std::fs;
use std::io;
use std::path::Path;

/// agy 1.3 stores OAuth in its data directory rather than macOS Keychain.
/// Copy only that login into the private turn; IsolatedTurn removes the whole
/// directory on drop. Never put credentials in prompt text, logs or artifacts.
pub(super) fn copy_antigravity_oauth(
    host_home: Option<&Path>,
    private_data: &Path,
) -> io::Result<()> {
    let Some(home) = host_home else {
        return Ok(());
    };
    let source = home.join(".gemini/antigravity-cli/antigravity-oauth-token");
    if !source.is_file() {
        return Ok(());
    }
    let destination = private_data.join("antigravity-oauth-token");
    fs::copy(source, &destination)?;
    super::set_private_file(&destination)
}
