-- ============================================================
-- ORDERS SCHEMA - ORDERS & CHECKOUT SERVICE
-- ============================================================
-- Handles: Orders, payments, shipping, guest checkout
-- Schema: orders
-- Port: 3003

-- Set search path to orders schema (with access to products schema)
SET search_path TO orders, products, public;

-- ============================================================
-- ORDERS TABLE
-- ============================================================
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Order identification
    order_number VARCHAR(50) UNIQUE NOT NULL, -- e.g., "ORD-20241118-001"

    -- Customer info (guest checkout - no user_id needed)
    customer_email VARCHAR(255) NOT NULL,
    customer_phone VARCHAR(50),
    is_guest_order BOOLEAN DEFAULT true,

    -- Pricing breakdown
    subtotal DECIMAL(10, 2) NOT NULL, -- Sum of item prices
    tax_amount DECIMAL(10, 2) NOT NULL DEFAULT 0,
    shipping_amount DECIMAL(10, 2) NOT NULL DEFAULT 0,
    total DECIMAL(10, 2) NOT NULL, -- subtotal + tax + shipping

    -- Order status
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'processing', 'shipped', 'delivered', 'cancelled', 'failed'
    payment_status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'paid', 'failed', 'refunded'

    -- Timestamps
    ordered_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_orders_uuid ON orders(uuid);
CREATE INDEX idx_orders_order_number ON orders(order_number);
CREATE INDEX idx_orders_customer_email ON orders(customer_email);
CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_payment_status ON orders(payment_status);
CREATE INDEX idx_orders_ordered_at ON orders(ordered_at);

COMMENT ON TABLE orders IS 'Customer orders - Orders Service';
COMMENT ON COLUMN orders.uuid IS 'External UUID (exposed as eid in API)';
COMMENT ON COLUMN orders.order_number IS 'Human-readable order number';
COMMENT ON COLUMN orders.is_guest_order IS 'True for guest checkout (no user account)';

-- ============================================================
-- ORDER_ITEMS TABLE
-- ============================================================
CREATE TABLE order_items (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Order reference
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,

    -- Product reference (from Products Service via UUID)
    product_uuid UUID NOT NULL, -- References products.uuid from Products Service
    product_asin VARCHAR(20),

    -- Product snapshot (data at time of order)
    product_name VARCHAR(500) NOT NULL,
    product_sku VARCHAR(100),

    -- Pricing
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price DECIMAL(10, 2) NOT NULL,
    total_price DECIMAL(10, 2) NOT NULL, -- unit_price * quantity

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_order_items_uuid ON order_items(uuid);
CREATE INDEX idx_order_items_order_id ON order_items(order_id);
CREATE INDEX idx_order_items_product_uuid ON order_items(product_uuid);
CREATE INDEX idx_order_items_product_asin ON order_items(product_asin);

COMMENT ON TABLE order_items IS 'Line items in orders with product snapshots';
COMMENT ON COLUMN order_items.product_uuid IS 'References products.uuid from Products Service';
COMMENT ON COLUMN order_items.product_name IS 'Product name snapshot at time of order';

-- ============================================================
-- SHIPPING_ADDRESSES TABLE
-- ============================================================
CREATE TABLE shipping_addresses (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Order reference
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,

    -- Recipient
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,

    -- Address
    address_line1 VARCHAR(255) NOT NULL,
    address_line2 VARCHAR(255),
    city VARCHAR(100) NOT NULL,
    state VARCHAR(100) NOT NULL,
    postal_code VARCHAR(20) NOT NULL,
    country VARCHAR(100) NOT NULL DEFAULT 'US',

    -- Contact
    phone VARCHAR(50),

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(order_id) -- One shipping address per order
);

CREATE INDEX idx_shipping_addresses_uuid ON shipping_addresses(uuid);
CREATE INDEX idx_shipping_addresses_order_id ON shipping_addresses(order_id);

COMMENT ON TABLE shipping_addresses IS 'Shipping addresses for orders';
COMMENT ON COLUMN shipping_addresses.uuid IS 'External UUID (exposed as eid in API)';

-- ============================================================
-- PAYMENTS TABLE
-- ============================================================
CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Order reference
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,

    -- Payment details
    payment_method VARCHAR(50) NOT NULL, -- 'credit_card', 'debit_card'
    amount DECIMAL(10, 2) NOT NULL,

    -- Payment status
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'paid', 'failed', 'refunded', 'cancelled'

    -- Simulated payment info (not real payment processing)
    payment_reference VARCHAR(255), -- Simulated transaction ID
    card_last4 VARCHAR(4), -- Last 4 digits (for display)
    card_brand VARCHAR(50), -- 'visa', 'mastercard', 'amex', etc.

    -- Timestamps
    processed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(order_id) -- One payment per order (simplified)
);

