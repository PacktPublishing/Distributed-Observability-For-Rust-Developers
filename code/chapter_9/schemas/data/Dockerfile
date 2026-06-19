# ============================================================
# Data Ingestion Service
# ============================================================
# Loads initial data into the database after schema is ready

FROM postgres:17-alpine

# Install wait-for utility
RUN apk add --no-cache postgresql-client

# Copy data files
COPY data/*.sql /data/

# Copy ingestion script
COPY data/ingest.sh /ingest.sh
RUN chmod +x /ingest.sh

# Set working directory
WORKDIR /data

# Run ingestion script
CMD ["/ingest.sh"]
