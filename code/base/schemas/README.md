# PostgreSQL Multi-Schema Database for Ecommerce Microservices

Single PostgreSQL database with **4 schemas** (namespaces) for microservices architecture.

## Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                PostgreSQL Database: ecommerce                          │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐  │
│  │ products │  │inventory │  │  orders  │  │       users          │  │
│  │  schema  │  │  schema  │  │  schema  │  │       schema         │  │
│  ├──────────┤  ├──────────┤  ├──────────┤  ├──────────────────────┤  │
│  │• categrs │  │• product_│  │• orders  │  │• users               │  │
│  │• products│  │  invntry │  │• order_  │  │• user_sessions       │  │
│  │• product_│  │• product_│  │  items   │  │• user_addresses      │  │
│  │  specs   │  │  pricing │  │• shipping│  │• user_profiles       │  │
│  │• product_│  │• price_  │  │  _addrs  │  │• guest_users         │  │
│  │  reviews │  │  history │  │• payments│  │                      │  │
│  │          │  │• invntry_│  │• shipmnts│  │                      │  │
│  │(4 tables)│  │  trans   │  │          │  │(5 tables)            │  │
│  │          │  │          │  │(5 tables)│  │                      │  │
│  │          │  │(4 tables)│  │          │  │                      │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────────────┘  │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
      ↑               ↑               ↑                   ↑
      │               │               │                   │
   Products      Inventory        Orders             Users/Auth
   Service        Service         Service             Service
  (Port 3001)    (Port 3002)     (Port 3003)        (Port 3004)
```

**Total: 18 tables across 4 schemas in 1 database**

---

## Quick Start

### 1. Build and Run with Docker Compose

```bash
cd schemas

# Copy environment file
cp .env.example .env

# Build and start the database
docker-compose up -d

# Check logs
docker-compose logs -f

# Verify schemas created
docker-compose exec postgres psql -U ecommerce_user -d ecommerce -c "\dn"
```

### 2. Build Docker Image Only

```bash
cd schemas

# Build the image
docker build -t ecommerce-postgres:latest .

# Run the container
docker run -d \
  --name ecommerce_postgres \
  -e POSTGRES_DB=ecommerce \
  -e POSTGRES_USER=ecommerce_user \
  -e POSTGRES_PASSWORD=ecommerce_password \
  -p 5432:5432 \
  -v postgres_data:/var/lib/postgresql/data \
  ecommerce-postgres:latest

# Check logs
docker logs -f ecommerce_postgres
```

### 3. Connect to Database

```bash
# Using docker-compose
docker-compose exec postgres psql -U ecommerce_user -d ecommerce

# Using docker run
docker exec -it ecommerce_postgres psql -U ecommerce_user -d ecommerce

