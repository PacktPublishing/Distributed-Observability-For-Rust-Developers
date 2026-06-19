# Docker Compose Setup Guide

## Overview

The docker-compose setup automatically orchestrates the following services in order:

1. **Jaeger** - Distributed tracing backend for viewing traces
2. **Prometheus** - Metrics backend receiving OTLP push from services
3. **Grafana** - Dashboard visualization for metrics and traces
4. **PostgreSQL Database** - Sets up with schema initialization
5. **Products Service** - Runs database migrations
6. **Inventory Service** - Stock and pricing management
7. **Orders Service** - Order processing and checkout
8. **Data Ingestion** - Loads 2000 products into the database
9. **OtelMart Service** - Main e-commerce application (Rust + Angular)

## Quick Start

```bash
# Start all services (builds images if needed)
docker-compose up --build -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down

# Stop and remove all data (fresh start)
docker-compose down -v
```

## Service Orchestration Flow

```
jaeger (started) → receives traces via OTLP gRPC (port 4317)
prometheus (started) → receives metrics via OTLP HTTP (port 9090)
grafana (started) → queries Prometheus and Jaeger for dashboards
    ↓
postgres (healthy) → initializes schemas from init/*.sql
    ↓
products, inventory, orders (started) → connect to database, send traces to Jaeger, push metrics to Prometheus
    ↓
data-ingestion (waits 10s) → loads 2000 products + specs
    ↓
otelmart (started) → serves UI and API, sends traces to Jaeger, pushes metrics to Prometheus
```

## Services

### Jaeger (Ports 16686, 4317, 4318)
- **Container**: `jaeger`
- **UI**: http://localhost:16686
- **OTLP gRPC**: Port 4317 (used by services for traces)
- **OTLP HTTP**: Port 4318
- **Purpose**: Distributed tracing visualization

### Prometheus (Port 9090)
- **Container**: `prometheus`
- **UI**: http://localhost:9090
- **OTLP Receiver**: `http://prometheus:9090/api/v1/otlp/v1/metrics`
- **Config**: `prometheus.yml` (minimal — no scrape targets, uses OTLP push)
- **Purpose**: Metrics storage via native OTLP ingestion (Prometheus 3.0)

### Grafana (Port 3000)
- **Container**: `grafana`
- **UI**: http://localhost:3000
- **Default Login**: admin / admin
- **Purpose**: Dashboard visualization for metrics (Prometheus) and traces (Jaeger)
- **Data Sources**: Add manually — Prometheus (`http://prometheus:9090`), Jaeger (`http://jaeger:16686`)

### PostgreSQL (Port 5433)
- **Container**: `opentel-postgres`
- **Database**: `opentel_db`
- **User**: `opentel_user`
- **Password**: `opentel_pass`
- **Volume**: `opentel_postgres_data`
- **Schema**: Initialized from `schemas/init/*.sql`

### Products Service (Port 3001)
- **Container**: `products-service`
- **API**: http://localhost:3001
- **Endpoints**:
  - `GET /products` - List products
  - `GET /products/{id}` - Get product details
  - `PUT /products/{id}/ratings` - Rate product

### Data Ingestion
- **Container**: `data-ingestion`
- **Type**: One-time job (exits after completion)
- **Data Loaded**:
  - 2000 products
  - 1989 product specifications
- **Source**: `schemas/data/products_data_enhanced.sql`

### OtelMart (Port 4200)
- **Container**: `otelmart`
- **UI**: http://localhost:4200
- **Backend**: Rust (Axum)
- **Frontend**: Angular

## Data Verification

```bash
# Check products count
PGPASSWORD=opentel_pass psql -h localhost -p 5433 -U opentel_user -d opentel_db \\
  -c "SELECT COUNT(*) FROM products.products;"

# Check data ingestion logs
docker logs data-ingestion

# Test products API
curl http://localhost:3001/products?page=1&page_size=5

# Test OtelMart UI
curl http://localhost:4200
```

## Troubleshooting

### Reset Everything
```bash
docker-compose down -v
docker-compose up --build -d
```

### View Service Logs
```bash
# All services
docker-compose logs -f

# Specific service
docker logs postgres
docker logs products-service
docker logs data-ingestion
docker logs otelmart
```

### Check Service Status
```bash
docker-compose ps
```

### Database Access
```bash
# Connect to database
PGPASSWORD=opentel_pass psql -h localhost -p 5433 -U opentel_user -d opentel_db

# List tables
\dt products.*
```

