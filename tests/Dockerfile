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

# Copy binaries from builder
COPY --from=builder /app/target/release/tmux-sessionbar /usr/local/bin/
COPY --from=builder /app/target/release/tmux-windowbar /usr/local/bin/

COPY tests/ /app/tests/

RUN /app/tests/install-bats.sh /usr/local

# tmux needs a terminal
ENV TERM=xterm-256color

CMD ["bats", "--tap", "/app/tests/smoke.bats"]
