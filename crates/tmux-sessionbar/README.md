# tmux-sessionbar

Clickable session list for the tmux status bar, built in Rust.

Switch between tmux sessions by clicking on session blocks in the status bar — no existing tmux plugin supports this. Uses tmux's native `range=user` format with `run -C` to make each session block a clickable target.

## Features

- **Clickable session blocks** — click any session in the status bar to switch to it
- **Auto-updating** — session list updates instantly via tmux hooks (no polling)
- **Customizable blocks** — configure colors, layout, and which blocks appear via `config.toml`
- **Single binary** — no shell scripts, no TPM, no dependencies beyond tmux >= 3.4
- **Template system** — status bar managed as configurable blocks (session-list, hostname, datetime)

## Install

```bash
# From the workspace root
cargo build --release -p tmux-sessionbar
sudo cp target/release/tmux-sessionbar /usr/local/bin/

# Initialize
tmux-sessionbar init
```

## Usage

```bash
tmux-sessionbar init       # First-time setup
tmux-sessionbar apply      # Regenerate config after editing config.toml
tmux-sessionbar status     # Show diagnostics
```

## Configuration

Edit `~/.config/tmux-sessionbar/config.toml`:

```toml
[status]
interval = 2
position = "top"           # "top" or "bottom"
bg = "#282c34"
fg = "#abb2bf"

[status.left]
blocks = ["session-list"]

[status.right]
blocks = ["hostname", "datetime"]

[blocks.session-list]
active_fg = "#282c34"
active_bg = "#98c379"      # Green for current session
inactive_fg = "#abb2bf"
inactive_bg = "#3e4452"

[blocks.hostname]
fg = "#282c34"
bg = "#61afef"
format = " #H "

[blocks.datetime]
fg = "#282c34"
bg = "#c678dd"
format = " %H:%M "
```

After editing, run `tmux-sessionbar apply` to regenerate and reload.

## Key Bindings

| Binding | Action |
|---------|--------|
| Mouse click on session block | Switch to that session |
| `Alt+(` | Previous session |
| `Alt+)` | Next session |
| `Alt+s` | Session chooser |

## How It Works

1. `init` / `apply` generates `~/.tmux.conf` with hooks that call the binary on session events
2. On each event (session created/closed/switched/renamed), the binary sets `status-format[0]` directly
3. Each session block is wrapped in `#[range=user|session_name]`, making it a clickable region
4. Mouse clicks are bound via `run -C 'switch-client -t ...'` to expand the range value and switch sessions
