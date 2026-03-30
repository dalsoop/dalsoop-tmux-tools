//! Shared shim-installation helpers used by tmux-sessionbar and tmux-windowbar.

use anyhow::{Result, bail};
use std::path::Path;

/// Resolve the absolute path of an executable by searching `PATH`.
///
/// Returns an error if the executable is not found.
pub fn resolve_executable(name: &str) -> Result<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    bail!("required executable not found in PATH: {name}")
}

/// Escape single quotes in a shell path for use inside a `'...'` shell string.
pub fn shell_escape(path: &str) -> String {
    path.replace('\'', "'\"'\"'")
}

/// Write a thin shim shell script at `path` that delegates to `target`.
///
/// The shim is written as a POSIX `/bin/sh` script and made executable (on Unix).
pub fn write_shim(path: &Path, target: &str) -> Result<()> {
    let script = format!("#!/bin/sh\nexec '{}' \"$@\"\n", shell_escape(target));
    std::fs::write(path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Install shims for both `tmux-sessionbar` and `tmux-windowbar` into `bin_dir`.
///
/// Creates the directory if it does not exist.
pub fn install_shims(bin_dir: &Path, sessionbar_path: &str, windowbar_path: &str) -> Result<()> {
    std::fs::create_dir_all(bin_dir)?;
    write_shim(&bin_dir.join("tmux-sessionbar"), sessionbar_path)?;
    write_shim(&bin_dir.join("tmux-windowbar"), windowbar_path)?;
    Ok(())
}
