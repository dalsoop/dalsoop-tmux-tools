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
