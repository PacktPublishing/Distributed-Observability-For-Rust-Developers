#!/usr/bin/env bash
# DoS Attack Traffic Simulator for Chapter 13
#
# Simulates an application-layer Denial of Service attack by flooding the
# OtelMart gateway with a high volume of concurrent requests. This script is
# intended for use against a local staging environment to validate that:
#
#   1. The ConcurrencyLimitLayer returns HTTP 503 when saturated
#   2. The track_load_shedding middleware records the
#      `otelmart.security.event.type = "dos_protection"` span attribute
#   3. The OTel Collector tail-sampling `security-dos-dropped` policy
#      captures and forwards 100% of those saturated traces
#
# Usage:
#   ./scripts/generate_dos_traffic.sh
#   ./scripts/generate_dos_traffic.sh --concurrency 200 --duration 60
#   GATEWAY_URL=http://localhost:4200 ./scripts/generate_dos_traffic.sh
#
# WARNING: Run this only against a local staging environment.
#          Do NOT run against a production system.

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
GATEWAY_URL="${GATEWAY_URL:-http://localhost:4200}"
CONCURRENCY="${CONCURRENCY:-150}"   # concurrent workers (above 1000 triggers 503s)
DURATION="${DURATION:-30}"          # seconds to run the flood
ENDPOINT="${ENDPOINT:-/api/products}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --concurrency)
      CONCURRENCY="$2"; shift 2 ;;
    --duration)
      DURATION="$2"; shift 2 ;;
    --endpoint)
      ENDPOINT="$2"; shift 2 ;;
    --url)
      GATEWAY_URL="$2"; shift 2 ;;
    -h|--help)
      cat <<'USAGE'
Usage: generate_dos_traffic.sh [options]

Options:
  --url <url>              Base gateway URL (default: http://localhost:4200)
  --endpoint <path>        Target endpoint    (default: /api/products)
  --concurrency <number>   Concurrent workers (default: 150)
  --duration <seconds>     How long to flood  (default: 30)
  -h, --help               Show this help
USAGE
      exit 0 ;;
    *)
      echo "Unknown option: $1"; exit 1 ;;
  esac
done

TARGET_URL="${GATEWAY_URL%/}${ENDPOINT}"

echo -e "${RED}=========================================${NC}"
echo -e "${RED}  DoS Attack Traffic Simulator           ${NC}"
echo -e "${RED}  Chapter 13: Detecting Attacks in Traces${NC}"
echo -e "${RED}=========================================${NC}"
echo ""
echo -e "${YELLOW}WARNING: Run this ONLY against a local staging environment.${NC}"
echo ""
echo "Target URL   : $TARGET_URL"
echo "Concurrency  : $CONCURRENCY workers"
echo "Duration     : ${DURATION}s"
echo ""

# ---------------------------------------------------------------------------
# Pre-flight check
# ---------------------------------------------------------------------------
if ! curl -sf "$GATEWAY_URL/health" > /dev/null 2>&1; then
  echo -e "${RED}ERROR: Gateway not reachable at $GATEWAY_URL${NC}"
  echo "Make sure the services are running: docker-compose up"
  exit 1
fi
echo -e "${GREEN}✓ Gateway is healthy — starting flood${NC}"
echo ""

# ---------------------------------------------------------------------------
# Determine the flood tool to use.
# `ab` (ApacheBench) is preferred: it multiplexes all connections inside one
# process so it achieves true CONCURRENCY-level parallelism, which is what
# we need to actually saturate the ConcurrencyLimitLayer.
# Bash subprocesses (curl &) cannot reliably maintain 1000+ in-flight
# requests simultaneously because process-spawning latency lets earlier
# requests finish before later ones start.
# ---------------------------------------------------------------------------
if command -v ab > /dev/null 2>&1; then
  USE_AB=true
else
  USE_AB=false
  echo -e "${YELLOW}NOTE: 'ab' (ApacheBench) not found — falling back to curl workers.${NC}"
  echo "      Install apache2-utils (Linux) or use the built-in macOS ab for best results."
  echo ""
fi

echo -e "${CYAN}Flooding...${NC}"
echo ""

TMPDIR_DOS=$(mktemp -d)
trap 'rm -rf "$TMPDIR_DOS"' EXIT
AB_OUTPUT="$TMPDIR_DOS/ab_output.txt"

