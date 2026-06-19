# Multi-stage Dockerfile for Products Service
# Build from app directory: docker build -t products-service -f products/Dockerfile .

# Stage 1: Build stage
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY products ./products
COPY otelmart ./otelmart
COPY inventory ./inventory
COPY orders ./orders

# Build the application
WORKDIR /build/products
RUN cargo build --release --bin products

# Stage 2: Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc wget

# Create non-root user
RUN addgroup -g 1000 products && \
    adduser -D -u 1000 -G products products

# Set working directory
WORKDIR /app

# Copy binary from builder (workspace builds to /build/target)
COPY --from=builder /build/target/release/products /app/products

# Copy configuration (use docker config)
COPY --from=builder /build/products/config.docker.toml /app/config.toml

# Change ownership
RUN chown -R products:products /app

# Switch to non-root user
USER products

# Expose port
EXPOSE 3001

# Run the application
CMD ["/app/products"]
