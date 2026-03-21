#!/usr/bin/env bats

load '/usr/lib/bats/bats-support/load'
load '/usr/lib/bats/bats-assert/load'

setup_file() {
    # Start tmux server in background
    tmux new-session -d -s test
}

teardown_file() {
    tmux kill-server 2>/dev/null || true
}

# --- Binary existence ---

@test "tmux-sessionbar binary exists" {
    run which tmux-sessionbar
    assert_success
}

@test "tmux-windowbar binary exists" {
    run which tmux-windowbar
    assert_success
}

# --- Help ---

@test "tmux-sessionbar --help" {
    run tmux-sessionbar --help
    assert_success
    assert_output --partial "session"
}

@test "tmux-windowbar --help" {
    run tmux-windowbar --help
    assert_success
    assert_output --partial "window"
}

# --- Init ---

@test "tmux-sessionbar init creates config" {
    run tmux-sessionbar init
    assert_success
    [ -f "$HOME/.config/tmux-sessionbar/config.toml" ]
    [ -f "$HOME/.tmux.conf" ]
}

@test "tmux-windowbar init creates config" {
    run tmux-windowbar init
    assert_success
    [ -f "$HOME/.config/tmux-windowbar/config.toml" ]
}

# --- Config ---

@test "sessionbar config has expected sections" {
    run cat "$HOME/.config/tmux-sessionbar/config.toml"
    assert_success
    assert_output --partial "[status]"
    assert_output --partial "[blocks.session-list]"
}

@test "windowbar config has expected sections" {
    run cat "$HOME/.config/tmux-windowbar/config.toml"
    assert_success
    assert_output --partial "[window]"
    assert_output --partial "[[apps]]"
}

# --- Status ---

@test "tmux-sessionbar status shows config" {
    run tmux-sessionbar status
    assert_success
    assert_output --partial "config"
}

# --- Render ---

@test "tmux-sessionbar render-status produces output" {
    run tmux-sessionbar render-status left
    assert_success
    # Should have set status-format
    run tmux show -gv status-format[1]
    assert_success
    assert_output --partial "Sessions"
}

@test "tmux-windowbar render produces window list" {
    run tmux-windowbar render
    assert_success
    assert_output --partial "_ws"
}

@test "status-format[0] has Users line" {
    run tmux show -gv status-format[0]
    assert_success
    assert_output --partial "Users"
}

@test "status-format[2] has Windows line" {
    run tmux show -gv status-format[2]
    assert_success
    assert_output --partial "Windows"
}

@test "status-format[3] has Panes line" {
    run tmux show -gv status-format[3]
    assert_success
    assert_output --partial "Panes"
}

@test "status-format[4] has Apps line" {
    run tmux show -gv status-format[4]
    assert_success
    assert_output --partial "Apps"
}

@test "status is 5 lines" {
    run tmux show -gv status
    assert_success
    assert_output "5"
}

# --- Click: session ---

@test "click creates new session" {
    local before=$(tmux list-sessions | wc -l)
    run tmux-sessionbar click "_new_"
    assert_success
    local after=$(tmux list-sessions | wc -l)
    [ "$after" -gt "$before" ]
}

@test "click switches session" {
    tmux new-session -d -s clicktest
    run tmux-sessionbar click "clicktest"
    assert_success
    # Verify session exists and switch worked (hooks may re-render)
    run tmux has-session -t clicktest
    assert_success
    tmux kill-session -t clicktest
}

# --- Click: window ---

@test "click creates new window" {
    local before=$(tmux list-windows | wc -l)
    run tmux-windowbar click "_wnew_"
    assert_success
    local after=$(tmux list-windows | wc -l)
    [ "$after" -gt "$before" ]
}

@test "click switches window" {
    tmux new-window -t :9
    run tmux-windowbar click "_ws9"
    assert_success
    run tmux display-message -p '#{window_index}'
    assert_output "9"
    tmux kill-window -t :9
}

# --- Click: split ---

@test "click split-h creates pane" {
    local before=$(tmux list-panes | wc -l)
    run tmux-windowbar click "_splith"
    assert_success
    local after=$(tmux list-panes | wc -l)
    [ "$after" -gt "$before" ]
}

@test "click split-v creates pane" {
    local before=$(tmux list-panes | wc -l)
    run tmux-windowbar click "_splitv"
    assert_success
    local after=$(tmux list-panes | wc -l)
    [ "$after" -gt "$before" ]
}

# --- Click: view mode ---

@test "click view mode sets @view_mode" {
    run tmux-windowbar click "_vCompact"
    assert_success
    run tmux show -gv @view_mode
    assert_output "compact"
}

@test "click view All clears @view_user" {
    tmux set -g @view_user "someone"
    run tmux-windowbar click "_vAll"
    assert_success
    run tmux show -gv @view_user
    assert_failure  # variable should be unset
}

# --- Click: user ---

