use std::process::Command;

pub fn run(range: &str) -> Result<(), Box<dyn std::error::Error>> {
    if range == "_wnew_" {
        Command::new("tmux")
            .args(["new-window"])
            .status()?;
    } else if let Some(idx) = range.strip_prefix("_wk") {
        kill_window(idx)?;
    } else if let Some(idx) = range.strip_prefix("_ws") {
        // Window switch
        Command::new("tmux")
            .args(["select-window", "-t", &format!(":{idx}")])
            .status()?;
    } else {
        // Not our range, exit with error so sessionbar can handle it
        return Err(format!("unknown range: {range}").into());
    }

    Ok(())
}

fn kill_window(idx: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Count windows
    let output = Command::new("tmux")
        .args(["list-windows", "-F", "#{window_index}"])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    let windows: Vec<&str> = output_str
        .lines()
        .filter(|l| !l.is_empty())
        .collect();

    if windows.len() <= 1 {
        Command::new("tmux")
            .args(["display-message", "cannot kill last window"])
            .status()?;
        return Ok(());
    }

    Command::new("tmux")
        .args([
            "confirm-before",
            "-p",
            &format!("kill window '{idx}'? (y/n)"),
            &format!("kill-window -t :{idx}"),
        ])
        .status()?;

    Ok(())
}
