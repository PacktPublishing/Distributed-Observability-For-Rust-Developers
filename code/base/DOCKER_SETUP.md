# Docker Compose Setup Guide

## Overview

The docker-compose setup automatically orchestrates the following services in order:

1. **PostgreSQL Database** - Sets up with schema initialization
2. **Products Service** - Runs database migrations
3. **Data Ingestion** - Loads 2000 products into the database
4. **OtelMart Service** - Main e-commerce application (Rust + Angular)

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
postgres (healthy) → initializes schemas from init/*.sql
    ↓
products, inventory, orders (started) → connect to database
    ↓
data-ingestion (waits 10s) → loads 2000 products + specs
    ↓
otelmart (started) → serves UI and API
```

## Services

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
