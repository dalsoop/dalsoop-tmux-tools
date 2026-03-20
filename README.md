# dalsoop-tmux-tools

A collection of tmux utilities built in Rust. Fully clickable 5-line status bar with session/window/pane management, user switching, app launcher, and system monitoring.

## Status Bar

```
Users     👤 root  👤 jeonghan  👤 dalroot-dns ...
Sessions  0  1 x  2  [+]  1:claude  2:bash x  [+]   0.5 3.2/32G  pve 00:15  🌐👤📋⚡
Windows   0.0:bash  0.1:vim x  1.0:claude x ...
Panes     2.1.0:bash  2.1.1:claude x  | -
Apps      🔐 spf  🤖 claude  🧠 codex  📊 htop  🐍 python3  🖥️ bash
```

## Features

- **Clickable everything** — click sessions, windows, panes to switch; all use tmux `range=user` for native click support
- **[+] / [x] buttons** — create and kill sessions, windows, panes with one click
- **Kill confirmation** — `confirm-before` (y/n) prompt before killing
- **5-line status bar** — Users / Sessions / Windows / Panes / Apps
- **User switching** — click a user to `sudo -iu` into their session
- **View filtering** — click a user to filter all lines to their sessions/windows/panes only
- **Pane status colors** — idle (gray), running (cyan), custom per-command via `config.toml`
- **Pane split buttons** — `[|]` horizontal, `[-]` vertical split
- **App launcher** — click to launch spf, claude, codex, htop, python3, bash
- **CPU/Memory monitor** — load average and memory usage on Sessions line
- **Layout save/restore** — save and reload window/pane layouts
- **Double-click rename** — double-click session or window to rename
- **Custom color mapping** — per-command colors in `config.toml`
- **View switcher** — toggle between All/User/Session/Compact views

## Tools

| Crate | Description |
|-------|-------------|
| [tmux-sessionbar](crates/tmux-sessionbar/) | Session management, status bar generation, CPU/memory monitor |
| [tmux-windowbar](crates/tmux-windowbar/) | Window/pane management, user switching, app launcher, layout save/restore |

## Requirements

- tmux >= 3.4 (for `range=user` support)
- Rust >= 1.70

## Install

```bash
git clone https://github.com/dalsoop/dalsoop-tmux-tools.git
cd dalsoop-tmux-tools
cargo build --release

sudo cp target/release/tmux-sessionbar target/release/tmux-windowbar /usr/local/bin/

tmux-sessionbar init
tmux-windowbar init
```

## Usage

```bash
# Session bar
tmux-sessionbar init           # First-time setup
tmux-sessionbar apply          # Regenerate config
tmux-sessionbar status         # Show diagnostics

# Window bar
tmux-windowbar init            # First-time setup
tmux-windowbar apply           # Re-apply settings

# Layout management
tmux-windowbar layout-save work    # Save current layout
tmux-windowbar layout-load work    # Restore layout
tmux-windowbar layout-list         # List saved layouts
```

## Configuration

### Session bar: `~/.config/tmux-sessionbar/config.toml`

```toml
[status]
position = "top"
interval = 2

[status.left]
blocks = ["session-list"]

[status.right]
blocks = ["hostname", "datetime"]
```

### Window bar: `~/.config/tmux-windowbar/config.toml`

```toml
[window]
show_kill_button = true
show_new_button = true

# Per-command colors for pane status
[colors.vim]
fg = "#282c34"
bg = "#e06c75"

[colors.node]
fg = "#282c34"
bg = "#98c379"

# App launcher entries
[[apps]]
emoji = "🔐"
command = "spf"
fg = "#282c34"
bg = "#c678dd"
mode = "window"
```

## Key Bindings

| Binding | Action |
|---------|--------|
| Click session/window/pane | Switch to it |
| Click [+] | Create new session/window |
| Click [x] | Kill with y/n confirmation |
| Click [|] | Split pane horizontally |
| Click [-] | Split pane vertically |
| Click user | Switch to user session + filter view |
| Click app | Launch in new window |
| Double-click session/window | Rename |
| `Alt+(` / `Alt+)` | Previous/next session |
| `Alt+s` | Session chooser |

## Testing

```bash
# Run smoke tests (28 tests)
bats tests/smoke.bats

# Or via Docker
./tests/run.sh
```

## License

MIT