CREATE INDEX idx_payments_uuid ON payments(uuid);
CREATE INDEX idx_payments_order_id ON payments(order_id);
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_payment_method ON payments(payment_method);

COMMENT ON TABLE payments IS 'Simulated payment records';
COMMENT ON COLUMN payments.uuid IS 'External UUID (exposed as eid in API)';
COMMENT ON COLUMN payments.payment_reference IS 'Simulated transaction ID (not real)';

-- ============================================================
-- SHIPMENTS TABLE (Optional - for tracking)
-- ============================================================
CREATE TABLE shipments (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Order reference
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,

    -- Shipping carrier (from client: 'ups', 'fedex', 'dhl')
    carrier VARCHAR(50) NOT NULL,
    tracking_number VARCHAR(255),

    -- Shipment status
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'shipped', 'in_transit', 'delivered'

    -- Delivery dates
    estimated_delivery_date DATE,
    actual_delivery_date DATE,

    -- Timestamps
    shipped_at TIMESTAMP WITH TIME ZONE,
    delivered_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(order_id) -- One shipment per order (simplified)
);

CREATE INDEX idx_shipments_uuid ON shipments(uuid);
CREATE INDEX idx_shipments_order_id ON shipments(order_id);
CREATE INDEX idx_shipments_tracking_number ON shipments(tracking_number);
CREATE INDEX idx_shipments_status ON shipments(status);
CREATE INDEX idx_shipments_carrier ON shipments(carrier);

COMMENT ON TABLE shipments IS 'Order shipment tracking';
COMMENT ON COLUMN shipments.uuid IS 'External UUID (exposed as eid in API)';

-- ============================================================
-- TRIGGERS FOR UPDATED_AT
-- ============================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_orders_updated_at BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_shipping_addresses_updated_at BEFORE UPDATE ON shipping_addresses
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_payments_updated_at BEFORE UPDATE ON payments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_shipments_updated_at BEFORE UPDATE ON shipments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- FUNCTION: Generate order number
-- ============================================================

CREATE OR REPLACE FUNCTION generate_order_number()
RETURNS TEXT AS $$
DECLARE
    v_date TEXT;
    v_sequence TEXT;
    v_order_number TEXT;
BEGIN
    -- Format: ORD-YYYYMMDD-XXX
    v_date := TO_CHAR(CURRENT_TIMESTAMP, 'YYYYMMDD');

    -- Get next sequence number for today
    SELECT LPAD((COUNT(*) + 1)::TEXT, 3, '0')
    INTO v_sequence
    FROM orders
    WHERE order_number LIKE 'ORD-' || v_date || '-%';

    v_order_number := 'ORD-' || v_date || '-' || v_sequence;

    RETURN v_order_number;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION generate_order_number IS 'Generate unique order number: ORD-YYYYMMDD-001';

-- ============================================================
-- FUNCTION: Generate simulated payment reference
-- ============================================================

CREATE OR REPLACE FUNCTION generate_payment_reference()
RETURNS TEXT AS $$
BEGIN
    -- Format: PAY-TIMESTAMP-RANDOM
    RETURN 'PAY-' ||
           EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT::TEXT || '-' ||
           SUBSTRING(MD5(RANDOM()::TEXT), 1, 8);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION generate_payment_reference IS 'Generate simulated payment transaction ID';

-- ============================================================
-- FUNCTION: Calculate order total
-- ============================================================

CREATE OR REPLACE FUNCTION calculate_order_total(p_order_id INTEGER)
RETURNS DECIMAL AS $$
DECLARE
    v_subtotal DECIMAL;
    v_tax DECIMAL;
    v_shipping DECIMAL;
    v_total DECIMAL;