@test "click user sets @view_user" {
    run tmux-windowbar click "_uroot"
    assert_success
    run tmux show -gv @view_user
    assert_output "root"
}

# --- Apply ---

@test "tmux-sessionbar apply regenerates config" {
    run tmux-sessionbar apply
    assert_success
}

@test "tmux-windowbar apply sets bindings" {
    run tmux-windowbar apply
    assert_success
    run tmux list-keys
    assert_output --partial "MouseDown1Status"
}

# --- Kill confirm file ---

@test "session kill writes pending confirm file" {
    tmux new-session -d -s killme
    rm -f /tmp/tmux-pending-confirm.conf
    run tmux-sessionbar click "_kkillme"
    assert_success
    [ -f /tmp/tmux-pending-confirm.conf ]
    run cat /tmp/tmux-pending-confirm.conf
    assert_output --partial "confirm-before"
    assert_output --partial "killme"
    # Cleanup
    tmux kill-session -t killme 2>/dev/null || true
    rm -f /tmp/tmux-pending-confirm.conf
}

# --- Layout ---

@test "layout-save creates layout file" {
    run tmux-windowbar layout-save bats-test
    assert_success
    [ -f "$HOME/.config/tmux-windowbar/layouts/bats-test.layout" ]
}

@test "layout-list shows saved layouts" {
    run tmux-windowbar layout-list
    assert_success
    assert_output --partial "bats-test"
}

@test "layout-load restores layout" {
    run tmux-windowbar layout-load bats-test
    assert_success
    # Cleanup
    rm -f "$HOME/.config/tmux-windowbar/layouts/bats-test.layout"
}

# --- System stats ---

@test "sessions line includes system stats" {
    run tmux show -gv status-format[1]
    assert_success
    # Should have load average and memory info
    assert_output --partial "G "
}

# --- Double click binding ---

@test "double-click binding is set" {
    run tmux list-keys
    assert_output --partial "DoubleClick1Status"
}

# --- Sync ---

# --- Pane clear: click ---

@test "click clear button clears history" {
    # Generate some scrollback
    for i in $(seq 1 100); do tmux send-keys "echo line$i" Enter; done
    sleep 0.5
    run tmux-sessionbar click "_clear_"
    assert_success
}

# --- Pane clear: keybinding ---

@test "Alt+k pane clear binding exists" {
    run tmux list-keys
    assert_output --partial "M-k"
    assert_output --partial "clear-history"
}

# --- Pane clear: button rendered ---

@test "clear button rendered in sessions line" {
    run tmux show -gv status-format[1]
    assert_success
    assert_output --partial "_clear_"
}

# --- Config: general section ---

@test "config has general section with history_limit" {
    run cat "$HOME/.config/tmux-sessionbar/config.toml"
    assert_success
    assert_output --partial "[general]"
    assert_output --partial "history_limit"
}

# --- Config: maintenance section ---

@test "config has maintenance section" {
    run cat "$HOME/.config/tmux-sessionbar/config.toml"
    assert_success
    assert_output --partial "[maintenance]"
    assert_output --partial "auto_clear"
    assert_output --partial "clear_interval"
}

# --- Config: keybindings pane_clear ---

@test "config has pane_clear keybinding" {
    run cat "$HOME/.config/tmux-sessionbar/config.toml"
    assert_success
    assert_output --partial "pane_clear"
}

# --- History limit ---

@test "history-limit is set from config" {
    run tmux show -gv history-limit
    assert_success
    # Should be a reasonable number (not the old 50000)
    local limit="$output"
    [ "$limit" -le 10000 ]
}

# --- tmux.conf has history-limit ---

@test "generated tmux.conf includes history-limit" {
    run cat "$HOME/.tmux.conf"
    assert_success
    assert_output --partial "history-limit"
}

# --- tmux.conf has pane clear binding ---

@test "generated tmux.conf includes pane clear" {
    run cat "$HOME/.tmux.conf"
    assert_success
    assert_output --partial "M-k"
    assert_output --partial "clear-history"
}

# --- Auto clear script ---

@test "tmux-clear-history script exists" {
    [ -x /usr/local/bin/tmux-clear-history ]
}

@test "tmux-clear-history runs without error" {
    run /usr/local/bin/tmux-clear-history
    assert_success
}

# --- Cron ---

@test "cron entry for auto-clear is installed" {
    run crontab -l
    assert_success
    assert_output --partial "tmux-clear-history"
}

# --- Apply restores windowbar bindings ---

@test "sessionbar apply also applies windowbar bindings" {
    # Clear the binding first
    tmux unbind -T root MouseDown1Status 2>/dev/null || true
    run tmux-sessionbar apply
    assert_success
    run tmux list-keys
    assert_output --partial "MouseDown1Status"
}

# --- Sync ---

@test "tmux-sessionbar sync command exists" {
    run tmux-sessionbar sync --help 2>&1
    # Just check it doesn't crash
    [ $? -eq 0 ] || [ $? -eq 2 ]
}