# From host machine (if psql is installed)
psql -h localhost -U ecommerce_user -d ecommerce
```

---

## Database Schema Details

### Schema: `products`

**Tables (4):**
- `categories` - Product categories
- `products` - Product catalog with full-text search
- `product_specifications` - Detailed product specs
- `product_reviews` - Customer ratings and reviews

**Views:**
- `v_products_with_category` - Products with category info
- `v_product_ratings` - Aggregated ratings

**Sample Data:**
- 8 default categories pre-loaded

### Schema: `inventory`

**Tables (4):**
- `product_inventory` - Stock levels and availability
- `product_pricing` - Current prices and discounts
- `price_history` - Historical pricing data
- `inventory_transactions` - Audit trail

**Functions:**
- `reserve_stock(product_uuid, quantity)` - Reserve stock for orders
- `release_stock(product_uuid, quantity)` - Release reserved stock
- `confirm_stock_sale(product_uuid, quantity, order_uuid)` - Confirm sale

**Views:**
- `v_product_inventory_pricing` - Combined inventory + pricing
- `v_low_stock_products` - Products needing restock

**Triggers:**
- Auto-calculate discounts
- Auto-update stock status

### Schema: `orders`

**Tables (5):**
- `orders` - Customer orders
- `order_items` - Line items with product snapshots
- `shipping_addresses` - Delivery addresses
- `payments` - Simulated payment records
- `shipments` - Shipment tracking

**Functions:**
- `generate_order_number()` - Creates order number (ORD-YYYYMMDD-XXX)
- `generate_payment_reference()` - Creates payment transaction ID
- `calculate_order_total(order_id)` - Calculates order total

**Views:**
- `v_orders_complete` - Complete order data with all relations
- `v_order_items_detail` - Order items with order info
- `v_recent_orders` - Last 100 orders

**Features:**
- Guest checkout support (no user authentication needed)
- Simulated payment processing
- One shipment per order

### Schema: `users`

**Tables (5):**
- `users` - User accounts with authentication
- `user_sessions` - Active user sessions with JWT tokens
- `user_addresses` - Multiple saved addresses per user
- `user_profiles` - User information and preferences
- `guest_users` - Guest checkout tracking and conversion

**Functions:**
- `generate_session_token()` - Creates session token
- `revoke_user_session(token)` - Logout/revoke session
- `convert_guest_to_user(guest_uuid, user_id)` - Convert guest to registered user

**Views:**
- `v_users_with_addresses` - Users with default shipping/billing addresses
- `v_guest_users` - Guest users with conversion status

**Features:**
- Email/password authentication
- Session management (login/logout)
- Multiple addresses per user (shipping/billing)
- Guest user conversion to registered users
- Email verification support
- Password reset support

---

## Connecting from Microservices

### Connection String

```
postgres://ecommerce_user:ecommerce_password@postgres:5432/ecommerce
```

**For localhost (development):**
```
postgres://ecommerce_user:ecommerce_password@localhost:5432/ecommerce
```

### Schema-Specific Connections

Each microservice should set its own search_path:

**Products Service (Rust example):**
```rust
// Set search path after connection
sqlx::query("SET search_path TO products, public")
    .execute(&pool)
    .await?;
```

**Inventory Service (Rust example):**
```rust
// Inventory needs access to products schema for UUID references
sqlx::query("SET search_path TO inventory, products, public")
    .execute(&pool)
    .await?;
```

**Orders Service (Rust example):**
```rust
// Orders needs access to products schema for UUID references
sqlx::query("SET search_path TO orders, products, public")
    .execute(&pool)
    .await?;
```

**Users Service (Rust example):**
```rust
// Users service is independent, no cross-schema references needed
sqlx::query("SET search_path TO users, public")
    .execute(&pool)
    .await?;
```

### Cross-Schema References

Tables reference each other via **UUIDs** (not integer IDs):

```sql
-- Inventory service referencing Products service
inventory.product_inventory.product_uuid → products.products.uuid

-- Orders service referencing Products service
orders.order_items.product_uuid → products.products.uuid
```

---

## Database Management

### View All Schemas

```sql
\dn
-- or
SELECT schema_name FROM information_schema.schemata;
```

### View Tables in a Schema

```sql
\dt products.*
\dt inventory.*
\dt orders.*
```

### Switch Between Schemas

```sql
SET search_path TO products;
\dt  -- shows products schema tables

SET search_path TO inventory;
\dt  -- shows inventory schema tables
```

### Query Across Schemas

```sql
-- Get product with pricing
SELECT
    p.product_name,
    pr.final_price,
    pi.stock_quantity
FROM products.products p
JOIN inventory.product_pricing pr ON p.uuid = pr.product_uuid
JOIN inventory.product_inventory pi ON p.uuid = pi.product_uuid
WHERE p.is_active = true;
```

---

## File Structure

```
schemas/
├── Dockerfile                    # PostgreSQL container definition
├── docker-compose.yml            # Compose file for easy deployment
├── .env.example                  # Environment variables template
├── README.md                     # This file
└── init/                         # SQL initialization scripts
    ├── 00_init.sql              # Creates schemas and extensions
    ├── inventory_schema_1.sql   # Inventory schema tables
    ├── orders_schema_1.sql      # Orders schema tables
    └── products_schema_1.sql    # Products schema tables
