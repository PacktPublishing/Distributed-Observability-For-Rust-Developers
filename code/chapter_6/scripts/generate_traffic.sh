#!/bin/bash
# Realistic E-Commerce Traffic Generator for Chapter 6
#
# Simulates realistic user behavior patterns:
# - User sessions (30s-5min) with multiple concurrent users
# - Product popularity distribution (price-based)
# - User personas: Window Shoppers, Buyers, Power Buyers, Bots
# - Realistic timing patterns and cart abandonment
# - Clustered error scenarios (payment outages, stock depletion)
#
# Prerequisites:
#   - The Chapter 6 docker stack is up:    docker compose up -d
#   - bash, curl, jq are available
#   - The gateway responds at $GATEWAY_URL/health (default: http://localhost:4200)
#
# Usage:
#   ./generate_traffic.sh                   # Run for 2 minutes (default)
#   ./generate_traffic.sh --pv              # Also print Prometheus verification queries at the end
#   ./generate_traffic.sh -h | --help       # Show this help and exit
#   DURATION=300 ./generate_traffic.sh      # Run for 5 minutes
#   GATEWAY_URL=http://otelmart:4200 ./generate_traffic.sh
#                                           # Hit the gateway by container name (run from inside docker)
#
# Environment variables:
#   GATEWAY_URL  Base URL of the otelmart gateway. Default: http://localhost:4200
#   DURATION     How long to run, in seconds.       Default: 120
#
# Running on Windows (no native bash):
#   Use the alpine/curl image on the same docker network as the stack:
#
#     docker run --rm --network chapter_6_app-network \
#       -v ${PWD}/scripts:/scripts \
#       -e GATEWAY_URL=http://otelmart:4200 \
#       -e DURATION=120 \
#       alpine/curl:latest \
#       sh -c "apk add --no-cache jq bash >/dev/null && bash /scripts/generate_traffic.sh"
#
# After the run, point Prometheus / Grafana at http://localhost:9090 and try the
# example PromQL queries from Chapter 6 (sections 6.2, 6.4, 6.6).

set -e

# Configuration
GATEWAY_URL="${GATEWAY_URL:-http://localhost:4200}"
DURATION="${DURATION:-120}"  # Default: 2 minutes
VERIFY_PROMETHEUS=false

print_help() {
    sed -n '2,40p' "$0" | sed 's/^# \{0,1\}//'
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --pv|--prometheus-verify)
            VERIFY_PROMETHEUS=true
            shift
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--pv] [-h|--help]"
            echo "Run '$0 --help' for full instructions."
            exit 1
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo "========================================="
echo "OpenTel E-Commerce Traffic Generator"
echo "========================================="
echo "Gateway URL: $GATEWAY_URL"
echo "Duration: ${DURATION}s"
echo "Prometheus Verification: $VERIFY_PROMETHEUS"
echo ""

# Check if gateway is reachable
echo "Checking service health..."
if ! curl -s -f "$GATEWAY_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Gateway service not reachable at $GATEWAY_URL${NC}"
    echo "Make sure the services are running: docker-compose up"
    exit 1
fi

echo -e "${GREEN}✓ Gateway is healthy${NC}"
echo ""

# Counters
TOTAL_REQUESTS=0
SUCCESS_COUNT=0
ERROR_404_COUNT=0
ERROR_409_COUNT=0
ERROR_422_COUNT=0
ERROR_500_COUNT=0
ORDER_SUCCESS_COUNT=0
ORDER_FAILURE_COUNT=0
SESSION_COUNT=0

# Fetch products once
echo "Fetching products..."
PRODUCTS_JSON=$(curl -s "$GATEWAY_URL/api/products?page_size=50")

if [ -z "$PRODUCTS_JSON" ] || [ "$PRODUCTS_JSON" = "null" ]; then
    echo -e "${RED}ERROR: Failed to fetch products from gateway${NC}"
    exit 1
fi

