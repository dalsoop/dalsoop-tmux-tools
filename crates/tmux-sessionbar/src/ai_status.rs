use anyhow::Result;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub const SCRIPT_NAME: &str = "tmux-ai-status";
const SCRIPT: &str = include_str!("../assets/tmux-ai-status.py");

pub fn install(bin_dir: &Path) -> Result<()> {
    fs::create_dir_all(bin_dir)?;
    let script_path = bin_dir.join(SCRIPT_NAME);
    fs::write(&script_path, SCRIPT)?;
    #[cfg(unix)]
    {
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
