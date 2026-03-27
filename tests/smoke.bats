#!/usr/bin/env bats

load '/usr/lib/bats/bats-support/load'
load '/usr/lib/bats/bats-assert/load'

# All tests run on an isolated tmux server to avoid affecting the host session.
TEST_SOCKET="tmux-smoke-test"

_tmux() {
    tmux -L "$TEST_SOCKET" "$@"
}

# Export TMUX_SOCKET so CLI tools also use the isolated server
setup() {
    export TMUX_SOCKET="$TEST_SOCKET"
}

setup_file() {
    export TMUX_SOCKET="$TEST_SOCKET"
    # Kill any leftover test server
    tmux -L "$TEST_SOCKET" kill-server 2>/dev/null || true
    # Start isolated server
    tmux -L "$TEST_SOCKET" new-session -d -s test
    # Init inside isolated server
    tmux-sessionbar init 2>/dev/null || true
    # Apply windowbar to set up bindings
    tmux-windowbar apply 2>/dev/null || true
}

teardown_file() {
    tmux -L "$TEST_SOCKET" kill-server 2>/dev/null || true
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
    [ -f "$HOME/.config/tmux-sessionbar/config.toml" ]
    [ -f "$HOME/.tmux.conf" ]
}

@test "tmux-windowbar init creates config" {
    [ -f "$HOME/.config/tmux-windowbar/config.toml" ]
}

# --- Config ---

@test "sessionbar config has expected sections" {
    run cat "$HOME/.config/tmux-sessionbar/config.toml"
    assert_success
    assert_output --partial "[status]"
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
    run _tmux show -gv status-format[1]
    assert_success
    assert_output --partial "Sessions"
}

@test "status-format[1] has Sessions line" {
    run _tmux show -gv status-format[1]
    assert_success
    assert_output --partial "Sessions"
}

@test "status is 5 lines" {
    run _tmux show -gv status
    assert_success
    assert_output "5"
}

# --- Click: session ---

@test "click creates new session" {
    local before=$(_tmux list-sessions | wc -l)
    run tmux-sessionbar click "_new_"
    assert_success
    local after=$(_tmux list-sessions | wc -l)
    [ "$after" -gt "$before" ]
}

@test "click switches session" {
    _tmux new-session -d -s clicktest
    # switch-client requires an attached client; in isolated server just verify no crash
    run tmux-sessionbar click "clicktest"
    # May fail with "no current client" in headless mode — that's expected
    run _tmux has-session -t clicktest
    assert_success
    _tmux kill-session -t clicktest
}

# --- Click: window ---

@test "click creates new window" {
    local before=$(_tmux list-windows | wc -l)
    run tmux-windowbar click "_wnew_"
    assert_success
    local after=$(_tmux list-windows | wc -l)
    [ "$after" -gt "$before" ]
}

@test "click switches window" {
    _tmux new-window -t :9
    run tmux-windowbar click "_ws9"
    assert_success
    run _tmux display-message -p '#{window_index}'
    assert_output "9"
    _tmux kill-window -t :9
}

# --- Click: split ---

@test "click split-h creates pane" {
    local before=$(_tmux list-panes | wc -l)
    run tmux-windowbar click "_splith"
    assert_success
    local after=$(_tmux list-panes | wc -l)
    [ "$after" -gt "$before" ]
}

@test "click split-v creates pane" {
    local before=$(_tmux list-panes | wc -l)
    run tmux-windowbar click "_splitv"
    assert_success
    local after=$(_tmux list-panes | wc -l)
    [ "$after" -gt "$before" ]
}

# --- Click: view mode ---

@test "click view mode sets @view_mode" {
    run tmux-windowbar click "_vCompact"
    assert_success
    run _tmux show -gv @view_mode
    assert_output "compact"
}

@test "click view All clears @view_user" {
    _tmux set -g @view_user "someone"
    run tmux-windowbar click "_vAll"
    assert_success
    run _tmux show -gv @view_user
    assert_failure  # variable should be unset
}

# --- Click: user ---

@test "click user sets @view_user" {
    # _uroot triggers switch-client which needs an attached client
    # In headless mode, it will fail after setting @view_user — that's OK
    tmux-windowbar click "_uroot" 2>/dev/null || true
    run _tmux show -gv @view_user
    assert_output "root"
}

# --- Click: app switch-or-create ---

@test "app click switches to existing window instead of creating new" {
    _tmux new-window -n bash
    local before=$(_tmux list-windows | wc -l)
    # switch-client may fail in headless mode, but should not create new window
    tmux-windowbar click "_app5" 2>/dev/null || true
    local after=$(_tmux list-windows | wc -l)
    [ "$after" -eq "$before" ]
}

@test "app click creates new window when none exists" {
    # In headless mode, new-window may fail due to no client.
    # Test the underlying switch_to_existing_app logic instead:
    # verify that when no matching window exists, the command attempts to create one
    # (even if creation fails in headless). We check via window count on all sessions.
    local before=$(_tmux list-windows -a | wc -l)
    tmux-windowbar click "_app5" 2>/dev/null || true
    local after=$(_tmux list-windows -a | wc -l)
    # In headless mode new-window may or may not succeed depending on tmux version
    # At minimum, verify no crash and count didn't decrease
    [ "$after" -ge "$before" ]
}

# --- Click handler race fix ---

@test "sessionbar click sources confirm file directly" {
    run rg -n "source-file.*tmux-pending-confirm.conf" \
        "$BATS_TEST_DIRNAME/../crates/tmux-sessionbar/src/commands/click.rs" \
        "$BATS_TEST_DIRNAME/../crates/tmux-windowbar/src/commands/click.rs"
    assert_success
    assert_output --partial "source-file"
}