# Parse products into arrays
PRODUCT_UUIDS=($(echo "$PRODUCTS_JSON" | jq -r '.products[].eid'))
PRODUCT_NAMES=($(echo "$PRODUCTS_JSON" | jq -r '.products[].product_name'))
PRODUCT_PRICES=($(echo "$PRODUCTS_JSON" | jq -r '.products[].final_price'))

if [ ${#PRODUCT_UUIDS[@]} -eq 0 ]; then
    echo -e "${RED}ERROR: No products found. Make sure the database is populated.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Found ${#PRODUCT_UUIDS[@]} products${NC}"

# Create product popularity index (price-based: expensive = more popular)
# Sort products by price descending, top 20% are "popular"
POPULAR_COUNT=$((${#PRODUCT_UUIDS[@]} / 5))
if [ $POPULAR_COUNT -lt 3 ]; then
    POPULAR_COUNT=3
fi

# Create popularity tiers
POPULAR_PRODUCTS=()
REGULAR_PRODUCTS=()

for i in "${!PRODUCT_PRICES[@]}"; do
    if [ $i -lt $POPULAR_COUNT ]; then
        POPULAR_PRODUCTS+=("$i")
    else
        REGULAR_PRODUCTS+=("$i")
    fi
done

echo -e "${CYAN}✓ Product popularity: ${#POPULAR_PRODUCTS[@]} popular, ${#REGULAR_PRODUCTS[@]} regular${NC}"
echo ""

# Helper: Get product with popularity bias
get_product_by_popularity() {
    # 80% chance to select popular product, 20% regular
    if [ $((RANDOM % 100)) -lt 80 ] && [ ${#POPULAR_PRODUCTS[@]} -gt 0 ]; then
        local idx=${POPULAR_PRODUCTS[$((RANDOM % ${#POPULAR_PRODUCTS[@]}))]}
    else
        local idx=${REGULAR_PRODUCTS[$((RANDOM % ${#REGULAR_PRODUCTS[@]}))]}
    fi
    echo "${PRODUCT_UUIDS[$idx]}|${PRODUCT_NAMES[$idx]}|${PRODUCT_PRICES[$idx]}"
}

# Helper: Random delay with activity context
think_time() {
    local activity=$1
    case $activity in
        "quick_scan")
            sleep $(awk 'BEGIN{srand(); print 1+rand()*2}')  # 1-3s
            ;;
        "product_view")
            sleep $(awk 'BEGIN{srand(); print 5+rand()*10}')  # 5-15s
            ;;
        "decision")
            sleep $(awk 'BEGIN{srand(); print 2+rand()*2}')  # 2-4s
            ;;
        "checkout")
            sleep $(awk 'BEGIN{srand(); print 10+rand()*20}')  # 10-30s
            ;;
        "rapid")
            sleep $(awk 'BEGIN{srand(); print 0.1+rand()*0.4}')  # 0.1-0.5s (bot)
            ;;
        *)
            sleep $(awk 'BEGIN{srand(); print 0.5+rand()*1.5}')  # 0.5-2s default
            ;;
    esac
}

# User Persona: Window Shopper (50% of users)
# Browses 5-10 products, checks stock, abandons cart
simulate_window_shopper() {
    local session_id=$1
    local browse_count=$((5 + RANDOM % 6))  # 5-10 products
    
    echo -e "${BLUE}[Session $session_id] Window Shopper: browsing $browse_count products${NC}"
    
    # Browse product list
    curl -s "$GATEWAY_URL/api/products?page_size=20" > /dev/null
    TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
    think_time "quick_scan"
    
    # View individual products
    for i in $(seq 1 $browse_count); do
        local product_info=$(get_product_by_popularity)
        local uuid=$(echo "$product_info" | cut -d'|' -f1)
        
        curl -s "$GATEWAY_URL/api/products/$uuid" > /dev/null
        TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
        think_time "product_view"
        
        # 30% chance to check stock (showing intent)
        if [ $((RANDOM % 100)) -lt 30 ]; then
            curl -s "$GATEWAY_URL/api/inventory/$uuid/stock" > /dev/null
            TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
            think_time "decision"
        fi
    done
    
    echo -e "${YELLOW}[Session $session_id] Window Shopper: abandoned (no purchase)${NC}"
}

# User Persona: Casual Buyer (30% of users)
# Browses 2-5 products, completes purchase
simulate_casual_buyer() {
    local session_id=$1
    local browse_count=$((2 + RANDOM % 4))  # 2-5 products
    
    echo -e "${BLUE}[Session $session_id] Casual Buyer: browsing $browse_count products${NC}"
    
    # Browse products
    for i in $(seq 1 $browse_count); do
        local product_info=$(get_product_by_popularity)
        local uuid=$(echo "$product_info" | cut -d'|' -f1)
        
        curl -s "$GATEWAY_URL/api/products/$uuid" > /dev/null
        TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
        think_time "product_view"
    done
    
    # Check stock before purchase
    local product_info=$(get_product_by_popularity)
    local uuid=$(echo "$product_info" | cut -d'|' -f1)
    local name=$(echo "$product_info" | cut -d'|' -f2)
    local price=$(echo "$product_info" | cut -d'|' -f3)
    
    curl -s "$GATEWAY_URL/api/inventory/$uuid/stock" > /dev/null
    TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
    think_time "decision"
    
    # Create order
    think_time "checkout"
    create_order "$uuid" "$name" "$price" 1 "$session_id"
}

# User Persona: Power Buyer (15% of users)
# Quick browse, multi-item orders
simulate_power_buyer() {
    local session_id=$1
    local item_count=$((2 + RANDOM % 4))  # 2-5 items
    
    echo -e "${BLUE}[Session $session_id] Power Buyer: purchasing $item_count items${NC}"
    
    # Quick product check
    curl -s "$GATEWAY_URL/api/products?page_size=10" > /dev/null
    TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
    think_time "quick_scan"
    
    # Create multi-item order
    local product_info=$(get_product_by_popularity)
    local uuid=$(echo "$product_info" | cut -d'|' -f1)
    local name=$(echo "$product_info" | cut -d'|' -f2)
    local price=$(echo "$product_info" | cut -d'|' -f3)
    
    think_time "checkout"
    create_order "$uuid" "$name" "$price" "$item_count" "$session_id"
}

# User Persona: Bot/Scraper (5% of users)
# Rapid requests, high error rate
simulate_bot() {
    local session_id=$1
    local request_count=$((10 + RANDOM % 20))  # 10-30 rapid requests
    
    echo -e "${CYAN}[Session $session_id] Bot: scraping $request_count pages${NC}"
    
    for i in $(seq 1 $request_count); do
        # Mix of valid and invalid requests
        if [ $((RANDOM % 100)) -lt 30 ]; then
            # 30% invalid requests (404s)
            curl -s "$GATEWAY_URL/api/products/00000000-0000-0000-0000-000000000000" > /dev/null 2>&1
            ERROR_404_COUNT=$((ERROR_404_COUNT + 1))
        else
            # Valid product requests
            curl -s "$GATEWAY_URL/api/products?page_size=50" > /dev/null
        fi
        TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
        think_time "rapid"
    done
}

# Create order with realistic error handling
create_order() {
    local uuid=$1
    local name=$2
    local price=$3
    local quantity=$4
    local session_id=$5
    
    local response=$(curl -s -w "\n%{http_code}" -X POST "$GATEWAY_URL/api/orders" \
        -H "Content-Type: application/json" \
        -d "{
            \"customer_email\": \"user${session_id}@example.com\",
            \"items\": [{
                \"product_uuid\": \"$uuid\",
                \"product_name\": \"$name\",
                \"quantity\": $quantity,
                \"unit_price\": \"$price\"
            }],
            \"shipping_address\": {
                \"first_name\": \"User\",
                \"last_name\": \"${session_id}\",
                \"address_line1\": \"123 Main St\",
                \"city\": \"San Francisco\",
                \"state\": \"CA\",
                \"postal_code\": \"94105\",
                \"country\": \"US\"
            },
            \"payment\": {
                \"payment_method\": \"credit_card\",
                \"card_last4\": \"4242\",
                \"card_brand\": \"visa\"
            }
        }")
    
    local http_code=$(echo "$response" | tail -n1)
    TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
    
    if [ "$http_code" = "201" ] || [ "$http_code" = "200" ]; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        ORDER_SUCCESS_COUNT=$((ORDER_SUCCESS_COUNT + 1))
        echo -e "${GREEN}[Session $session_id] ✓ Order created ($quantity items)${NC}"
    elif [ "$http_code" = "409" ]; then
        ERROR_409_COUNT=$((ERROR_409_COUNT + 1))
        ORDER_FAILURE_COUNT=$((ORDER_FAILURE_COUNT + 1))
        echo -e "${YELLOW}[Session $session_id] ⚠ Order failed: Insufficient stock (409)${NC}"
    else
        ORDER_FAILURE_COUNT=$((ORDER_FAILURE_COUNT + 1))
        echo -e "${RED}[Session $session_id] ✗ Order failed: HTTP $http_code${NC}"
    fi
}

# Simulate payment gateway outage (500 errors)
simulate_payment_outage() {
    echo -e "${RED}[INCIDENT] Payment gateway outage (30s)${NC}"
    local outage_end=$(($(date +%s) + 30))
    
    while [ $(date +%s) -lt $outage_end ]; do
        # Generate validation errors (simulating gateway timeout)
        local product_info=$(get_product_by_popularity)
        local uuid=$(echo "$product_info" | cut -d'|' -f1)
        local name=$(echo "$product_info" | cut -d'|' -f2)
        local price=$(echo "$product_info" | cut -d'|' -f3)
        
        curl -s -X POST "$GATEWAY_URL/api/orders" \
            -H "Content-Type: application/json" \
            -d "{
                \"customer_email\": \"outage@example.com\",
                \"items\": [{
                    \"product_uuid\": \"$uuid\",
                    \"product_name\": \"$name\",
                    \"quantity\": 1,
                    \"unit_price\": \"$price\"
                }],
                \"shipping_address\": {
                    \"first_name\": \"Test\",
                    \"last_name\": \"User\",
                    \"address_line1\": \"123 Main St\",
                    \"city\": \"San Francisco\",
                    \"state\": \"CA\",
                    \"postal_code\": \"94105\",
                    \"country\": \"INVALID\"
                },
                \"payment\": {
                    \"payment_method\": \"credit_card\",
                    \"card_last4\": \"4242\",
                    \"card_brand\": \"visa\"
                }
            }" > /dev/null 2>&1
        
        TOTAL_REQUESTS=$((TOTAL_REQUESTS + 1))
        ERROR_422_COUNT=$((ERROR_422_COUNT + 1))
        sleep 0.5
    done
    
    echo -e "${GREEN}[INCIDENT] Payment gateway recovered${NC}"
}

# Exhaust stock on popular products
exhaust_popular_stock() {
    echo -e "${YELLOW}[INCIDENT] Stock depletion wave on popular products${NC}"
    
    for i in {1..3}; do
        if [ ${#POPULAR_PRODUCTS[@]} -gt $i ]; then
            local idx=${POPULAR_PRODUCTS[$i]}
            local uuid="${PRODUCT_UUIDS[$idx]}"
            local name="${PRODUCT_NAMES[$idx]}"
            local price="${PRODUCT_PRICES[$idx]}"
            
            # Attempt large orders to exhaust stock
            for j in {1..5}; do
                create_order "$uuid" "$name" "$price" 10 "stock_depletion_$i"
                sleep 0.2
            done
        fi
    done
}

# Main traffic generation loop
echo "========================================="
echo "Generating realistic traffic for ${DURATION}s..."
echo "========================================="
echo ""

START_TIME=$(date +%s)
END_TIME=$((START_TIME + DURATION))

# Background session spawner
spawn_user_session() {
    local persona=$1
    SESSION_COUNT=$((SESSION_COUNT + 1))
    local session_id=$SESSION_COUNT
    
    (
        case $persona in
            "window_shopper")
                simulate_window_shopper $session_id
                ;;
            "casual_buyer")
                simulate_casual_buyer $session_id
                ;;
            "power_buyer")
                simulate_power_buyer $session_id
                ;;
            "bot")
                simulate_bot $session_id
                ;;
        esac
    ) &
}

