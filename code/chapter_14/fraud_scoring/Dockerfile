# Multi-stage Dockerfile for Fraud Scoring Service
# Build from app directory: docker build -t fraud-scoring-service -f fraud_scoring/Dockerfile .

# Stage 1: Build stage
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY fraud_scoring ./fraud_scoring
COPY products ./products
COPY otelmart ./otelmart
COPY inventory ./inventory
COPY orders ./orders

# Build the application
WORKDIR /build/fraud_scoring
RUN cargo build --release --bin fraud_scoring

# Stage 2: Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc wget && update-ca-certificates

# Create non-root user
RUN addgroup -g 1000 fraud && \
    adduser -D -u 1000 -G fraud fraud

# Set working directory
WORKDIR /app

# Copy binary from builder (workspace builds to /build/target)
COPY --from=builder /build/target/release/fraud_scoring /app/fraud_scoring

# Copy configuration (use docker config)
COPY --from=builder /build/fraud_scoring/config.docker.toml /app/config.toml

# Change ownership
RUN chown -R fraud:fraud /app

# Switch to non-root user
USER fraud

# Expose port
EXPOSE 3004

# Run the application
CMD ["/app/fraud_scoring"]
