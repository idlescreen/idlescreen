// SPDX-License-Identifier: MIT

use landlock::Access;
use landlock::RulesetAttr;
use landlock::{ABI, AccessFs, Ruleset};

/// Enforces a strict Landlock filesystem sandbox on the current process,
/// locking down all filesystem access (read, write, execute).
///
/// Skipped when `TRANCE_DISABLE_SANDBOX=1` (offline export / `render`).
pub fn enforce_sandbox() -> Result<(), String> {
    if std::env::var_os("TRANCE_DISABLE_SANDBOX").as_deref() == Some(std::ffi::OsStr::new("1")) {
        tracing::info!("Landlock sandbox skipped (TRANCE_DISABLE_SANDBOX=1)");
        return Ok(());
    }
    // Use ABI::V1 which is the baseline Landlock version supported since 5.13.
    // We handle all filesystem access rights to ensure a total lockdown.
    let ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V1))
        .map_err(|e| format!("Failed to initialize ruleset: {e}"))?;

    let ruleset = ruleset
        .create()
        .map_err(|e| format!("Failed to create ruleset: {e}"))?;

    let status = ruleset
        .restrict_self()
        .map_err(|e| format!("Failed to enforce Landlock sandbox: {e}"))?;

    tracing::info!("Landlock filesystem sandbox enforced: {:?}", status);
    Ok(())
}
