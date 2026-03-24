FROM rust:1.94-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    tmux \
    git \
    sudo \
    curl \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Install bats (latest from GitHub for better features)
RUN git clone --depth 1 https://github.com/bats-core/bats-core.git /tmp/bats && \
    /tmp/bats/install.sh /usr/local && \
    rm -rf /tmp/bats && \
    mkdir -p /usr/lib/bats && \
    git clone --depth 1 https://github.com/bats-core/bats-support.git /usr/lib/bats/bats-support && \
    git clone --depth 1 https://github.com/bats-core/bats-assert.git /usr/lib/bats/bats-assert

# Copy binaries from builder
COPY --from=builder /app/target/release/tmux-sessionbar /usr/local/bin/
COPY --from=builder /app/target/release/tmux-windowbar /usr/local/bin/

COPY tests/ /app/tests/

# tmux needs a terminal
ENV TERM=xterm-256color

CMD ["bats", "--tap", "/app/tests/smoke.bats"]
