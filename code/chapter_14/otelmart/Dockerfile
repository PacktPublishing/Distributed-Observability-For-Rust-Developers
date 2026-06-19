# ============================================================
# Multi-stage Dockerfile for OtelMart Service + Angular Client
# ============================================================

# Stage 1: Build Angular Client
FROM node:20-alpine AS client-builder

WORKDIR /app/client

# Copy client package files
COPY client/package*.json ./

# Install dependencies
RUN npm ci

# Copy client source
COPY client/ ./

# Build Angular app for production
RUN npm run build --configuration=production && \
    ls -la dist/ || echo "No dist directory" && \
    ls -la dist/*/browser || echo "No browser directory found"

# Stage 2: Build Rust OtelMart Service
FROM rust:latest AS rust-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy workspace Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy otelmart service
COPY otelmart/Cargo.toml ./otelmart/
COPY otelmart/src ./otelmart/src/
COPY otelmart/config.toml ./otelmart/
COPY otelmart/config.docker.toml ./otelmart/

# Copy products (required by workspace)
COPY products/Cargo.toml ./products/
COPY products/src ./products/src/

# Copy inventory (required by workspace)
COPY inventory/Cargo.toml ./inventory/
COPY inventory/src ./inventory/src/

# Copy orders (required by workspace)
COPY orders/Cargo.toml ./orders/
COPY orders/src ./orders/src/

# Copy fraud_scoring (required by workspace)
COPY fraud_scoring/Cargo.toml ./fraud_scoring/
COPY fraud_scoring/src ./fraud_scoring/src/

# Build the service (release mode)
WORKDIR /app
RUN cargo build --release

# Stage 3: Runtime image (trixie has GLIBC 2.38+ matching rust:latest builder)
FROM debian:trixie-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Copy built binary from rust-builder
COPY --from=rust-builder /app/target/release/otelmart /app/otelmart

# Copy config file (use docker config)
COPY --from=rust-builder /app/otelmart/config.docker.toml /app/config.toml

# Copy built Angular app from client-builder
COPY --from=client-builder /app/client/dist /app/dist

# Create non-root user
RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 4200

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/bin/curl", "-f", "http://localhost:4200/health"]

# Run the service
CMD ["/app/otelmart"]
