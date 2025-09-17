# Build stage
FROM rust:1.80-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY bot/ ./bot/
COPY lex/ ./lex/

# Build the application
RUN cargo build --release --bin bot

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Copy the binary
COPY --from=builder /app/target/release/bot /usr/local/bin/discord-to-sp-bot

# Set ownership and permissions
RUN chown appuser:appuser /usr/local/bin/discord-to-sp-bot

# Switch to non-root user
USER appuser

# Expose any necessary ports (if needed)
# EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/discord-to-sp-bot"]
