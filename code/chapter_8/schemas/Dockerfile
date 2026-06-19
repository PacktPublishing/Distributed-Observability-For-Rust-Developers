# ============================================================
# PostgreSQL Database Container
# ============================================================
# Multi-schema database for ecommerce microservices
# Schemas: products, inventory, orders

FROM postgres:17-alpine

# Set environment variables for PostgreSQL
ENV POSTGRES_DB=ecommerce \
    POSTGRES_USER=ecommerce_user \
    POSTGRES_PASSWORD=ecommerce_password \
    POSTGRES_INITDB_ARGS="--encoding=UTF8 --locale=en_US.UTF-8"

# Set PostgreSQL configuration
ENV POSTGRES_HOST_AUTH_METHOD=scram-sha-256

# Copy initialization scripts
# These will be executed in alphabetical order when container starts
COPY init/*.sql /docker-entrypoint-initdb.d/

# Expose PostgreSQL port
EXPOSE 5432

# Add labels
LABEL maintainer="OpenTel Book Team" \
      description="PostgreSQL database with multi-schema setup for ecommerce microservices" \
      version="1.0" \
      schemas="products,inventory,orders,users"

# Health check
HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=3 \
    CMD pg_isready -U $POSTGRES_USER -d $POSTGRES_DB || exit 1

# Volume for persistent data
VOLUME ["/var/lib/postgresql/data"]

# The default CMD from postgres:17-alpine will be used
# CMD ["postgres"]
