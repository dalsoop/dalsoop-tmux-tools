# dalsoop-tmux-tools

A collection of tmux utilities built in Rust. One command (`tmux-sessionbar init`) sets up a fully clickable 5-line status bar with session/window/pane management, user switching, app launcher, plugin manager, and system monitoring.

## Status Bar

```
Users     👤 root  🖥️ pve                  ⊞  ⤢  ↻      | -      ⚙
Sessions  0  1 x  2  [+]  1:claude  2:bash x  [+]   0.5 3.2/32G  pve 00:15
Windows   0.0:bash  0.1:vim x  1.0:claude x ...
Panes     2.1.0:bash  2.1.1:claude x
Apps      🔐 spf  📊 htop
```

Users 줄 오른쪽: layout / zoom / rotate (\`⊞ ⤢ ↻\`) + split-h/v (\`| -\`) + 설정 TUI(\`⚙\`) — 톱니바퀴 클릭 시 `tmux-topbar` TUI 가 단일 윈도우로 뜸.

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
- **AI usage overlay** — per-pane / per-window `tok`, `msg`, `use` counters from Claude/Codex local session logs
- **App launcher** — click to launch spf, claude, codex, htop, python3, bash in new window
- **CPU/Memory monitor** — load average + color-coded memory usage on Sessions line
- **Layout save/restore** — save and reload window/pane layouts
- **Double-click rename** — double-click session or window block to rename
- **Pane clear** — 🧹 button or `Alt+k` to clear screen + scrollback; cron auto-cleanup every 30 min
- **Custom color mapping** — per-command colors in `config.toml`
- **Plugin manager** — add/remove/list tmux plugins via CLI, managed in `config.toml`
- **Account sync** — sync configs + TPM + plugins to all system accounts
- **LXC compatible** — auto-creates tmux socket dir and writes stable home-local command shims
- **Re-entrancy guard** — timestamp-based debounce prevents recursive hook invocations
- **Proxmox 로컬 단락 (v0.2)** — 등록 호스트가 127.0.0.1/localhost/현재 hostname/LAN IP 이면 `ssh` 래퍼를 생략하고 `sh -c` 로 바로 실행. TUI 태그도 `[local]`
- **클러스터 peer 자동 탐색 (v0.2)** — 로컬 Proxmox 노드가 `/etc/pve/corosync.conf` 에 있으면 peer 를 자동으로 서버 목록에 붙임
- **TTL 캐시 (v0.2)** — `fetch_containers` 10s / `fetch_host_info` 5s 인메모리 캐시. `start/stop_container` 후 즉시 무효화
- **단일 인스턴스 TUI (v0.2)** — 톱니바퀴 클릭 시 기존 `tmux-topbar` 윈도우가 있으면 그리 전환, 없으면 새로 생성

## Tools

| Crate | Description |
|-------|-------------|
| [tmux-fmt](crates/tmux-fmt/) | Type-safe tmux format string builder, tmux command helpers, re-entrancy guard |
| [tmux-sessionbar](crates/tmux-sessionbar/) | Session management, status bar, CPU/memory, plugin manager, account sync |
| [tmux-windowbar](crates/tmux-windowbar/) | Window/pane management, user switching, app launcher, layout save/restore |
| [tmux-topbar](crates/tmux-config/) | TUI configuration manager (SSH, Apps, Proxmox, Settings). 구 이름 `tmux-config` 는 `tmux-topbar` 심링크로 유지 |

## Requirements

- tmux >= 3.4 (for `range=user` support)
- git (for TPM installation)
- Rust >= 1.70 (to build from source)
- bats-core + bats-support + bats-assert (for smoke tests)

## Install

```bash
# One-liner (downloads latest release)
curl -sL https://raw.githubusercontent.com/dalsoop/dalsoop-tmux-tools/main/install.sh | bash

# Update to latest
install.sh update

# Specific version
install.sh --version v0.1.0

# Uninstall (keeps config files)
curl -sL https://raw.githubusercontent.com/dalsoop/dalsoop-tmux-tools/main/uninstall.sh | bash

# Or build from source
git clone https://github.com/dalsoop/dalsoop-tmux-tools.git
cd dalsoop-tmux-tools
cargo build --release
sudo cp target/release/{tmux-sessionbar,tmux-windowbar,tmux-topbar} /usr/local/bin/
sudo ln -sf tmux-topbar /usr/local/bin/tmux-config  # tmux-sessionbar init 호환 심링크

# One-step setup (does everything)
tmux-sessionbar init
```

### Proxmox / phs integration

```bash
# Deploy to all running LXCs
phs workspace tmux-tools-deploy-all

# Install on a specific LXC
phs workspace lxc-tmux-tools --vmid 50161

# Auto-included in LXC bootstrap
phs infra lxc-create --vmid 50199 --hostname myhost --ip 10.0.50.199 --bootstrap
```

`tmux-sessionbar init` automatically:
1. Creates sessionbar + windowbar configs
2. Generates `.tmux.conf`
3. Installs TPM (Tmux Plugin Manager)
4. Installs 9 plugins (resurrect, continuum, yank, thumbs, open, logging, sensible, notify, menus)
5. Sets up mouse click/double-click bindings
6. Sets up session/window/pane event hooks
7. Writes `~/.config/tmux-sessionbar/bin/*` shims so tmux can call stable paths regardless of server `PATH`

## Architecture

### Workspace structure

```
dalsoop-tmux-tools/
├── crates/
│   ├── tmux-fmt/          # Shared library: format builder + tmux helpers
│   ├── tmux-sessionbar/   # CLI: session management + status bar
│   ├── tmux-windowbar/    # CLI: window/pane management
│   └── tmux-config/       # TUI: configuration manager (ratatui) — 산출물은 `tmux-topbar`
├── .dal/tester/           # dalcenter test runner dal (내부용, UI 숨김)
├── tests/                 # Integration tests (bats)
├── install.sh             # curl | sh installer
└── uninstall.sh           # Uninstaller (keeps configs)
```

### tmux-fmt library

Shared crate used by both CLI tools. Provides:

- **Format string builder** — type-safe API that prevents missing `#[norange]`, forgotten `#[default]` resets
- **tmux command helpers** — `tmux::query()`, `tmux::run()`, `tmux::lines()` with `anyhow` error context
- **Re-entrancy guard** — `tmux::acquire_guard()` prevents recursive hook invocations via tmux timestamp variables

```rust
use tmux_fmt::{click, label, styled, Line};
use tmux_fmt::tmux;

// Type-safe format string (range/norange auto-paired)
let block = click("main", "#282c34", "#98c379", true, " main ");
// → "#[range=user|main]#[fg=#282c34,bg=#98c379,bold] main #[norange default]"

// Compose a full status line
let line = Line::new()
    .left()
    .push(&label("Sessions", "#98c379"))
    .push(&block)
    .right()
    .push(&styled("#abb2bf", "#3e4452", " 5.2 "))
    .build();

// tmux command helpers with error context
let session = tmux::query(&["display-message", "-p", "#S"])?;
let sessions = tmux::lines(&["list-sessions", "-F", "#{session_name}"])?;
tmux::run(&["set", "-g", "status-format[1]", &line])?;

// Re-entrancy guard (100ms debounce)
if !tmux::acquire_guard("render", 100) {
    return Ok(()); // skip, another render just completed
}
```

### Startup sequence

When tmux starts, `.tmux.conf` (generated by `tmux-sessionbar apply`) runs the following in order:

1. **Static config** — key bindings, status bar style, plugin declarations
2. **`tmux-windowbar apply`** (`run-shell -b`) — registers mouse click/double-click bindings via home-local shims under `~/.config/tmux-sessionbar/bin`, sets up window/pane event hooks
3. **`tmux-sessionbar render-status left`** (`run-shell`) — populates session list in status bar via the same shim directory
4. **Session hooks** — `session-created`, `session-closed`, etc. trigger sessionbar re-render
5. **TPM** — loads plugins (resurrect, continuum, etc.)

### Dependency between tools

| Component | Depends on | Why |
|-----------|-----------|-----|
| `tmux-sessionbar` | `tmux-windowbar` binary | `.tmux.conf` calls `tmux-windowbar apply` at startup |
| `tmux-sessionbar` | `tmux-fmt` crate | Format string builder + tmux helpers |
| `tmux-windowbar apply` | `tmux-sessionbar` binary | Click handler falls back to `tmux-sessionbar click` |
| `tmux-windowbar` | `tmux-fmt` crate | Format string builder + tmux helpers |
| Click handlers | `tmux-windowbar` / `tmux-sessionbar` | Bound directly from tmux without wrapper scripts |

> **Note:** Both binaries must be installed before running `tmux-sessionbar init`. The `init` command handles this automatically.

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
blocks = ["ai-window", "hostname", "datetime"]
length = 300

[general]
history_limit = 5000          # scrollback lines per pane (default: 5000)

[keybindings]
session_switch = true
pane_clear = true             # Alt+k to clear pane scrollback

[pane_border]
show_ai_status = true         # optional AI usage summary on each pane border

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

Bindings provided by tmux-sessionbar/windowbar:

| Binding | Action |
|---------|--------|
| `Alt+(` / `Alt+)` | Previous/next session |
| `Alt+s` | Session chooser |
| `Alt+k` | Clear current pane screen + scrollback |

Bindings provided by plugins:

| Binding | Action | Plugin |
|---------|--------|--------|
| `prefix + Ctrl-s` | Save session | tmux-resurrect |
| `prefix + Ctrl-r` | Restore session | tmux-resurrect |
| `prefix + Space` | Highlight URLs/paths/hashes to copy | tmux-thumbs |
| `prefix + \` | Tmux command menu | tmux-menus |
| `prefix + m` | Alert when command finishes | tmux-notify |

## Default Plugins

| Plugin | Purpose |
|--------|---------|
| tmux-plugins/tmux-resurrect | Save/restore sessions across restarts |
| tmux-plugins/tmux-continuum | Auto-save every 15 min + auto-restore |
| tmux-plugins/tmux-yank | System clipboard copy |
| fcsonline/tmux-thumbs | Highlight URLs/paths/hashes for quick copy |
| tmux-plugins/tmux-open | Open selected URL/file |
| tmux-plugins/tmux-logging | Log pane output to file |
| tmux-plugins/tmux-sensible | Optimized defaults (UTF-8, escape-time, etc) |
| rickstaa/tmux-notify | Notification when long command finishes |
| jaclu/tmux-menus | Popup menu for tmux commands |

## Testing

```bash
# Run unit + doc tests
cargo test

# Install bats test dependencies
./tests/install-bats.sh /usr/local

# Run smoke tests (47 tests)
bats tests/smoke.bats

# Or via Docker
./tests/run.sh

# LXC integration test
pct exec <vmid> -- tmux attach -d
```

## License

MIT
