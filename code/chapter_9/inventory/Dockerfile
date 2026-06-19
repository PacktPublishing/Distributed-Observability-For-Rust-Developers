# Multi-stage Dockerfile for Inventory Service
# Build from app directory: docker build -t inventory-service -f inventory/Dockerfile .

# Stage 1: Build stage
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY orders ./orders
COPY products ./products
COPY otelmart ./otelmart
COPY inventory ./inventory

# Build the application
WORKDIR /build/inventory
RUN cargo build --release --bin inventory

# Stage 2: Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc wget

# Create non-root user
RUN addgroup -g 1000 inventory && \
    adduser -D -u 1000 -G inventory inventory

# Set working directory
WORKDIR /app

# Copy binary from builder (workspace builds to /build/target)
COPY --from=builder /build/target/release/inventory /app/inventory

# Copy configuration (use docker config)
COPY --from=builder /build/inventory/config.docker.toml /app/config.toml

# Change ownership
RUN chown -R inventory:inventory /app

# Switch to non-root user
USER inventory

# Expose port
EXPOSE 3002

# Run the application
CMD ["/app/inventory"]
