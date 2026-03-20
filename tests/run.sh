#!/bin/bash
set -e

cd "$(dirname "$0")/.."

# Try docker first, fall back to local
if command -v docker &>/dev/null; then
    echo "Building test container..."
    docker build -f tests/Dockerfile -t tmux-tools-test .
    echo "Running smoke tests..."
    docker run --rm tmux-tools-test
else
    echo "Docker not found, running tests locally..."
    bats tests/smoke.bats
fi