## Architecture Notes

- **Network**: All services communicate via `app-network`
- **Health Checks**: PostgreSQL has health check; services wait for it
- **Data Persistence**: PostgreSQL data stored in `opentel_postgres_data` volume
- **Build Context**: All services build from the `app/` directory
- **Restart Policy**: Services restart automatically except data-ingestion (one-time job)
- **Tracing**: All Rust services export traces to Jaeger via OTLP gRPC (port 4317)
- **Metrics**: All Rust services push metrics to Prometheus via OTLP HTTP (`/api/v1/otlp/v1/metrics`)
- **Telemetry Env Vars**: Each service sets `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` and `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`

## Viewing Traces in Jaeger

1. Open Jaeger UI: http://localhost:16686
2. Select a service from the dropdown (e.g., `otelmart`, `products`, `inventory`, `orders`)
3. Click "Find Traces" to see recent traces
4. Click on a trace to view the span waterfall diagram

### Example: Tracing a Product Request

1. Make a request: `curl http://localhost:4200/api/products?page=1&page_size=5`
2. Open Jaeger UI: http://localhost:16686
3. Select `otelmart` service
4. Find the trace - you'll see spans for:
   - `otelmart`: Incoming HTTP request
   - `proxy_request`: Outgoing call to products service
   - `products`: Handling the request
   - `list_products`: The handler function

### Example: Tracing an Order

1. Create an order through the UI or API
2. In Jaeger, select `orders` service
3. The trace shows:
   - `create_order` span in orders service
   - Child spans for product validation (calls to products service)
   - Child spans for stock reservation (calls to inventory service)

## Development Workflow

1. Make code changes
2. Rebuild specific service:
   ```bash
   docker-compose up --build -d products
   # or
   docker-compose up --build -d otelmart
   ```
3. View logs: `docker-compose logs -f <service>`
4. Test changes

## Profiling and Optimizations (Chapter 9)

You can toggle optimized versions of the code (fixing the Chapter 9 performance bottlenecks) by using the `ENABLE_OPTIMIZATIONS` environment variable.

### Running with Optimizations Enabled
To toggle optimizations and fix the bottlenecks, set the environment variable and recreate the container:

```bash
ENABLE_OPTIMIZATIONS=1 docker-compose up -d orders
```

### Memory Profiling with jemalloc
The `orders` image is always compiled with `jemalloc` support, and `docker-compose.yml` automatically provides the `MALLOC_CONF` variable for it.
To generate a heap profile, simply run the service as normal:
```bash
docker-compose up -d orders
```
*(When you stop the container later using `docker-compose stop orders`, jemalloc will flush the `.heap` file to the container's filesystem).*

## Viewing Metrics in Grafana

1. Open Grafana: http://localhost:3000 (login: admin / admin)
2. Add Prometheus data source: **Connections → Data Sources → Add → Prometheus** → URL: `http://prometheus:9090`
3. Add Jaeger data source: **Connections → Data Sources → Add → Jaeger** → URL: `http://jaeger:16686`
4. Use **Explore** to run PromQL queries

### Verify Metrics Are Flowing

```bash
# Query Prometheus for HTTP request metrics from all services
curl -s 'http://localhost:9090/api/v1/query?query=http_server_request_duration_seconds_count' | jq '.data.result[] | {job: .metric.job}'
```

Expected output shows all four services:
```json
{"job": "products"}
{"job": "inventory"}
{"job": "orders"}
{"job": "otelmart"}
```

### Useful PromQL Queries

```promql
# Request rate per service
sum(rate(http_server_request_duration_seconds_count[5m])) by (job)

# Error rate percentage
sum(rate(http_server_request_duration_seconds_count{http_response_status_code=~"4..|5.."}[5m])) by (job)
/ sum(rate(http_server_request_duration_seconds_count[5m])) by (job) * 100

# p99 latency per service
histogram_quantile(0.99, sum(rate(http_server_request_duration_seconds_bucket[5m])) by (le, job))

# Checkout success rate
(sum(rate(orders_checkout_attempts_total[5m])) - sum(rate(orders_checkout_failures_total[5m])))
/ sum(rate(orders_checkout_attempts_total[5m])) * 100

# DB pool utilization
db_pool_utilization_percent
```

## Clean Build

```bash
# Remove all containers, volumes, and images
docker-compose down -v --rmi all

# Rebuild everything from scratch
docker-compose up --build -d
```
