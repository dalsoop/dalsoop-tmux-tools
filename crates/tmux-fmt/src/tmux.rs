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

/// Build a `Command` for tmux, respecting `TMUX_SOCKET` env var.
/// If `TMUX_SOCKET` is set, prepends `-L <socket>` to use an isolated server.
fn tmux_cmd(args: &[&str]) -> Command {
    let mut cmd = Command::new("tmux");
    if let Ok(socket) = std::env::var("TMUX_SOCKET")
        && !socket.is_empty()
    {
        cmd.args(["-L", &socket]);
    }
    cmd.args(args);
    cmd
}

/// Run a tmux command and return trimmed stdout.
///
/// Fails if tmux is not found or exits with an error.
pub fn query(args: &[&str]) -> Result<String> {
    let output = tmux_cmd(args)
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
    let status = tmux_cmd(args)
        .status()
        .with_context(|| format!("tmux {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("tmux {} exited with {}", args.join(" "), status);
    }

    Ok(())
}

/// Run a tmux command, ignoring any failure.
pub fn run_quiet(args: &[&str]) {
    let _ = tmux_cmd(args).status();
}

/// Run a tmux command and return the full output (stdout lines).
pub fn lines(args: &[&str]) -> Result<Vec<String>> {
    let out = query(args)?;
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

// ── Sanitization ──

/// Sanitize a string for safe embedding in tmux command strings.
///
/// Removes characters that could break tmux command parsing or enable
/// command injection: quotes, backslashes, semicolons, `#`, backticks,
/// `$`, curly braces, newlines, and null bytes.
///
/// Curly braces are filtered because tmux interprets `#{...}` as format
/// strings; user input containing `{` or `}` could be interpreted as a
/// tmux format expression (e.g. `#{shell-command:...}`) if passed to a
/// tmux command without sanitization.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !matches!(
                c,
                '\'' | '"' | '\\' | ';' | '#' | '`' | '$' | '{' | '}' | '\n' | '\r' | '\0'
            )
        })
        .collect()
}

// ── Confirm dialog ──

const CONFIRM_FILE: &str = "/tmp/tmux-pending-confirm.conf";

/// Write a confirm-before prompt to the shared confirm file.
///
/// When the user answers "y", tmux executes `cmd`.
/// Both `title` and `cmd` are sanitized before embedding.
pub fn confirm(title: &str, cmd: &str) -> Result<()> {
    let safe_title = sanitize(title);
    let safe_cmd = sanitize(cmd);
    let content = format!("confirm-before -p \"{safe_title} (y/n)\" \"{safe_cmd}\"");
    std::fs::write(CONFIRM_FILE, content)?;
    Ok(())
}

