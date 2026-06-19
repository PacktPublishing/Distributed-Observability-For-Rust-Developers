#!/bin/sh
# ============================================================
# Data Ingestion Script
# ============================================================
# Loads data into PostgreSQL database

set -e

echo "========================================="
echo "Starting data ingestion..."
echo "========================================="

# Database connection parameters
DB_HOST="${DB_HOST:-postgres}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-opentel_db}"
DB_USER="${DB_USER:-opentel_user}"
DB_PASSWORD="${DB_PASSWORD:-opentel_pass}"
WAIT_TIME="${WAIT_TIME:-10}"

export PGPASSWORD="$DB_PASSWORD"

echo "Waiting for database to be ready..."
until pg_isready -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME"; do
  echo "Database is unavailable - sleeping"
  sleep 2
done

echo "Database is ready!"
echo ""

# Wait for products service to complete migrations
echo "Waiting ${WAIT_TIME}s for products service migrations to complete..."
sleep "$WAIT_TIME"
echo ""

# Load data files in order
echo "Loading products test data..."
if [ -f /data/products_test_data.sql ]; then
  psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f /data/products_test_data.sql
  echo "✓ Products test data loaded successfully"
else
  echo "⚠ products_test_data.sql not found, skipping..."
fi

echo ""
echo "========================================="
echo "Data ingestion completed!"
echo "========================================="