TOTAL=0; OK=0; SHED=0; ERR=0

if [[ "$USE_AB" == true ]]; then
  # ---------------------------------------------------------------------------
  # ApacheBench flood: -n total requests, -c concurrency
  # We cap at CONCURRENCY * 5 requests to avoid overwhelming the tail-sampling
  # buffer (num_traces=100000). With -c 1100 that is 5500 requests — enough to
  # demonstrate load shedding without flooding the OTel Collector.
  # -r: do not exit on socket-receive errors (expected when 503s arrive)
  # ---------------------------------------------------------------------------
  TOTAL_REQUESTS=$(( CONCURRENCY * 5 ))
  ab -n "$TOTAL_REQUESTS" -c "$CONCURRENCY" -r \
    "$TARGET_URL" > "$AB_OUTPUT" 2>&1 || true

  # Parse ab's summary output
  TOTAL=$(grep "^Complete requests:" "$AB_OUTPUT" | awk '{print $3}')
  NON2XX=$(grep "^Non-2xx responses:" "$AB_OUTPUT" | awk '{print $3}')
  NON2XX="${NON2XX:-0}"
  OK=$(( TOTAL - NON2XX ))
  SHED="$NON2XX"    # in this scenario all non-2xx are 503 load-shed responses
  ERR=0

else
  # ---------------------------------------------------------------------------
  # Fallback: bash worker flood (less reliable for high concurrency)
  # ---------------------------------------------------------------------------
  RESULTS_FILE="$TMPDIR_DOS/results"
  touch "$RESULTS_FILE"

  send_request() {
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" \
      --max-time 5 --connect-timeout 2 "$TARGET_URL" 2>/dev/null || echo "000")
    echo "$status" >> "$RESULTS_FILE"
  }
  export -f send_request
  export TARGET_URL RESULTS_FILE

  END_TIME=$(( $(date +%s) + DURATION ))
  BATCH=0
  while [[ $(date +%s) -lt $END_TIME ]]; do
    for (( i=0; i<CONCURRENCY; i++ )); do send_request & done
    wait
    BATCH=$(( BATCH + 1 ))
    if (( BATCH % 5 == 0 )); then
      _T=$(wc -l < "$RESULTS_FILE" | tr -d ' ')
      _S=0; _O=0
      _S=$(grep -c "^503$" "$RESULTS_FILE" 2>/dev/null) || _S=0
      _O=$(grep -c "^2"    "$RESULTS_FILE" 2>/dev/null) || _O=0
      printf "\r  Requests: %-6d  |  200 OK: %-6d  |  503 Shed: %-6d" "$_T" "$_O" "$_S"
    fi
  done
  wait
  echo ""

  TOTAL=$(wc -l < "$RESULTS_FILE" | tr -d ' ')
  SHED=0; OK=0
  SHED=$(grep -c "^503$" "$RESULTS_FILE" 2>/dev/null) || SHED=0
  OK=$(grep -c "^2"    "$RESULTS_FILE" 2>/dev/null) || OK=0
  ERR=$(( TOTAL - OK - SHED ))
fi

echo ""

echo -e "${CYAN}=========================================${NC}"
echo "  Flood complete"
echo "-----------------------------------------"
printf "  Total requests sent  : %d\n" "$TOTAL"
printf "  HTTP 2xx (served)    : %d\n" "$OK"
printf "  HTTP 503 (load shed) : %d\n" "$SHED"
printf "  Other errors         : %d\n" "$ERR"
echo "-----------------------------------------"

if [[ "$SHED" -gt 0 ]]; then
  echo -e "${GREEN}✓ Load shedding triggered ($SHED requests dropped)${NC}"
  echo ""
  echo "Next steps to validate the chapter_13 pipeline:"
  echo "  1. Open Jaeger at http://localhost:16686"
  echo "  2. Search for service 'otelmart', tag 'otelmart.security.event.type=dos_protection'"
  echo "  3. Confirm those traces were retained at 100% by the tail-sampling policy"
  echo "  4. Check Grafana (http://localhost:3000) for the"
  echo "     'otelmart.security.requests' counter spike"
else
  echo -e "${YELLOW}⚠ No 503s observed — concurrency limit not reached.${NC}"
  echo "  Try increasing --concurrency above 1000 (the ConcurrencyLimitLayer cap)"
fi
echo ""