/// Write a confirm-before prompt with a raw (pre-built) command string.
///
/// Use this when `cmd` contains tmux sub-commands (e.g. `run-shell '...'`)
/// that should not be sanitized. `title` is still sanitized.
pub fn confirm_raw(title: &str, cmd: &str) -> Result<()> {
    let safe_title = sanitize(title);
    let content = format!("confirm-before -p \"{safe_title} (y/n)\" \"{cmd}\"");
    std::fs::write(CONFIRM_FILE, content)?;
    Ok(())
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
        let result = query_or(&["show", "-gv", "@nonexistent_var_xyz"], "default");
        assert!(!result.is_empty() || result == "default");
    }

    #[test]
    fn sanitize_removes_dangerous_chars() {
        assert_eq!(sanitize("hello"), "hello");
        assert_eq!(sanitize("it's"), "its");
        assert_eq!(sanitize(r#"a"b"#), "ab");
        assert_eq!(sanitize("a\\b"), "ab");
        assert_eq!(sanitize("a;b"), "ab");
        assert_eq!(sanitize("a#b"), "ab");
        assert_eq!(sanitize("a`whoami`"), "awhoami");
        assert_eq!(sanitize("a$(cmd)"), "a(cmd)");
        assert_eq!(sanitize("#{shell-command:ls}"), "shell-command:ls");
        assert_eq!(sanitize("a{b}c"), "abc");
        assert_eq!(sanitize("a\nb"), "ab");
        assert_eq!(sanitize("a\rb"), "ab");
        assert_eq!(sanitize("a\0b"), "ab");
    }

    #[test]
    fn sanitize_preserves_safe_chars() {
        assert_eq!(sanitize("hello-world_123"), "hello-world_123");
        assert_eq!(sanitize("session (1)"), "session (1)");
        assert_eq!(sanitize("user@host"), "user@host");
        assert_eq!(sanitize("path/to/file"), "path/to/file");
    }

    // ── Domain invariant tests ──

    /// No dangerous character survives sanitize().
    /// These are the characters that could enable shell injection, tmux format
    /// injection, or quote escaping if they leak through.
    const DANGEROUS_CHARS: &[char] = &[
        '\'', '"', '\\', ';', '#', '`', '$', '{', '}', '\n', '\r', '\0',
    ];

    #[test]
    fn domain_sanitize_shell_injection_dollar_paren() {
        let result = sanitize("$(rm -rf /)");
        assert!(!result.contains('$'), "$ survived: {result}");
        assert!(!result.contains("$("), "$( survived: {result}");
    }

    #[test]
    fn domain_sanitize_shell_injection_backtick() {
        let result = sanitize("`whoami`");
        assert!(!result.contains('`'), "backtick survived: {result}");
    }

    #[test]
    fn domain_sanitize_shell_injection_semicolon() {
        let result = sanitize("; rm -rf /");
        assert!(!result.contains(';'), "semicolon survived: {result}");
    }

    #[test]
    fn domain_sanitize_tmux_format_injection_hash_brace() {
        let result = sanitize("#{command}");
        assert!(!result.contains('#'), "# survived: {result}");
        assert!(!result.contains('{'), "{{ survived: {result}");
        assert!(!result.contains('}'), "}} survived: {result}");
    }

    #[test]
    fn domain_sanitize_tmux_format_injection_hash_paren() {
        let result = sanitize("#(shell-cmd)");
        assert!(!result.contains('#'), "# survived in #(): {result}");
    }

    #[test]
    fn domain_sanitize_quote_escaping() {
        for ch in &['\'', '"', '\\'] {
            let input = format!("a{ch}b");
            let result = sanitize(&input);
            assert!(!result.contains(*ch), "{ch:?} survived: {result}");
        }
    }

    #[test]
    fn domain_sanitize_null_and_newlines() {
        for ch in &['\0', '\n', '\r'] {
            let input = format!("a{ch}b");
            let result = sanitize(&input);
            assert!(!result.contains(*ch), "{ch:?} survived in output");
        }
    }

    #[test]
    fn domain_sanitize_combined_attack() {
        let attack = "'; $(rm -rf /) #";
        let result = sanitize(attack);
        for ch in DANGEROUS_CHARS {
            assert!(
                !result.contains(*ch),
                "{ch:?} survived combined attack: {result}"
            );
        }
    }

    #[test]
    fn domain_sanitize_all_dangerous_chars_removed() {
        // Build a string containing every dangerous char
        let poison: String = DANGEROUS_CHARS.iter().collect();
        let result = sanitize(&poison);
        for ch in DANGEROUS_CHARS {
            assert!(
                !result.contains(*ch),
                "{ch:?} survived sanitize of all-dangerous string"
            );
        }
        assert!(
            result.is_empty(),
            "expected empty after removing all dangerous chars, got: {result}"
        );
    }

    #[test]
    fn domain_sanitize_idempotent() {
        let inputs = [
            "hello",
            "$(cmd)",
            "`whoami`",
            "; rm -rf /",
            "#{command}",
            "#(shell-cmd)",
            "'\"\\;#`${}",
            "a\nb\rc\0d",
            "'; $(rm -rf /) #",
            "normal text with spaces",
            "",
            "🔐 unicode 한글",
        ];
        for input in &inputs {
            let once = sanitize(input);
            let twice = sanitize(&once);
            assert_eq!(once, twice, "sanitize not idempotent for input: {input:?}");
        }
    }
}