```

**Execution Order:**
1. `00_init.sql` - Creates schemas
2. `inventory_schema_1.sql` - Creates inventory tables
3. `orders_schema_1.sql` - Creates orders tables
4. `products_schema_1.sql` - Creates products tables (includes sample data)

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `POSTGRES_DB` | `ecommerce` | Database name |
| `POSTGRES_USER` | `ecommerce_user` | Database user |
| `POSTGRES_PASSWORD` | `ecommerce_password` | User password |
| `POSTGRES_PORT` | `5432` | External port mapping |

---

## Docker Commands

### Start Database
```bash
docker-compose up -d
```

### Stop Database
```bash
docker-compose down
```

### Stop and Remove Data
```bash
docker-compose down -v  # WARNING: Deletes all data
```

### View Logs
```bash
docker-compose logs -f postgres
```

### Restart Database
```bash
docker-compose restart postgres
```

### Execute SQL Script
```bash
docker-compose exec postgres psql -U ecommerce_user -d ecommerce -f /path/to/script.sql
```

### Backup Database
```bash
docker-compose exec postgres pg_dump -U ecommerce_user ecommerce > backup.sql
```

### Restore Database
```bash
docker-compose exec -T postgres psql -U ecommerce_user -d ecommerce < backup.sql
```

---

## Health Check

The container includes a built-in health check:

```bash
# Check container health status
docker-compose ps

# Manual health check
docker-compose exec postgres pg_isready -U ecommerce_user -d ecommerce
```

---

## Network Configuration

The database is attached to the `ecommerce_network` Docker network, allowing microservices to connect using the service name:

```yaml
# In microservice docker-compose.yml
services:
  products-service:
    environment:
      DATABASE_URL: postgres://ecommerce_user:ecommerce_password@postgres:5432/ecommerce
    networks:
      - ecommerce_network

networks:
  ecommerce_network:
    external: true
```

---

## Troubleshooting

### Database Won't Start

```bash
# Check logs
docker-compose logs postgres

# Verify init scripts
ls -la init/

# Check permissions
docker-compose exec postgres ls -la /docker-entrypoint-initdb.d/
```

### Connection Refused

```bash
# Check if container is running
docker-compose ps

# Check if port is exposed
docker-compose port postgres 5432

# Test connection from host
telnet localhost 5432
```

### Schemas Not Created

```bash
# Connect and check schemas
docker-compose exec postgres psql -U ecommerce_user -d ecommerce -c "\dn"

# Re-initialize database
docker-compose down -v
docker-compose up -d
```

### Reset Database Completely

```bash
# Stop and remove everything
docker-compose down -v

# Remove Docker volume
docker volume rm ecommerce_postgres_data

# Rebuild and start
docker-compose up -d --build
```

---

## Production Considerations

### Security

1. **Change default credentials** in production:
   ```bash
   POSTGRES_PASSWORD=$(openssl rand -base64 32)
   ```

2. **Use secrets management** (Docker Swarm, Kubernetes):
   ```yaml
   secrets:
     - postgres_password
   ```

3. **Restrict network access**:
   - Don't expose port 5432 publicly
   - Use internal Docker network only

### Performance

1. **Tune PostgreSQL settings** (add to Dockerfile):
   ```dockerfile
   ENV POSTGRES_SHARED_BUFFERS=256MB \
       POSTGRES_MAX_CONNECTIONS=200
   ```

2. **Use connection pooling** in microservices (PgBouncer)

3. **Monitor query performance**:
   ```sql
   -- Enable query logging
   ALTER SYSTEM SET log_statement = 'all';
   ```

### Backup & Recovery

1. **Automated backups**:
   ```bash
   # Cron job for daily backups
   0 2 * * * docker-compose exec postgres pg_dump -U ecommerce_user ecommerce > /backups/ecommerce_$(date +\%Y\%m\%d).sql
   ```

2. **Point-in-time recovery** (enable WAL archiving)

### High Availability

For production, consider:
- PostgreSQL replication (streaming replication)
- Connection pooling (PgBouncer)
- Load balancing (HAProxy)
- Managed PostgreSQL (AWS RDS, Google Cloud SQL, Azure Database)

---

## Next Steps

1. ✅ **Database is ready** - All schemas created
2. 🔲 **Import product data** - Load amazon-products.csv
3. 🔲 **Implement microservices** - Build 3 Rust services
4. 🔲 **Connect services** - Configure database connections
5. 🔲 **Test with client** - Connect Angular frontend

---

## Support

For issues or questions:
- Check logs: `docker-compose logs postgres`
- Verify connection: `docker-compose exec postgres psql -U ecommerce_user -d ecommerce`
- Reset database: `docker-compose down -v && docker-compose up -d`
