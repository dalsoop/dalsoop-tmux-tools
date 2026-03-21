# dalsoop-tmux-tools

A collection of tmux utilities built in Rust. One command (`tmux-sessionbar init`) sets up a fully clickable 5-line status bar with session/window/pane management, user switching, app launcher, plugin manager, and system monitoring.

## Status Bar

```
Users     👤 root  👤 jeonghan  👤 dalroot-dns  👤 dalroot-ops ...
Sessions  0  1 x  2  [+]  1:claude  2:bash x  [+]   0.5 3.2/32G  pve 00:15  🌐👤📋⚡
Windows   0.0:bash  0.1:vim x  1.0:claude x ...
Panes     2.1.0:bash  2.1.1:claude x  | -
Apps      🔐 spf  🤖 claude  🧠 codex  📊 htop  🐍 python3  🖥️ bash
```

## Features

- **One-step setup** — `tmux-sessionbar init` does everything: config, TPM, plugins, windowbar, bindings
- **Clickable everything** — click sessions, windows, panes to switch; uses tmux `range=user` for native click
- **[+] / [x] buttons** — create and kill sessions, windows, panes with one click
- **Kill confirmation** — `confirm-before` (y/n) bottom prompt before killing
- **5-line status bar** — Users / Sessions / Windows / Panes / Apps (always top)
- **User switching** — click a user to `sudo -iu` into their named session
- **View filtering** — click a user to filter Sessions/Windows/Panes to their data only
- **Pane status colors** — idle (gray), running (cyan), custom per-command via `config.toml`
- **Pane split buttons** — `[|]` horizontal, `[-]` vertical split
- **App launcher** — click to launch spf, claude, codex, htop, python3, bash in new window
- **CPU/Memory monitor** — load average + color-coded memory usage on Sessions line
- **Layout save/restore** — save and reload window/pane layouts
- **Double-click rename** — double-click session or window block to rename
- **Pane clear** — 🧹 button or `Alt+k` to clear screen + scrollback; cron auto-cleanup every 30 min
- **Custom color mapping** — per-command colors in `config.toml`
- **Plugin manager** — add/remove/list tmux plugins via CLI, managed in `config.toml`
- **Account sync** — sync configs + TPM + plugins to all system accounts
- **LXC compatible** — auto-creates tmux socket dir, handles PATH issues

## Tools

| Crate | Description |
|-------|-------------|
| [tmux-sessionbar](crates/tmux-sessionbar/) | Session management, status bar, CPU/memory, plugin manager, account sync |
| [tmux-windowbar](crates/tmux-windowbar/) | Window/pane management, user switching, app launcher, layout save/restore |

## Requirements

- tmux >= 3.4 (for `range=user` support)
- git (for TPM installation)
- Rust >= 1.70 (to build from source)

## Install

```bash
# Build from source
git clone https://github.com/dalsoop/dalsoop-tmux-tools.git
cd dalsoop-tmux-tools
cargo build --release

# Install binaries
sudo cp target/release/tmux-sessionbar target/release/tmux-windowbar /usr/local/bin/

# One-step setup (does everything)
tmux-sessionbar init
```

`tmux-sessionbar init` automatically:
1. Creates sessionbar + windowbar configs
2. Generates `.tmux.conf`
3. Installs TPM (Tmux Plugin Manager)
4. Installs 9 plugins (resurrect, continuum, yank, thumbs, open, logging, sensible, notify, menus)
5. Sets up mouse click/double-click bindings
6. Sets up session/window/pane event hooks
7. Symlinks binaries if `/usr/local/bin` not in PATH (LXC fix)

## Usage

```bash
# Setup
tmux-sessionbar init              # One-step full setup
tmux-sessionbar apply             # Regenerate .tmux.conf and reload
tmux-sessionbar status            # Show diagnostics
tmux-sessionbar sync              # Sync configs to all system accounts

# Plugin management
tmux-sessionbar plugin-list                           # List plugins
tmux-sessionbar plugin-add tmux-plugins/tmux-copycat  # Add + install
tmux-sessionbar plugin-rm tmux-plugins/tmux-copycat   # Remove + cleanup
tmux-sessionbar plugin-install                        # Reinstall all

# Layout management
tmux-windowbar layout-save work   # Save current window/pane layout
tmux-windowbar layout-load work   # Restore layout
tmux-windowbar layout-list        # List saved layouts
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

[general]
history_limit = 5000          # scrollback lines per pane (default: 5000)

[keybindings]
session_switch = true
pane_clear = true             # Alt+k to clear pane scrollback

[maintenance]
auto_clear = true             # periodic scrollback cleanup via cron
clear_interval = 30           # interval in minutes

# Plugins (managed via plugin-add/plugin-rm)
[[plugins]]
name = "tmux-plugins/tmux-resurrect"
enabled = true
options = ["@resurrect-capture-pane-contents 'on'"]

[[plugins]]
name = "tmux-plugins/tmux-continuum"
enabled = true
options = ["@continuum-restore 'on'", "@continuum-save-interval '15'"]
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

[colors.spf]
fg = "#282c34"
bg = "#c678dd"

# App launcher entries
[[apps]]
emoji = "🔐"
command = "spf"
fg = "#282c34"
bg = "#c678dd"
mode = "window"  # "window" or "pane"
```

## Key Bindings

### Mouse

| Binding | Action |
|---------|--------|
| Click session/window/pane | Switch to it |
| Click [+] | Create new session/window |
| Click [x] | Kill with y/n confirmation |
| Click [|] | Split pane horizontally |
| Click [-] | Split pane vertically |
| Click user | Switch to user session + filter view |
| Click app | Launch in new window |
| Click 🌐 | Show all (clear user filter) |
| Click 🧹 | Clear current pane scrollback |
| Double-click session/window | Rename |

### Keyboard

| Binding | Action |
|---------|--------|
| `Alt+(` / `Alt+)` | Previous/next session |
| `Alt+s` | Session chooser |
| `Alt+k` | Clear current pane screen + scrollback |
| `prefix + Ctrl-s` | Save session (resurrect) |
| `prefix + Ctrl-r` | Restore session (resurrect) |
| `prefix + Space` | Thumbs: highlight URLs/paths/hashes to copy |
| `prefix + \` | Menus: tmux command menu |
| `prefix + m` | Notify: alert when command finishes |

## Default Plugins

| Plugin | Purpose |
|--------|---------|
| tmux-resurrect | Save/restore sessions across restarts |
| tmux-continuum | Auto-save every 15 min + auto-restore |
| tmux-yank | System clipboard copy |
| tmux-thumbs | Highlight URLs/paths/hashes for quick copy |
| tmux-open | Open selected URL/file |
| tmux-logging | Log pane output to file |
| tmux-sensible | Optimized defaults (UTF-8, escape-time, etc) |
| tmux-notify | Notification when long command finishes |
| tmux-menus | Popup menu for tmux commands |

## Testing

```bash
# Run smoke tests (47 tests)
bats tests/smoke.bats

# Or via Docker
./tests/run.sh

# LXC integration test
pct exec <vmid> -- tmux attach -d
```

## License

MIT
