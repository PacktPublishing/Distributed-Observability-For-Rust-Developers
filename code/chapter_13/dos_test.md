# DoS Load-Shedding Test — Chapter 13

This document describes how to run and interpret the DoS attack simulation for
Chapter 13: *Detecting Attacks in Traces*.

---

## What it tests

The gateway (`otelmart`) enforces a **concurrency limit of 1 000 in-flight
requests** using Tower's `ServiceBuilder`:

```rust
ServiceBuilder::new()
    .layer(HandleErrorLayer::new(|e: BoxError| async move {
        if e.is::<Overloaded>() {
            StatusCode::SERVICE_UNAVAILABLE   // HTTP 503
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }))
    .load_shed()
    .concurrency_limit(1000)
```

When the limit is exceeded `load_shed()` immediately returns an `Overloaded`
error instead of queuing the request, and `HandleErrorLayer` converts that into
an HTTP 503. The `track_load_shedding` middleware then records a span attribute
on every shed request:

```
otelmart.security.event.type = "dos_protection"
```

The OTel Collector tail-sampling policy `security-dos-dropped` retains **100 %**
of spans carrying that attribute and forwards them to Jaeger.

---

## Prerequisites

| Requirement | Notes |
|-------------|-------|
| Docker stack running | `docker compose up -d` from `code/chapter_13/` |
| `ab` (ApacheBench) | Pre-installed on macOS; `sudo apt install apache2-utils` on Linux |
| `curl` | Used for the health-check pre-flight |

Verify the stack is healthy:

```bash
curl http://localhost:4200/health
# Expected: {"status":"healthy","service":"otelmart"}
```

---

## Running the test

```bash
# From the repo root or code/chapter_13/
bash scripts/generate_dos_traffic.sh
```

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GATEWAY_URL` | `http://localhost:4200` | Base URL of the otelmart gateway |
| `ENDPOINT` | `/api/products` | Path to flood |
| `CONCURRENCY` | `150` | Concurrent workers (set above 1000 to trigger 503s) |
| `DURATION` | `30` | Unused when `ab` is available (requests capped at `CONCURRENCY * 5`) |

To trigger load shedding run with concurrency above 1 000:

```bash
CONCURRENCY=1100 bash scripts/generate_dos_traffic.sh
```

### Expected output

```
=========================================
  DoS Attack Traffic Simulator
  Chapter 13: Detecting Attacks in Traces
=========================================

Target URL   : http://localhost:4200/api/products
Concurrency  : 1100 workers
Duration     : 30s

✓ Gateway is healthy — starting flood

Flooding...

=========================================
  Flood complete
-----------------------------------------
  Total requests sent  : 5500
  HTTP 2xx (served)    : 1084
  HTTP 503 (load shed) : 4416
  Other errors         : 0
-----------------------------------------
✓ Load shedding triggered (4416 requests dropped)
```

Around 80 % of requests will receive HTTP 503 when the 1 000-request limit is
saturated at a concurrency of 1 100.

---

## Verifying the traces in Jaeger

1. Open **Jaeger UI** at <http://localhost:16686>.
2. In the **Search** panel set:
   - **Service**: `otelmart`
   - **Tags**: `otelmart.security.event.type=dos_protection`
3. Click **Find Traces**.

You should see traces for every request that was load-shed, each containing:

| Span attribute | Value |
|----------------|-------|
| `otelmart.security.event.type` | `dos_protection` |
| `http.response.status_code` | `503` |
| `otel.name` | `GET /api/products` |

The tail-sampling `security-dos-dropped` policy retains 100 % of these spans
regardless of the baseline 1 % probabilistic policy.

---

## Verifying the OTel Collector

```bash
docker logs otel-collector 2>&1 | grep -Ei "error|warn" | tail -10
```

After a correctly sized test (5 500 requests or fewer) you should see **no**
`ResourceExhausted` errors. Those errors appear only when the batch size exceeds
Jaeger's 4 MB gRPC message limit, which the `batch` processor prevents by
capping exports at 512 spans per batch.

---

## How the pipeline fits together

```
ab (1100 concurrent)
        │
        ▼
otelmart :4200
  ├── requests <= 1000 in-flight → forwarded to products-service → HTTP 200
  └── requests > 1000 in-flight → load_shed() → HTTP 503
             │
             └── track_load_shedding middleware
                   records otelmart.security.event.type="dos_protection"
                        │
                        ▼
             OTLP gRPC → otel-collector :4317
                        │
                  tail_sampling processor
                  policy: security-dos-dropped
                  (string_attribute key=otelmart.security.event.type
                                    value=dos_protection)
                  → 100 % sampling decision
                        │
                  batch processor (max 512 spans/batch)
                        │
                        ▼
                  Jaeger :16686
```

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| `0` HTTP 503s | Concurrency too low (≤ 1000) | Set `CONCURRENCY=1100` or higher |
| `ResourceExhausted` in collector logs | Batch size too large | Check `otel-collector-config.yaml` `send_batch_max_size: 512` |
| No services visible in Jaeger | Collector not routing to Jaeger | `docker logs otel-collector` — check for startup errors |
| `ERROR: Gateway not reachable` | Stack not running | `docker compose up -d && docker compose ps` |
