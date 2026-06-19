# OpenTel E-Commerce - Architecture Documentation

## 1. High-Level System Architecture

```mermaid
graph TB
    subgraph Internet["🌐 Internet"]
        User([👤 User<br/>Web Browser])
    end
    
    subgraph Frontend["Frontend Layer"]
        SPA[Angular 20 SPA<br/>📱 Single Page Application<br/>━━━━━━━━━━━━━━<br/>• Product Browsing<br/>• Shopping Cart<br/>• Checkout Flow]
    end
    
    subgraph Gateway["API Gateway Layer"]
        BFF[OtelMart Service<br/>🚪 Backend for Frontend<br/>━━━━━━━━━━━━━━<br/>Port: 4200<br/>━━━━━━━━━━━━━━<br/>• Static File Server<br/>• User Authentication<br/>• API Proxy/Router]
    end
    
    subgraph Services["Microservices Layer"]
        PS[Products Service<br/>📦 Product Catalog<br/>━━━━━━━━━━━━━━<br/>Port: 3001<br/>━━━━━━━━━━━━━━<br/>• Catalog Management<br/>• Product Search<br/>• Ratings & Reviews]
        
        IS[Inventory Service<br/>📊 Stock Management<br/>━━━━━━━━━━━━━━<br/>Port: 3002<br/>━━━━━━━━━━━━━━<br/>• Stock Levels<br/>• Pricing<br/>• Reservations]
        
        OS[Orders Service<br/>🛒 Order Processing<br/>━━━━━━━━━━━━━━<br/>Port: 3003<br/>━━━━━━━━━━━━━━<br/>• Order Creation<br/>• Payment Processing<br/>• Shipment Tracking]
        
        FS[Fraud Scoring Service<br/>🤖 GenAI Fraud Detection<br/>━━━━━━━━━━━━━━<br/>Port: 3004<br/>━━━━━━━━━━━━━━<br/>• Risk Scoring<br/>• Model Routing<br/>• Token Tracking]
    end
    
    subgraph Data["Data Layer"]
        DB[(PostgreSQL 17<br/>🗄️ Relational Database<br/>━━━━━━━━━━━━━━<br/>Port: 5433<br/>━━━━━━━━━━━━━━<br/>Schemas:<br/>• users<br/>• products<br/>• inventory<br/>• orders)]
    end
    
    User -->|HTTPS| SPA
    SPA -.->|Bundled & Served| BFF
    User -->|REST API| BFF
    
    BFF -->|/api/products/*| PS
    BFF -->|/api/inventory/*| IS
    BFF -->|/api/orders/*| OS
    BFF -->|User Auth| DB
    
    PS <-->|SQL Queries| DB
    IS <-->|SQL Queries| DB
    OS <-->|SQL Queries| DB
    OS -->|POST /score| FS
    FS -.->|Anthropic API| GenAI([☁️ GenAI Provider<br/>api.anthropic.com])
    
    style User fill:#E3F2FD,stroke:#1976D2,stroke-width:3px
    style SPA fill:#61DAFB,stroke:#20232A,stroke-width:2px,color:#000
    style BFF fill:#CE422B,stroke:#8B0000,stroke-width:3px,color:#fff
    style PS fill:#4CAF50,stroke:#2E7D32,stroke-width:2px,color:#fff
    style IS fill:#FF9800,stroke:#E65100,stroke-width:2px,color:#fff
    style OS fill:#2196F3,stroke:#0D47A1,stroke-width:2px,color:#fff
    style FS fill:#9C27B0,stroke:#6A1B9A,stroke-width:2px,color:#fff
    style DB fill:#336791,stroke:#1A3A52,stroke-width:3px,color:#fff
```

---

## 2. Detailed Service Communication Flow

```mermaid
sequenceDiagram
    participant U as 👤 User
    participant A as Angular SPA
    participant G as OtelMart Gateway
    participant P as Products Service
    participant I as Inventory Service
    participant O as Orders Service
    participant F as Fraud Scoring
    participant D as PostgreSQL
    participant AI as Anthropic API
    
    Note over U,D: User Browse Products Flow
    
    U->>A: Browse Products
    A->>G: GET /api/products?page=1
    G->>P: Proxy → GET /products?page=1
    P->>D: SELECT FROM products.products
    D-->>P: Product Data
    P-->>G: JSON Response
    G-->>A: Product List
    A-->>U: Display Products
    
    Note over U,D: User Place Order Flow
    
    U->>A: Add to Cart & Checkout
    A->>G: POST /api/orders (order data)
    G->>O: Proxy → POST /orders
    
    O->>I: Check Stock: GET /inventory/{product_id}
    I->>D: SELECT FROM inventory.stock
    D-->>I: Stock Data
    I-->>O: Stock Available
    
    O->>I: Reserve Stock: POST /inventory/{product_id}/reserve
    I->>D: UPDATE inventory.stock
    D-->>I: Updated
    I-->>O: Reserved
    
    O->>F: Score Fraud: POST /score
    F->>AI: POST /v1/messages (GenAI call)
    AI-->>F: Risk Score + Token Usage
    F-->>O: {risk_score, decision}
    
    Note over O,F: If decision="rejected", release stock and return 403
    
    O->>D: INSERT INTO orders.orders
    D-->>O: Order Created
    
    O->>D: INSERT INTO orders.payments
    D-->>O: Payment Recorded
    
    O-->>G: Order Success
    G-->>A: Order Confirmation
    A-->>U: Show Confirmation Page
```