PHASE=1
INCIDENT_TRIGGERED=false

while [ $(date +%s) -lt $END_TIME ]; do
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))
    REMAINING=$((END_TIME - CURRENT_TIME))
    
    # Progress indicator
    if [ $((ELAPSED % 15)) -eq 0 ] && [ $ELAPSED -gt 0 ]; then
        echo -e "${BLUE}[${ELAPSED}s/${DURATION}s] Phase $PHASE - Sessions: $SESSION_COUNT, Requests: $TOTAL_REQUESTS, Orders: $ORDER_SUCCESS_COUNT${NC}"
    fi
    
    # Phase 1 (0-30s): Morning Traffic - Low Volume
    if [ $ELAPSED -lt 30 ]; then
        PHASE=1
        # Spawn 1-2 concurrent sessions
        persona_roll=$((RANDOM % 100))
        if [ $persona_roll -lt 70 ]; then
            spawn_user_session "window_shopper"
        elif [ $persona_roll -lt 90 ]; then
            spawn_user_session "casual_buyer"
        else
            spawn_user_session "power_buyer"
        fi
        sleep 2
        
    # Phase 2 (30-60s): Lunch Rush - Burst Traffic
    elif [ $ELAPSED -lt 60 ]; then
        PHASE=2
        # Spawn 3-5 concurrent sessions
        for i in {1..3}; do
            persona_roll=$((RANDOM % 100))
            if [ $persona_roll -lt 50 ]; then
                spawn_user_session "window_shopper"
            elif [ $persona_roll -lt 90 ]; then
                spawn_user_session "casual_buyer"
            else
                spawn_user_session "power_buyer"
            fi
        done
        sleep 1
        
    # Phase 3 (60-90s): Afternoon - Incidents
    elif [ $ELAPSED -lt 90 ]; then
        PHASE=3
        
        # Trigger payment outage once
        if [ $ELAPSED -eq 65 ] && [ "$INCIDENT_TRIGGERED" = false ]; then
            simulate_payment_outage &
            INCIDENT_TRIGGERED=true
        fi
        
        # Trigger stock depletion
        if [ $ELAPSED -eq 75 ]; then
            exhaust_popular_stock &
        fi
        
        # Normal traffic with bots
        persona_roll=$((RANDOM % 100))
        if [ $persona_roll -lt 50 ]; then
            spawn_user_session "window_shopper"
        elif [ $persona_roll -lt 75 ]; then
            spawn_user_session "casual_buyer"
        elif [ $persona_roll -lt 90 ]; then
            spawn_user_session "power_buyer"
        else
            spawn_user_session "bot"
        fi
        sleep 1.5
        
    # Phase 4 (90-120s): Evening Peak - Sustained High Traffic
    else
        PHASE=4
        # Spawn 2-4 concurrent sessions with higher buyer ratio
        for i in {1..2}; do
            persona_roll=$((RANDOM % 100))
            if [ $persona_roll -lt 40 ]; then
                spawn_user_session "window_shopper"
            elif [ $persona_roll -lt 85 ]; then
                spawn_user_session "casual_buyer"
            else
                spawn_user_session "power_buyer"
            fi
        done
        sleep 1
    fi
