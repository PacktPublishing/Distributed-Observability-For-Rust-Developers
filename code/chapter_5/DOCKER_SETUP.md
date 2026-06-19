# Docker Compose Setup Guide

## Overview

The docker-compose setup automatically orchestrates the following services in order:

1. **Jaeger** - Distributed tracing backend for viewing traces
2. **PostgreSQL Database** - Sets up with schema initialization
3. **Products Service** - Runs database migrations
4. **Inventory Service** - Stock and pricing management
5. **Orders Service** - Order processing and checkout
6. **Data Ingestion** - Loads 2000 products into the database
7. **OtelMart Service** - Main e-commerce application (Rust + Angular)

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
jaeger (started) → receives traces via OTLP (port 4317)
    ↓
postgres (healthy) → initializes schemas from init/*.sql
    ↓
products, inventory, orders (started) → connect to database, send traces to Jaeger
    ↓
data-ingestion (waits 10s) → loads 2000 products + specs
    ↓
otelmart (started) → serves UI and API, sends traces to Jaeger
```

## Services

### Jaeger (Ports 16686, 4317, 4318)
- **Container**: `jaeger`
- **UI**: http://localhost:16686
- **OTLP gRPC**: Port 4317 (used by services)
- **OTLP HTTP**: Port 4318
- **Purpose**: Distributed tracing visualization

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
- **Tracing**: All Rust services export traces to Jaeger via OTLP gRPC

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

## Clean Build

```bash
# Remove all containers, volumes, and images
docker-compose down -v --rmi all

# Rebuild everything from scratch
docker-compose up --build -d
```