---

## 3. Service Dependencies & Technology Stack

```mermaid
graph LR
    subgraph "Tech Stack"
        direction TB
        
        subgraph "Frontend"
            A[Angular 20<br/>TypeScript<br/>Bootstrap 5]
        end
        
        subgraph "Backend Services"
            B[Rust + Axum<br/>Tokio Runtime<br/>Async/Await]
        end
        
        subgraph "Database"
            C[PostgreSQL 17<br/>SQLx Driver<br/>Migrations]
        end
        
        subgraph "DevOps"
            D[Docker + Docker Compose<br/>Multi-stage Builds<br/>Health Checks]
        end
    end
    
    subgraph "Service Dependencies"
        direction TB
        O[OtelMart] --> P[Products]
        O --> I[Inventory]
        O --> Or[Orders]
        Or --> F[Fraud Scoring]
        
        P --> DB[(Database)]
        I --> DB
        Or --> DB
        O --> DB
        F -.-> AI([Anthropic API])
    end
    
    style A fill:#61DAFB,color:#000
    style B fill:#CE422B,color:#fff
    style C fill:#336791,color:#fff
    style D fill:#2496ED,color:#fff
```

---

## 4. Database Schema Organization

```mermaid
graph TB
    subgraph "PostgreSQL - opentel_db"
        
        subgraph "users Schema"
            U1[users<br/>━━━━━━━<br/>id, eid, email<br/>password_hash]
            U2[user_addresses<br/>━━━━━━━<br/>id, user_id<br/>address details]
            U3[sessions<br/>━━━━━━━<br/>session_id<br/>user_id, expires_at]
            
            U1 --- U2
            U1 --- U3
        end
        
        subgraph "products Schema"
            P1[products<br/>━━━━━━━<br/>id, eid, sku<br/>name, price, stock]
            P2[product_specifications<br/>━━━━━━━<br/>id, product_id<br/>specifications]
            P3[customer_reviews<br/>━━━━━━━<br/>id, product_id<br/>rating, review]
            
            P1 --- P2
            P1 --- P3
        end
        
        subgraph "inventory Schema"
            I1[inventory_stock<br/>━━━━━━━<br/>id, product_id<br/>quantity, reserved]
            I2[pricing<br/>━━━━━━━<br/>id, product_id<br/>price, discount]
            I3[inventory_transactions<br/>━━━━━━━<br/>id, product_id<br/>quantity_change]
            
            I1 --- I2
            I1 --- I3
        end
        
        subgraph "orders Schema"
            O1[orders<br/>━━━━━━━<br/>id, eid, order_number<br/>user_id, total]
            O2[order_items<br/>━━━━━━━<br/>id, order_id<br/>product_id, quantity]
            O3[payments<br/>━━━━━━━<br/>id, order_id<br/>amount, status]
            O4[shipments<br/>━━━━━━━<br/>id, order_id<br/>tracking, status]
            
            O1 --- O2
            O1 --- O3
            O1 --- O4
        end
    end
    
    style U1 fill:#9C27B0,color:#fff
    style U2 fill:#9C27B0,color:#fff
    style U3 fill:#9C27B0,color:#fff
    style P1 fill:#4CAF50,color:#fff
    style P2 fill:#4CAF50,color:#fff
    style P3 fill:#4CAF50,color:#fff
    style I1 fill:#FF9800,color:#fff
    style I2 fill:#FF9800,color:#fff
    style I3 fill:#FF9800,color:#fff
    style O1 fill:#2196F3,color:#fff
    style O2 fill:#2196F3,color:#fff
    style O3 fill:#2196F3,color:#fff
    style O4 fill:#2196F3,color:#fff
```

---

## 5. Docker Container Architecture

