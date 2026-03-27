# tmux-sessionbar

Session management, status bar generation, plugin manager, and account sync for tmux. Built in Rust.

## Commands

```bash
tmux-sessionbar init              # One-step full setup (TPM, plugins, windowbar, bindings)
tmux-sessionbar apply             # Regenerate .tmux.conf and reload
tmux-sessionbar status            # Show diagnostics
tmux-sessionbar sync              # Sync configs to all system accounts

tmux-sessionbar plugin-list       # List plugins
tmux-sessionbar plugin-add <name> # Add + install plugin
tmux-sessionbar plugin-rm <name>  # Remove + cleanup plugin
tmux-sessionbar plugin-install    # Reinstall all plugins
```

## What `init` does

1. Creates `~/.config/tmux-sessionbar/config.toml`
2. Generates `~/.tmux.conf` with hooks, key bindings, plugin config
3. Installs TPM if missing
4. Initializes tmux-windowbar (config + mouse bindings)
5. Reloads tmux config
6. Installs all plugins via TPM
7. Creates `/tmp/tmux-{uid}` dir (LXC compatibility)
8. Writes `~/.config/tmux-sessionbar/bin` shims so tmux can call stable paths without relying on server `PATH`

## How It Works

- `status-format[1]` is set directly with session blocks wrapped in `#[range=user|name]`
- tmux hooks (`client-session-changed`, `session-created`, etc.) trigger re-render
- Mouse clicks handled directly by home-local shim bindings with confirm-before for kills
- CPU/memory stats read from `/proc/loadavg` and `/proc/meminfo`
- Plugins managed as `[[plugins]]` entries in `config.toml`
- 🧹 clear button and `Alt+k` keybinding to clear pane screen + scrollback
- Configurable `history-limit` (default 5000) and cron-based auto-cleanup
- `apply` automatically re-applies windowbar mouse bindings
