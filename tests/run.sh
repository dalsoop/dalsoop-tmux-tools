#!/bin/bash
set -e

cd "$(dirname "$0")/.."

if ! command -v docker &>/dev/null; then
    echo "Docker not found. Install docker or run: bats tests/smoke.bats"
    exit 1
fi

echo "Building test container..."
docker build -f tests/Dockerfile -t tmux-tools-test .

echo "Running smoke tests in container..."
docker run --rm tmux-tools-test