```mermaid
graph TB
    subgraph DC["Docker Compose Network: app-network"]
        
        subgraph "Container: postgres"
            PG[PostgreSQL 17<br/>━━━━━━━━━━<br/>Volume: postgres_data<br/>Port: 5432→5433<br/>Health Check: pg_isready]
        end
        
        subgraph "Container: products-service"
            PS[Products Service<br/>━━━━━━━━━━<br/>Port: 3001<br/>Env: DATABASE_URL<br/>Runs Migrations]
        end
        
        subgraph "Container: inventory-service"
            IS[Inventory Service<br/>━━━━━━━━━━<br/>Port: 3002<br/>Env: DATABASE_URL<br/>Runs Migrations]
        end
        
        subgraph "Container: orders-service"
            OS[Orders Service<br/>━━━━━━━━━━<br/>Port: 3003<br/>Env: DATABASE_URL<br/>Runs Migrations]
        end
        
        subgraph "Container: data-ingestion"
            DI[Data Ingestion<br/>━━━━━━━━━━<br/>One-time Job<br/>Loads CSV Data<br/>restart: no]
        end
        
        subgraph "Container: otelmart"
            OM[OtelMart<br/>━━━━━━━━━━<br/>Port: 4200<br/>Serves Angular SPA<br/>API Gateway]
        end
    end
    
    PG -.->|Health Check| PS
    PG -.->|Health Check| IS
    PG -.->|Health Check| OS
    PS -.->|Migrations Done| DI
    DI -.->|Data Loaded| OM
    
    PS -->|SQL| PG
    IS -->|SQL| PG
    OS -->|SQL| PG
    OM -->|SQL| PG
    DI -->|SQL| PG
    
    OM -->|HTTP| PS
    OM -->|HTTP| IS
    OM -->|HTTP| OS
    
    style PG fill:#336791,color:#fff,stroke:#1A3A52,stroke-width:3px
    style PS fill:#4CAF50,color:#fff,stroke:#2E7D32,stroke-width:2px
    style IS fill:#FF9800,color:#fff,stroke:#E65100,stroke-width:2px
    style OS fill:#2196F3,color:#fff,stroke:#0D47A1,stroke-width:2px
    style DI fill:#9E9E9E,color:#fff,stroke:#424242,stroke-width:2px
    style OM fill:#CE422B,color:#fff,stroke:#8B0000,stroke-width:3px
```

---

## 6. Request Routing Details

```mermaid
graph LR
    Browser[🌐 Browser<br/>localhost:4200]
    
    subgraph OtelMart["OtelMart Service :4200"]
        Static[Static Files<br/>Angular SPA]
        Auth[Auth Routes<br/>/api/auth/*]
        Users[User Routes<br/>/api/users/*]
        Proxy[API Proxy]
    end
    
    subgraph Backend["Backend Microservices"]
        P[Products<br/>:3001]
        I[Inventory<br/>:3002]
        O[Orders<br/>:3003]
    end
    
    Browser -->|GET /| Static
    Browser -->|POST /api/auth/login| Auth
    Browser -->|GET /api/users/profile| Users
    Browser -->|GET /api/products| Proxy
    
    Proxy -->|/api/products/*| P
    Proxy -->|/api/inventory/*| I
    Proxy -->|/api/orders/*| O
    
    style Browser fill:#E3F2FD,stroke:#1976D2,stroke-width:2px
    style Static fill:#61DAFB,stroke:#20232A,stroke-width:2px,color:#000
    style Auth fill:#9C27B0,stroke:#4A148C,stroke-width:2px,color:#fff
    style Users fill:#9C27B0,stroke:#4A148C,stroke-width:2px,color:#fff
    style Proxy fill:#FF5722,stroke:#BF360C,stroke-width:2px,color:#fff
    style P fill:#4CAF50,stroke:#2E7D32,stroke-width:2px,color:#fff
    style I fill:#FF9800,stroke:#E65100,stroke-width:2px,color:#fff
    style O fill:#2196F3,stroke:#0D47A1,stroke-width:2px,color:#fff
```

---

## Key Architecture Decisions

### ✅ **Microservices Benefits**
- **Independent Scaling**: Each service can scale independently
- **Technology Flexibility**: Each service could use different tech (though all use Rust here)
- **Isolated Failures**: One service failure doesn't bring down entire system
- **Team Autonomy**: Different teams can own different services

### 🏗️ **API Gateway Pattern**
- **Single Entry Point**: OtelMart acts as unified API gateway
- **Cross-Cutting Concerns**: Authentication, CORS, logging handled centrally
- **Backend for Frontend**: Tailored for web client needs
- **Service Discovery**: Routes requests to appropriate microservices

### 📊 **Database Strategy**
- **Logical Separation**: Different schemas per service (users, products, inventory, orders)
- **Shared Database**: Simplified for learning (not ideal for production microservices)
- **Migration Management**: Each service manages its own schema migrations
- **Performance**: Indexes on all foreign keys and common query patterns

### 🐳 **Containerization**
- **Docker Compose**: Orchestrates all services locally
- **Health Checks**: Ensures proper startup order
- **Networking**: All services communicate via Docker network
- **Volumes**: Persistent database storage

### 🔍 **Observability Ready**
This architecture is designed for OpenTelemetry instrumentation:
- Distributed tracing across service boundaries
- Metrics collection from each service
- Structured logging with correlation IDs
- Context propagation via HTTP headers

---

## Port Reference

| Service | Internal Port | External Port | Purpose |
|---------|--------------|---------------|---------|
| PostgreSQL | 5432 | 5433 | Database connections |
| Products | 3001 | 3001 | Product API |
| Inventory | 3002 | 3002 | Inventory API |
| Orders | 3003 | 3003 | Orders API |
| OtelMart | 4200 | 4200 | Web App + Gateway |

---

## Startup Sequence

1. **PostgreSQL** starts with health checks
2. **Products, Inventory, Orders** services start and run migrations
3. **Data Ingestion** loads initial product data (runs once)
4. **OtelMart** starts serving Angular app and proxying APIs

All services wait for PostgreSQL health check before connecting.
