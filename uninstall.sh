#!/bin/bash
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARIES="tmux-sessionbar tmux-windowbar tmux-config"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[info]${NC} $*"; }
ok()   { echo -e "${GREEN}[ok]${NC} $*"; }

echo "This will remove tmux-tools binaries from $INSTALL_DIR"
echo "Config files in ~/.config/tmux-{sessionbar,windowbar}/ will be kept."
echo ""
read -p "Continue? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

for bin in $BINARIES; do
    if [ -f "$INSTALL_DIR/$bin" ]; then
        rm "$INSTALL_DIR/$bin"
        ok "Removed $INSTALL_DIR/$bin"
    else
        info "$bin not found, skipping"
    fi
done

echo ""
ok "tmux-tools uninstalled. Config files preserved."
