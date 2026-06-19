# Multi-stage Dockerfile for Orders Service
# Build from app directory: docker build -t orders-service -f orders/Dockerfile .

# Stage 1: Build stage
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY orders ./orders
COPY products ./products
COPY otelmart ./otelmart
COPY inventory ./inventory

# Build the application
WORKDIR /build/orders
RUN cargo build --release --bin orders

# Stage 2: Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc wget

# Create non-root user
RUN addgroup -g 1000 orders && \
    adduser -D -u 1000 -G orders orders

# Set working directory
WORKDIR /app

# Copy binary from builder (workspace builds to /build/target)
COPY --from=builder /build/target/release/orders /app/orders

# Copy configuration (use docker config)
COPY --from=builder /build/orders/config.docker.toml /app/config.toml

# Change ownership
RUN chown -R orders:orders /app

# Switch to non-root user
USER orders

# Expose port
EXPOSE 3003

# Run the application
CMD ["/app/orders"]
