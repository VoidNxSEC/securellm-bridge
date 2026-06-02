# Multi-stage Dockerfile for SecureLLM Bridge
FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    libspeechd-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY vendor ./vendor

# Build release binaries
RUN cargo build --release --bin securellm-api-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libspeechd2 \
    libasound2 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user and required directories
RUN useradd -m -u 1000 securellm \
    && mkdir -p /var/log/securellm /var/lib/securellm \
    && chown -R securellm:securellm /var/log/securellm /var/lib/securellm

# Copy binary
COPY --from=builder /app/target/release/securellm-api-server /usr/local/bin/
RUN chown securellm:securellm /usr/local/bin/securellm-api-server

USER securellm
WORKDIR /home/securellm

RUN mkdir -p .config/securellm data

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=15s --retries=5 \
    CMD curl -sf http://localhost:8080/health || curl -sf http://localhost:8080/api/health || exit 1

ENTRYPOINT ["securellm-api-server"]
