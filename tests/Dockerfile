FROM rust:1.94-bookworm

RUN apt-get update && apt-get install -y \
    tmux \
    bats \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Install bats helpers
RUN mkdir -p /usr/lib/bats && \
    git clone --depth 1 https://github.com/bats-core/bats-support.git /usr/lib/bats/bats-support && \
    git clone --depth 1 https://github.com/bats-core/bats-assert.git /usr/lib/bats/bats-assert

WORKDIR /app
COPY . .

# Build
RUN cargo build --release && \
    cp target/release/tmux-sessionbar /usr/local/bin/ && \
    cp target/release/tmux-windowbar /usr/local/bin/

COPY tests/ /app/tests/

CMD ["bats", "/app/tests/smoke.bats"]