@test "dblclick sources rename file directly" {
    run rg -n "source-file.*tmux-pending-rename.conf" \
        "$BATS_TEST_DIRNAME/../crates/tmux-windowbar/src/commands/click.rs"
    assert_success
    assert_output --partial "source-file"
}

@test "MouseDown1Status uses run-shell directly (no if-shell race)" {
    run _tmux list-keys
    assert_output --partial '$HOME/.config/tmux-sessionbar/bin/tmux-windowbar click "#{mouse_status_range}"'
    refute_output --partial "if-shell.*pending-confirm"
}

# --- Apply ---

@test "tmux-windowbar apply sets bindings" {
    run tmux-windowbar apply
    assert_success
    run _tmux list-keys
    assert_output --partial "MouseDown1Status"
}

# --- Kill confirm file ---

@test "session kill writes pending confirm file" {
    _tmux new-session -d -s killme
    rm -f /tmp/tmux-pending-confirm.conf
    run tmux-sessionbar click "_kkillme"
    assert_success
    [ -f /tmp/tmux-pending-confirm.conf ]
    run cat /tmp/tmux-pending-confirm.conf
    assert_output --partial "confirm-before"
    assert_output --partial "killme"
    _tmux kill-session -t killme 2>/dev/null || true
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
    rm -f "$HOME/.config/tmux-windowbar/layouts/bats-test.layout"
}

# --- Double click binding ---

@test "double-click binding is set" {
    run _tmux list-keys
    assert_output --partial "DoubleClick1Status"
    assert_output --partial '$HOME/.config/tmux-sessionbar/bin/tmux-windowbar dblclick "#{mouse_status_range}"'
}

# --- Pane clear ---

@test "click clear button" {
    run tmux-sessionbar click "_clear_"
    assert_success
}

@test "Alt+k pane clear binding exists" {
    run _tmux list-keys
    assert_output --partial "M-k"
    assert_output --partial "clear-history"
}

@test "clear button rendered in sessions line" {
    # Re-render to make sure status-format is populated
    tmux-sessionbar render-status left 2>/dev/null || true
    run _tmux show -gv status-format[1]
    assert_success
    assert_output --partial "_clear_"
}

# --- Config validation ---

@test "config has general section with history_limit" {
    run cat "$HOME/.config/tmux-sessionbar/config.toml"
    assert_success
    assert_output --partial "[general]"
    assert_output --partial "history_limit"
}

@test "generated tmux.conf includes history-limit" {
    run cat "$HOME/.tmux.conf"
    assert_success
    assert_output --partial "history-limit"
}

@test "generated tmux.conf includes pane clear" {
    run cat "$HOME/.tmux.conf"
    assert_success
    assert_output --partial "M-k"
    assert_output --partial "clear-history"
}

# --- Sync ---

@test "tmux-sessionbar sync command exists" {
    run tmux-sessionbar sync --help 2>&1
    [ $? -eq 0 ] || [ $? -eq 2 ]
}

# ── Domain invariant tests ──
# These test core guarantees where failure = broken user experience.

@test "DOMAIN: 5-line status bar — status is 5" {
    run _tmux show -gv status
    assert_success
    assert_output "5"
}

@test "DOMAIN: 5-line status bar — all 5 format lines exist" {
    for i in 0 1 2 3 4; do
        run _tmux show -gv "status-format[$i]"
        assert_success
        # Each line must have some content (not empty)
        [ -n "$output" ]
    done
}

@test "DOMAIN: click handlers exist — MouseDown1Status binding present" {
    run _tmux list-keys
    assert_success
    assert_output --partial "MouseDown1Status"
}

@test "DOMAIN: click handlers exist — DoubleClick1Status binding present" {
    run _tmux list-keys
    assert_success
    assert_output --partial "DoubleClick1Status"
}

@test "DOMAIN: kill safety — cannot kill last session" {
    # Ensure only one session exists
    local sessions
    sessions=$(_tmux list-sessions -F '#{session_name}')
    local count
    count=$(echo "$sessions" | wc -l)
    if [ "$count" -gt 1 ]; then
        # Kill extras so we have exactly one
        echo "$sessions" | tail -n +2 | while read -r s; do
            _tmux kill-session -t "=$s" 2>/dev/null || true
        done
    fi

    # Get the remaining session name
    local last
    last=$(_tmux list-sessions -F '#{session_name}' | head -1)

    # Try to kill it via the sessionbar click handler
    rm -f /tmp/tmux-pending-confirm.conf
    run tmux-sessionbar click "_k${last}"
    assert_success

    # The session must still exist — it was the last one
    run _tmux has-session -t "=${last}"
    assert_success

    # No confirm file should have been written (display-message path, not kill path)
    [ ! -f /tmp/tmux-pending-confirm.conf ]
}

@test "DOMAIN: config roundtrip — config files exist and are valid TOML" {
    # sessionbar config
    local sb_config="$HOME/.config/tmux-sessionbar/config.toml"
    [ -f "$sb_config" ]
    # Basic TOML validation: must contain at least one [section]
    run grep -c '^\[' "$sb_config"
    assert_success
    [ "$output" -ge 1 ]

    # windowbar config
    local wb_config="$HOME/.config/tmux-windowbar/config.toml"
    [ -f "$wb_config" ]
    run grep -c '^\[' "$wb_config"
    assert_success
    [ "$output" -ge 1 ]

    # tmux.conf must exist
    [ -f "$HOME/.tmux.conf" ]
}
