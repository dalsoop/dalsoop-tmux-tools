#!/bin/bash
set -euo pipefail

REPO="dalsoop/dalsoop-tmux-tools"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARIES="tmux-sessionbar tmux-windowbar tmux-topbar"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${BLUE}[info]${NC} $*"; }
ok()    { echo -e "${GREEN}[ok]${NC} $*"; }
err()   { echo -e "${RED}[err]${NC} $*" >&2; exit 1; }

# Detect version
if [ "${1:-}" = "--version" ] && [ -n "${2:-}" ]; then
    VERSION="$2"
elif [ "${1:-}" = "update" ]; then
    VERSION="latest"
else
    VERSION="${VERSION:-latest}"
fi

# Check deps
command -v curl >/dev/null || err "curl required"
command -v tar >/dev/null || err "tar required"

# Resolve latest version
if [ "$VERSION" = "latest" ]; then
    info "Fetching latest release..."
    VERSION=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    [ -n "$VERSION" ] || err "Failed to fetch latest version"
fi

info "Installing tmux-tools $VERSION"

# Check current version
CURRENT=""
if [ -f "$INSTALL_DIR/tmux-sessionbar" ]; then
    # Try to get version from binary
    CURRENT=$("$INSTALL_DIR/tmux-sessionbar" --version 2>/dev/null | head -1 || echo "unknown")
fi

# Download
URL="https://github.com/$REPO/releases/download/$VERSION/tmux-tools-x86_64-linux.tar.gz"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading $URL"
curl -sL "$URL" -o "$TMPDIR/tmux-tools.tar.gz" || err "Download failed"

# Extract
tar xzf "$TMPDIR/tmux-tools.tar.gz" -C "$TMPDIR"

# Install
for bin in $BINARIES; do
    if [ -f "$TMPDIR/$bin" ]; then
        cp "$TMPDIR/$bin" "$INSTALL_DIR/$bin"
        chmod +x "$INSTALL_DIR/$bin"
        ok "$bin → $INSTALL_DIR/$bin"
    fi
done

# tmux-sessionbar init 가 내부에서 tmux-config 이름을 호출하므로 호환 심링크.
if [ -f "$INSTALL_DIR/tmux-topbar" ]; then
    ln -sf tmux-topbar "$INSTALL_DIR/tmux-config"
    ok "tmux-config → tmux-topbar (compat symlink)"
fi

# Init configs if first install
if [ ! -f "$HOME/.config/tmux-sessionbar/config.toml" ]; then
    info "First install — initializing sessionbar config..."
    "$INSTALL_DIR/tmux-sessionbar" init 2>/dev/null || true
fi

if [ ! -f "$HOME/.config/tmux-windowbar/config.toml" ]; then
    info "First install — initializing windowbar config..."
    "$INSTALL_DIR/tmux-windowbar" init 2>/dev/null || true
fi

# Apply
if command -v tmux >/dev/null && tmux info >/dev/null 2>&1; then
    info "Applying to running tmux..."
    "$INSTALL_DIR/tmux-sessionbar" apply 2>/dev/null || true
    "$INSTALL_DIR/tmux-windowbar" apply 2>/dev/null || true
fi

echo ""
ok "tmux-tools $VERSION installed!"
echo "  Binaries: $INSTALL_DIR/{$(echo $BINARIES | tr ' ' ',')}"
echo ""
echo "  Update:  curl -sL https://raw.githubusercontent.com/$REPO/main/install.sh | bash"
echo "  Or:      install.sh update"
