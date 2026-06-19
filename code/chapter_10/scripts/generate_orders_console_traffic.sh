#!/usr/bin/env bash
# Generate concurrent HTTP traffic for the Orders service.
#
# This script is intended for Tokio Console demos:
# 1) Start tokio-console in one terminal
# 2) Run this script in another terminal
# 3) Observe task/activity updates in the console UI
#
# Example:
#   ./scripts/generate_orders_console_traffic.sh
#   ./scripts/generate_orders_console_traffic.sh --requests 5000 --concurrency 40
#   ./scripts/generate_orders_console_traffic.sh --url http://localhost:4200 --endpoint /api/orders

set -euo pipefail

# Default configuration values for quick local testing.
BASE_URL="http://localhost:4200"
ENDPOINT="/api/orders"
TOTAL_REQUESTS=2000
CONCURRENCY=20
METHOD="GET"
BODY=""

# Print usage information for CLI help.
usage() {
  cat <<'USAGE'
Usage: generate_orders_console_traffic.sh [options]

Options:
  --url <url>             Base URL (default: http://localhost:4200)
  --endpoint <path>       Endpoint path (default: /api/orders)
  --requests <number>     Total requests to send (default: 2000)
  --concurrency <number>  Number of concurrent workers (default: 20)
  --method <GET|POST>     HTTP method (default: GET)
  --body <json>           JSON body for POST method
  -h, --help              Show this help
USAGE
}

# Parse command-line arguments.
while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      BASE_URL="$2"
      shift 2
      ;;
    --endpoint)
      ENDPOINT="$2"
      shift 2
      ;;
    --requests)
      TOTAL_REQUESTS="$2"
      shift 2
      ;;
    --concurrency)
      CONCURRENCY="$2"
      shift 2
      ;;
    --method)
      METHOD="${2^^}"
      shift 2
      ;;
    --body)
      BODY="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      usage
      exit 1
      ;;
  esac
done

# Validate numeric arguments early to avoid silent errors.
if ! [[ "$TOTAL_REQUESTS" =~ ^[0-9]+$ ]] || [[ "$TOTAL_REQUESTS" -le 0 ]]; then
  echo "--requests must be a positive integer"
  exit 1
fi

if ! [[ "$CONCURRENCY" =~ ^[0-9]+$ ]] || [[ "$CONCURRENCY" -le 0 ]]; then
  echo "--concurrency must be a positive integer"
  exit 1
fi

if [[ "$METHOD" != "GET" && "$METHOD" != "POST" ]]; then
  echo "--method must be GET or POST"
  exit 1
fi

if [[ "$METHOD" == "POST" && -z "$BODY" ]]; then
  echo "--body is required when --method POST is used"
  exit 1
fi

TARGET_URL="${BASE_URL%/}${ENDPOINT}"

# Verify target endpoint is reachable before starting load.
if ! curl -sS -o /dev/null --connect-timeout 2 --max-time 3 "$TARGET_URL"; then
  echo "Target not reachable: $TARGET_URL"
  exit 1
fi

# Create a temp directory to store per-worker counters.
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# Worker sends one slice of requests and writes success/fail counts.
worker() {
  local worker_id="$1"
  local success=0
  local failed=0

  # Distribute requests across workers with a stride pattern.
  local i
  for ((i=worker_id; i<=TOTAL_REQUESTS; i+=CONCURRENCY)); do
    if [[ "$METHOD" == "GET" ]]; then
      if curl -sS -o /dev/null "$TARGET_URL"; then
        success=$((success + 1))
      else
        failed=$((failed + 1))
      fi
    else
      if curl -sS -o /dev/null -X POST "$TARGET_URL" \
        -H 'Content-Type: application/json' \
        --data "$BODY"; then
        success=$((success + 1))
      else
        failed=$((failed + 1))
      fi
    fi
  done

  printf '%s %s\n' "$success" "$failed" > "$TMP_DIR/worker_${worker_id}.txt"
}

echo "Generating traffic..."
echo "  Target      : $TARGET_URL"
echo "  Method      : $METHOD"
echo "  Requests    : $TOTAL_REQUESTS"
echo "  Concurrency : $CONCURRENCY"

# Spawn workers in background for concurrent traffic generation.
for ((w=1; w<=CONCURRENCY; w++)); do
  worker "$w" &
done

# Wait for all workers to complete.
wait

# Aggregate final counters from worker outputs.
TOTAL_SUCCESS=0
TOTAL_FAILED=0
for result_file in "$TMP_DIR"/worker_*.txt; do
  read -r s f < "$result_file"
  TOTAL_SUCCESS=$((TOTAL_SUCCESS + s))
  TOTAL_FAILED=$((TOTAL_FAILED + f))
done

echo "Done."
echo "  Successful requests: $TOTAL_SUCCESS"
echo "  Failed requests    : $TOTAL_FAILED"