BEGIN
    SELECT subtotal, tax_amount, shipping_amount
    INTO v_subtotal, v_tax, v_shipping
    FROM orders
    WHERE id = p_order_id;

    v_total := v_subtotal + v_tax + v_shipping;

    UPDATE orders
    SET total = v_total
    WHERE id = p_order_id;

    RETURN v_total;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION calculate_order_total IS 'Calculate and update order total';

-- ============================================================
-- VIEWS
-- ============================================================

-- Complete order view with all related data
CREATE VIEW v_orders_complete AS
SELECT
    o.id,
    o.uuid as eid,
    o.order_number,
    o.customer_email,
    o.customer_phone,
    o.subtotal,
    o.tax_amount,
    o.shipping_amount,
    o.total,
    o.status,
    o.payment_status,
    o.is_guest_order,
    o.ordered_at,
    o.created_at,
    o.updated_at,

    -- Shipping address (as JSON)
    jsonb_build_object(
        'eid', sa.uuid,
        'first_name', sa.first_name,
        'last_name', sa.last_name,
        'address_line1', sa.address_line1,
        'address_line2', sa.address_line2,
        'city', sa.city,
        'state', sa.state,
        'postal_code', sa.postal_code,
        'country', sa.country,
        'phone', sa.phone
    ) as shipping_address,

    -- Payment info (as JSON)
    jsonb_build_object(
        'eid', p.uuid,
        'payment_method', p.payment_method,
        'amount', p.amount,
        'status', p.status,
        'payment_reference', p.payment_reference,
        'card_last4', p.card_last4,
        'card_brand', p.card_brand,
        'processed_at', p.processed_at
    ) as payment,

    -- Shipment info (as JSON, if exists)
    CASE
        WHEN s.id IS NOT NULL THEN
            jsonb_build_object(
                'eid', s.uuid,
                'carrier', s.carrier,
                'tracking_number', s.tracking_number,
                'status', s.status,
                'estimated_delivery_date', s.estimated_delivery_date,
                'actual_delivery_date', s.actual_delivery_date,
                'shipped_at', s.shipped_at,
                'delivered_at', s.delivered_at
            )
        ELSE NULL
    END as shipment,

    -- Item count
    (SELECT COUNT(*) FROM order_items WHERE order_id = o.id) as item_count,
    (SELECT SUM(quantity) FROM order_items WHERE order_id = o.id) as total_quantity

FROM orders o
LEFT JOIN shipping_addresses sa ON o.id = sa.order_id
LEFT JOIN payments p ON o.id = p.order_id
LEFT JOIN shipments s ON o.id = s.order_id;

-- Order items with product info
CREATE VIEW v_order_items_detail AS
SELECT
    oi.id,
    oi.uuid as eid,
    oi.order_id,
    o.order_number,
    oi.product_uuid,
    oi.product_asin,
    oi.product_name,
    oi.product_sku,
    oi.quantity,
    oi.unit_price,
    oi.total_price,
    oi.created_at
FROM order_items oi
INNER JOIN orders o ON oi.order_id = o.id;

-- Recent orders summary
CREATE VIEW v_recent_orders AS
SELECT
    uuid as eid,
    order_number,
    customer_email,
    total,
    status,
    payment_status,
    ordered_at
FROM orders
ORDER BY ordered_at DESC
LIMIT 100;

COMMENT ON VIEW v_orders_complete IS 'Complete order data with shipping, payment, and shipment info';
COMMENT ON VIEW v_order_items_detail IS 'Order items with order information';
COMMENT ON VIEW v_recent_orders IS 'Most recent 100 orders';

-- ============================================================
-- CONSTRAINTS & CHECKS
-- ============================================================

-- Ensure total matches subtotal + tax + shipping
ALTER TABLE orders ADD CONSTRAINT check_order_total
    CHECK (ABS(total - (subtotal + tax_amount + shipping_amount)) < 0.01);

-- Payment amount validation is handled at application level

-- Ensure quantity is positive
ALTER TABLE order_items ADD CONSTRAINT check_quantity_positive
    CHECK (quantity > 0);

-- Ensure prices are non-negative
ALTER TABLE order_items ADD CONSTRAINT check_prices_non_negative
    CHECK (unit_price >= 0 AND total_price >= 0);
