#!/bin/bash
# ============================================================
# Reset Products Schema
# ============================================================
# Drops existing products schema and recreates with fresh data

set -e

# Database connection parameters
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5433}"
DB_NAME="${DB_NAME:-opentel_db}"
DB_USER="${DB_USER:-opentel_user}"
DB_PASSWORD="${DB_PASSWORD:-opentel_pass}"

export PGPASSWORD="$DB_PASSWORD"

echo "========================================="
echo "Resetting Products Schema"
echo "========================================="
echo ""

# Drop existing schema
echo "1. Dropping existing products schema..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" <<EOF
DROP SCHEMA IF EXISTS products CASCADE;
EOF
echo "   ✓ Schema dropped"
echo ""

# Apply new schema
echo "2. Applying new schema..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f init/00_init.sql
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f init/products_schema_1.sql
echo "   ✓ Schema created"
echo ""

# Ingest test data
echo "3. Ingesting test data..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f data/products_test_data.sql
echo "   ✓ Data loaded"
echo ""

# Verify
echo "4. Verifying data..."
PRODUCT_COUNT=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM products.products;")
RATING_COUNT=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM products.ratings;")
CATEGORY_COUNT=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM products.categories;")

echo "   Products: $PRODUCT_COUNT"
echo "   Ratings: $RATING_COUNT"
echo "   Categories: $CATEGORY_COUNT"
echo ""

echo "========================================="
echo "Schema reset complete!"
echo "========================================="