done

# Wait for all background sessions to complete
echo ""
echo "Waiting for active sessions to complete..."
wait

echo ""
echo "========================================="
echo "Traffic Generation Complete!"
echo "========================================="
echo "Duration: ${DURATION}s"
echo "Total Sessions: $SESSION_COUNT"
echo "Total Requests: $TOTAL_REQUESTS"
echo "Successful Responses: $SUCCESS_COUNT"
echo ""
echo "Error Breakdown:"
echo "  404 (Not Found): $ERROR_404_COUNT"
echo "  409 (Conflict/Stock): $ERROR_409_COUNT"
echo "  422 (Validation): $ERROR_422_COUNT"
echo ""
echo "Order Statistics:"
echo "  Successful Orders: $ORDER_SUCCESS_COUNT"
echo "  Failed Orders: $ORDER_FAILURE_COUNT"
if [ $((ORDER_SUCCESS_COUNT + ORDER_FAILURE_COUNT)) -gt 0 ]; then
    echo "  Success Rate: $(awk "BEGIN {printf \"%.1f%%\", ($ORDER_SUCCESS_COUNT/($ORDER_SUCCESS_COUNT+$ORDER_FAILURE_COUNT))*100}")"
fi
echo ""

# Optional: Verify metrics in Prometheus
if [ "$VERIFY_PROMETHEUS" = true ]; then
    echo "========================================="
    echo "Verifying Metrics in Prometheus"
    echo "========================================="
    
    PROM_URL="http://localhost:9090"
    
    echo "Checking Prometheus availability..."
    if ! curl -s -f "$PROM_URL/-/healthy" > /dev/null 2>&1; then
        echo -e "${YELLOW}⚠ Prometheus not reachable at $PROM_URL${NC}"
        echo "Skipping verification."
    else
        echo -e "${GREEN}✓ Prometheus is healthy${NC}"
        echo ""
        
        echo "Querying metrics..."
        
        # HTTP Server Metrics (RED)
        HTTP_METRICS=$(curl -s "$PROM_URL/api/v1/query?query=http_server_request_duration_seconds_count" | jq -r '.data.result | length')
        echo "  HTTP Server Metrics: $HTTP_METRICS series"
        
        # Order Metrics
        ORDER_ATTEMPTS=$(curl -s "$PROM_URL/api/v1/query?query=orders_checkout_attempts_total" | jq -r '.data.result[0].value[1] // "0"')
        echo "  Order Attempts: $ORDER_ATTEMPTS"
        
        # Funnel Metrics
        FUNNEL_STARTED=$(curl -s "$PROM_URL/api/v1/query?query=checkout_funnel_started_total" | jq -r '.data.result[0].value[1] // "0"')
        FUNNEL_COMPLETED=$(curl -s "$PROM_URL/api/v1/query?query=checkout_funnel_completed_total" | jq -r '.data.result[0].value[1] // "0"')
        echo "  Funnel Started: $FUNNEL_STARTED"
        echo "  Funnel Completed: $FUNNEL_COMPLETED"
        
        if [ "$FUNNEL_STARTED" != "0" ]; then
            CONVERSION=$(awk "BEGIN {printf \"%.1f%%\", ($FUNNEL_COMPLETED/$FUNNEL_STARTED)*100}")
            echo "  Conversion Rate: $CONVERSION"
        fi
        
        # Inventory Metrics
        INV_ATTEMPTS=$(curl -s "$PROM_URL/api/v1/query?query=inventory_reservation_attempts_total" | jq -r '.data.result[0].value[1] // "0"')
        echo "  Inventory Reservations: $INV_ATTEMPTS"
        
        # Gateway Metrics
        GATEWAY_METRICS=$(curl -s "$PROM_URL/api/v1/query?query=http_client_request_duration_seconds_count" | jq -r '.data.result | length')
        echo "  Gateway Upstream Metrics: $GATEWAY_METRICS series"
        
        echo ""
        echo -e "${GREEN}✓ Metrics verification complete${NC}"
    fi
    echo ""
fi

echo "View metrics in Prometheus: http://localhost:9090"
echo "View dashboards in Grafana: http://localhost:3000"
echo ""
echo "Example PromQL queries:"
echo "  Request rate: rate(http_server_request_duration_seconds_count[5m])"
echo "  Error rate: (sum(rate(http_server_request_duration_seconds_count{http_response_status_code=~\"4..|5..\"}[5m])) / sum(rate(http_server_request_duration_seconds_count[5m])) * 100) or vector(0)"
echo "  P95 latency: histogram_quantile(0.95, rate(http_server_request_duration_seconds_bucket[5m]))"
echo "  Conversion rate: ((rate(checkout_funnel_completed_total[5m]) / rate(checkout_funnel_started_total[5m])) * 100) or vector(0)"
echo ""
