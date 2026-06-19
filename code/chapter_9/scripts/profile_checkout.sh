#!/bin/bash
# High-Throughput Checkout Profiling Script
# 
# Purpose:
# This script is specifically designed to blast the POST /api/orders endpoint.
# Instead of simulating realistic traffic (like generate_traffic.sh), it
# fires rapid concurrent requests to create maximum CPU and Memory pressure
# on the `orders` service. This guarantees a highly visible bottleneck when
# generating flamegraphs or dhat heap profiles.
#
# Usage:
#   ./profile_checkout.sh              # Blast for 30 seconds
#   DURATION=60 ./profile_checkout.sh  # Blast for 60 seconds

set -e

GATEWAY_URL="${GATEWAY_URL:-http://localhost:4200}"
DURATION="${DURATION:-30}"
CONCURRENCY=5

echo "========================================="
echo "Checkout Profiling Load Generator"
echo "========================================="
echo "Targeting: $GATEWAY_URL/api/orders"
echo "Duration:  ${DURATION}s"
echo "Concurrency: $CONCURRENCY worker loops"
echo ""

if ! curl -s -f "$GATEWAY_URL/health" > /dev/null 2>&1; then
    echo "ERROR: Gateway service not reachable at $GATEWAY_URL"
    exit 1
fi

# We need a stable product UUID to build the request. Let's fetch one.
echo "Fetching a valid product for checkout..."
PRODUCTS_JSON=$(curl -s "$GATEWAY_URL/api/products?page_size=1")
PRODUCT_UUID=$(echo "$PRODUCTS_JSON" | jq -r '.products[0].eid')
PRODUCT_NAME=$(echo "$PRODUCTS_JSON" | jq -r '.products[0].product_name')
PRODUCT_PRICE=$(echo "$PRODUCTS_JSON" | jq -r '.products[0].final_price')

if [ -z "$PRODUCT_UUID" ] || [ "$PRODUCT_UUID" = "null" ]; then
    echo "ERROR: Failed to fetch a product for checkout."
    exit 1
fi

echo "Using product: $PRODUCT_NAME ($PRODUCT_UUID)"
echo ""

START_TIME=$(date +%s)
END_TIME=$((START_TIME + DURATION))

# Worker function to constantly blast requests
blast_checkout() {
    local worker_id=$1
    local req_count=0
    
    # Pre-build the JSON payload to avoid bash processing overhead in the hot loop
    local payload="{
        \"customer_email\": \"profiler${worker_id}@example.com\",
        \"items\": [{
            \"product_uuid\": \"$PRODUCT_UUID\",
            \"product_name\": \"$PRODUCT_NAME\",
            \"quantity\": 1,
            \"unit_price\": \"$PRODUCT_PRICE\"
        }],
        \"shipping_address\": {
            \"first_name\": \"Perf\",
            \"last_name\": \"Test\",
            \"address_line1\": \"1 Profiling Way\",
            \"city\": \"Flamegraph City\",
            \"state\": \"CA\",
            \"postal_code\": \"90000\",
            \"country\": \"US\"
        },
        \"payment\": {
            \"payment_method\": \"credit_card\",
            \"card_last4\": \"1234\",
            \"card_brand\": \"visa\"
        }
    }"

    while [ $(date +%s) -lt $END_TIME ]; do
        # We don't care if it succeeds (201) or fails (409 Insufficient Stock).
        # Both go through the validation hot path which is what we want to profile!
        curl -s -X POST "$GATEWAY_URL/api/orders" \
            -H "Content-Type: application/json" \
            -d "$payload" > /dev/null 2>&1
            
        ((req_count++))
    done
    
    echo "Worker $worker_id finished. Fired ~$req_count requests."
}

echo "Starting $CONCURRENCY background workers..."

for ((i=1; i<=CONCURRENCY; i++)); do
    blast_checkout $i &
done

# Wait for completion
wait

echo "========================================="
echo "Profiling run complete!"
echo "Check your flamegraph.svg or dhat-heap.json"
echo "========================================="
