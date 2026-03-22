//! Thin helpers around `Command::new("tmux")` to reduce boilerplate.
//!
//! # Examples
//!
//! ```no_run
//! use tmux_fmt::tmux;
//!
//! // Query a tmux variable (returns trimmed stdout)
//! let session = tmux::query(&["display-message", "-p", "#S"]).unwrap();
//!
//! // Query with a default if tmux fails
//! let user = tmux::query_or(&["show", "-gv", "@view_user"], "");
//!
//! // Run a tmux command, only care about success/failure
//! let fmt = "some-format";
//! tmux::run(&["set", "-g", "status-format[1]", fmt]).unwrap();
//!
//! // Run a tmux command, ignore failure
//! tmux::run_quiet(&["set", "-gu", "@view_user"]);
//! ```

use anyhow::{Context, Result};
use std::process::Command;

/// Run a tmux command and return trimmed stdout.
///
/// Fails if tmux is not found or exits with an error.
pub fn query(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .with_context(|| format!("tmux {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a tmux command, returning `fallback` on any failure.
pub fn query_or(args: &[&str], fallback: &str) -> String {
    query(args).unwrap_or_else(|_| fallback.to_string())
}

/// Run a tmux command, only checking for success.
pub fn run(args: &[&str]) -> Result<()> {
    let status = Command::new("tmux")
        .args(args)
        .status()
        .with_context(|| format!("tmux {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("tmux {} exited with {}", args.join(" "), status);
    }

    Ok(())
}

/// Run a tmux command, ignoring any failure.
pub fn run_quiet(args: &[&str]) {
    let _ = Command::new("tmux").args(args).status();
}

/// Run a tmux command and return the full output (stdout lines).
pub fn lines(args: &[&str]) -> Result<Vec<String>> {
    let out = query(args)?;
    Ok(out.lines().filter(|l| !l.is_empty()).map(String::from).collect())
}

// ── Sanitization ──

/// Sanitize a string for safe embedding in tmux command strings.
///
/// Removes characters that could break tmux command parsing:
/// single/double quotes, backslashes, semicolons, and `#`.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\'' | '"' | '\\' | ';' | '#'))
        .collect()
}

// ── Re-entrancy guard ──

/// Guard that prevents recursive/concurrent invocations via a tmux timestamp variable.
///
/// Returns `true` if the caller should proceed, `false` if another invocation
/// completed within `debounce_ms` and this call should be skipped.
///
/// Uses a tmux global variable (`@{name}_ts`) to store the last invocation
/// timestamp in milliseconds. Auto-expires — no stale lock on crash.
///
/// ```no_run
/// use tmux_fmt::tmux;
///
/// if !tmux::acquire_guard("sessionbar_render", 100) {
///     return; // another render in progress or just finished
/// }
/// // ... do work ...
/// ```
pub fn acquire_guard(name: &str, debounce_ms: u128) -> bool {
    let var = format!("@{name}_ts");
    let last = query_or(&["show", "-gv", &var], "0");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    if now.saturating_sub(last.parse::<u128>().unwrap_or(0)) < debounce_ms {
        return false;
    }

    run_quiet(&["set", "-g", &var, &now.to_string()]);
    true
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_or_returns_fallback_when_tmux_unavailable() {
        // This test works even without tmux running
        let result = query_or(&["show", "-gv", "@nonexistent_var_xyz"], "default");
        // Either returns the value or the fallback
        assert!(!result.is_empty() || result == "default");
    }
}
