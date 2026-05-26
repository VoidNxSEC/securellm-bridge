FROM rust:1.95 as builder

WORKDIR /build

# Copy bridge source
COPY ./securellm-bridge .

# Build release binary
RUN cargo build --release --bin gateway-mcp

# ============================================
# Production Image
# ============================================
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/gateway-mcp /app/gateway-mcp

# Create logs directory
RUN mkdir -p /app/logs

# Runtime configuration
ENV GATEWAY_TRANSPORT=http
ENV LISTEN_ADDR=0.0.0.0:3000
ENV LOG_DIR=/app/logs
ENV RUST_LOG=info

EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=5s --retries=3 --start-period=10s \
  CMD curl -f http://localhost:3000/softwares || exit 1

ENTRYPOINT ["/app/gateway-mcp"]
