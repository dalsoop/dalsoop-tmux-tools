# tmux-windowbar

Window/pane management, user switching, app launcher, and layout save/restore for tmux. Built in Rust.

## Commands

```bash
tmux-windowbar init               # Create config + set bindings
tmux-windowbar apply              # Re-apply bindings and hooks
tmux-windowbar render             # Render window list (called by sessionbar)
tmux-windowbar render-view        # Output view switcher icons

tmux-windowbar click <range>      # Handle mouse click (called by tmux)

tmux-windowbar layout-save <name> # Save current window/pane layout
tmux-windowbar layout-load <name> # Restore saved layout
tmux-windowbar layout-list        # List saved layouts
```

## Status Lines Managed

| Line | Content |
|------|---------|
| 0 | Users — system accounts, click to switch + filter |
| 2 | Windows — all windows across sessions with [x][+] |
| 3 | Panes — all panes with status colors, [x], [|], [-] |
| 4 | Apps — clickable app launcher |

## Pane Colors

| State | Color | Condition |
|-------|-------|-----------|
| Active | Green | Current pane |
| Running | Cyan | Non-shell process (vim, node, etc) |
| Idle | Gray | Shell prompt (bash, zsh, etc) |
| Custom | Per config | Defined in `[colors.<command>]` |

## Click Handlers

| Range prefix | Action |
|-------------|--------|
| `_ws<idx>` | Switch window (current session) |
| `_wa<s>.<w>` | Switch window (any session) |
| `_wk<idx>` | Kill window (current session) |
| `_wx<s>.<w>` | Kill window (any session) |
| `_pp<s>.<w>.<p>` | Select pane |
| `_px<s>.<w>.<p>` | Kill pane |
| `_wnew_` | New window |
| `_splith` | Split horizontal |
| `_splitv` | Split vertical |
| `_u<user>` | Switch to user session + filter view |
| `_v<mode>` | Change view mode |
| `_app<idx>` | Launch app |
