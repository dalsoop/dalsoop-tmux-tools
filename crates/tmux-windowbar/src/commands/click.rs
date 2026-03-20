use std::process::Command;

pub fn run(range: &str) -> Result<(), Box<dyn std::error::Error>> {
    if range == "_wnew_" {
        Command::new("tmux")
            .args(["new-window"])
            .status()?;
    } else if let Some(idx) = range.strip_prefix("_wk") {
        kill_window(idx)?;
    } else if let Some(idx) = range.strip_prefix("_ws") {
        // Window switch (current session)
        Command::new("tmux")
            .args(["select-window", "-t", &format!(":{idx}")])
            .status()?;
    } else if let Some(target) = range.strip_prefix("_wa") {
        // All-windows switch: target is "session.window"
        if let Some((sess, win)) = target.split_once('.') {
            Command::new("tmux")
                .args(["switch-client", "-t", &format!("={sess}:{win}")])
                .status()?;
        }
    } else if let Some(target) = range.strip_prefix("_wx") {
        // Kill window from line2: target is "session.window"
        if let Some((sess, win)) = target.split_once('.') {
            Command::new("tmux")
                .args([
                    "confirm-before",
                    "-p",
                    &format!("kill window '{sess}:{win}'? (y/n)"),
                    &format!("kill-window -t ={sess}:{win}"),
                ])
                .status()?;
        }
    } else if let Some(target) = range.strip_prefix("_px") {
        // Kill pane: target is "session.window.pane"
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            Command::new("tmux")
                .args([
                    "confirm-before",
                    "-p",
                    &format!("kill pane '{sess}.{win}.{pane}'? (y/n)"),
                    &format!("kill-pane -t ={sess}:{win}.{pane}"),
                ])
                .status()?;
        }
    } else if let Some(target) = range.strip_prefix("_pp") {
        // Pane select: target is "session.window.pane"
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            // Switch to session+window first, then select pane
            Command::new("tmux")
                .args(["switch-client", "-t", &format!("={sess}:{win}")])
                .status()?;
            Command::new("tmux")
                .args(["select-pane", "-t", &format!("={sess}:{win}.{pane}")])
                .status()?;
        }
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
