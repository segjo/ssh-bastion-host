# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source files
COPY web-dashboard/Cargo.toml web-dashboard/Cargo.lock* ./
COPY web-dashboard/src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    procps \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/ssh-bastion-dashboard /app/ssh-bastion-dashboard

# Set environment
ENV RUST_LOG=info

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Run the application
CMD ["/app/ssh-bastion-dashboard"]
