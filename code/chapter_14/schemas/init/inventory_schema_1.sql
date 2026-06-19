-- ============================================================
-- INVENTORY SCHEMA - INVENTORY SERVICE
-- ============================================================
-- Handles: Stock management, pricing, discounts
-- Schema: inventory
-- ============================================================

-- Set search path to inventory schema (with access to products schema)
SET search_path TO inventory, products, public;

-- ============================================================
-- PRODUCT_INVENTORY TABLE
-- ============================================================
CREATE TABLE product_inventory (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Product reference (from Products Service via UUID)
    product_uuid UUID NOT NULL, -- References products.uuid in Products Service
    product_asin VARCHAR(20), -- Secondary reference for easier lookup

    -- Stock levels
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    reserved_quantity INTEGER DEFAULT 0, -- Reserved for pending orders
    available_quantity INTEGER GENERATED ALWAYS AS (stock_quantity - reserved_quantity) STORED,

    -- Reorder thresholds
    reorder_level INTEGER DEFAULT 10,
    reorder_quantity INTEGER DEFAULT 50,

    -- Stock status
    stock_status VARCHAR(50) DEFAULT 'in_stock', -- 'in_stock', 'low_stock', 'out_of_stock'

    -- Timestamps
    last_restocked_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(product_uuid)
);

CREATE INDEX idx_product_inventory_uuid ON product_inventory(uuid);
CREATE INDEX idx_product_inventory_product_uuid ON product_inventory(product_uuid);
CREATE INDEX idx_product_inventory_product_asin ON product_inventory(product_asin);
CREATE INDEX idx_product_inventory_stock_status ON product_inventory(stock_status);
CREATE INDEX idx_product_inventory_available_quantity ON product_inventory(available_quantity);

COMMENT ON TABLE product_inventory IS 'Product stock levels - Inventory Service';
COMMENT ON COLUMN product_inventory.product_uuid IS 'References products.uuid from Products Service';
COMMENT ON COLUMN product_inventory.reserved_quantity IS 'Quantity reserved for pending orders';
COMMENT ON COLUMN product_inventory.available_quantity IS 'Computed: stock_quantity - reserved_quantity';

-- ============================================================
-- PRODUCT_PRICING TABLE
-- ============================================================
CREATE TABLE product_pricing (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Product reference (from Products Service via UUID)
    product_uuid UUID NOT NULL, -- References products.uuid in Products Service
    product_asin VARCHAR(20), -- Secondary reference

    -- Pricing
    final_price DECIMAL(10, 2) NOT NULL,
    initial_price DECIMAL(10, 2), -- Original price before discount
    currency VARCHAR(10) DEFAULT 'USD',

    -- Discount
    discount_percentage DECIMAL(5, 2), -- e.g., 20.50 for 20.5%
    discount_amount DECIMAL(10, 2), -- Calculated discount in currency

    -- Validity
    price_valid_from TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    price_valid_until TIMESTAMP WITH TIME ZONE,

    -- Status
    is_active BOOLEAN DEFAULT true,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Only one active price per product
CREATE UNIQUE INDEX idx_product_pricing_unique_active ON product_pricing(product_uuid, is_active) WHERE is_active = true;

CREATE INDEX idx_product_pricing_uuid ON product_pricing(uuid);
CREATE INDEX idx_product_pricing_product_uuid ON product_pricing(product_uuid);
CREATE INDEX idx_product_pricing_product_asin ON product_pricing(product_asin);
CREATE INDEX idx_product_pricing_is_active ON product_pricing(is_active);
CREATE INDEX idx_product_pricing_final_price ON product_pricing(final_price);

COMMENT ON TABLE product_pricing IS 'Product pricing and discounts - Inventory Service';
COMMENT ON COLUMN product_pricing.product_uuid IS 'References products.uuid from Products Service';
COMMENT ON COLUMN product_pricing.discount_percentage IS 'Discount as percentage (e.g., 20.50 = 20.5%)';

-- ============================================================
-- PRICE_HISTORY TABLE (Optional - for tracking price changes)
-- ============================================================
CREATE TABLE price_history (
    id SERIAL PRIMARY KEY,
    product_uuid UUID NOT NULL,
    product_asin VARCHAR(20),

    -- Price snapshot
    final_price DECIMAL(10, 2) NOT NULL,
    initial_price DECIMAL(10, 2),
    discount_percentage DECIMAL(5, 2),
    currency VARCHAR(10) DEFAULT 'USD',

    -- When this price was active
    effective_from TIMESTAMP WITH TIME ZONE NOT NULL,
    effective_until TIMESTAMP WITH TIME ZONE,

    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_price_history_product_uuid ON price_history(product_uuid);
CREATE INDEX idx_price_history_product_asin ON price_history(product_asin);
CREATE INDEX idx_price_history_effective_from ON price_history(effective_from);

COMMENT ON TABLE price_history IS 'Historical price data for analytics';

-- ============================================================
-- INVENTORY_TRANSACTIONS TABLE (Optional - audit trail)
-- ============================================================
CREATE TABLE inventory_transactions (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Inventory reference
    inventory_id INTEGER NOT NULL REFERENCES product_inventory(id) ON DELETE CASCADE,
    product_uuid UUID NOT NULL,

    -- Transaction details
    transaction_type VARCHAR(50) NOT NULL, -- 'purchase', 'sale', 'return', 'adjustment', 'restock'
    quantity_change INTEGER NOT NULL, -- Positive for increase, negative for decrease
    quantity_before INTEGER NOT NULL,
    quantity_after INTEGER NOT NULL,

    -- Reference to source (e.g., order_uuid from Orders Service)
    reference_type VARCHAR(50), -- 'order', 'manual', 'return'
    reference_uuid UUID,

    -- Notes
    notes TEXT,

    -- Timestamp
    transaction_date TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_inventory_transactions_uuid ON inventory_transactions(uuid);
CREATE INDEX idx_inventory_transactions_inventory_id ON inventory_transactions(inventory_id);
CREATE INDEX idx_inventory_transactions_product_uuid ON inventory_transactions(product_uuid);
CREATE INDEX idx_inventory_transactions_type ON inventory_transactions(transaction_type);
CREATE INDEX idx_inventory_transactions_reference_uuid ON inventory_transactions(reference_uuid);
CREATE INDEX idx_inventory_transactions_date ON inventory_transactions(transaction_date);

COMMENT ON TABLE inventory_transactions IS 'Audit trail of all inventory changes';

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

CREATE TRIGGER update_product_inventory_updated_at BEFORE UPDATE ON product_inventory
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_product_pricing_updated_at BEFORE UPDATE ON product_pricing
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- FUNCTION: Update stock status based on quantity
-- ============================================================

CREATE OR REPLACE FUNCTION update_stock_status()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.available_quantity <= 0 THEN
        NEW.stock_status := 'out_of_stock';
    ELSIF NEW.available_quantity <= NEW.reorder_level THEN
        NEW.stock_status := 'low_stock';
    ELSE
        NEW.stock_status := 'in_stock';
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_inventory_stock_status BEFORE INSERT OR UPDATE ON product_inventory
    FOR EACH ROW EXECUTE FUNCTION update_stock_status();

-- ============================================================
-- FUNCTION: Calculate discount amount
-- ============================================================

CREATE OR REPLACE FUNCTION calculate_discount()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.initial_price IS NOT NULL AND NEW.initial_price > NEW.final_price THEN
        NEW.discount_amount := NEW.initial_price - NEW.final_price;
        NEW.discount_percentage := ROUND(((NEW.initial_price - NEW.final_price) / NEW.initial_price * 100), 2);
    ELSE
        NEW.discount_amount := 0;
        NEW.discount_percentage := 0;
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER calculate_pricing_discount BEFORE INSERT OR UPDATE ON product_pricing
    FOR EACH ROW EXECUTE FUNCTION calculate_discount();

-- ============================================================
-- FUNCTION: Reserve stock for order
-- ============================================================

CREATE OR REPLACE FUNCTION reserve_stock(
    p_product_uuid UUID,
    p_quantity INTEGER
) RETURNS BOOLEAN AS $$
DECLARE
    v_available INTEGER;
BEGIN
    -- Get current available quantity
    SELECT available_quantity INTO v_available
    FROM product_inventory
    WHERE product_uuid = p_product_uuid
    FOR UPDATE; -- Lock the row

    -- Check if enough stock
    IF v_available IS NULL OR v_available < p_quantity THEN
        RETURN FALSE;
    END IF;

    -- Reserve the stock
    UPDATE product_inventory
    SET reserved_quantity = reserved_quantity + p_quantity
    WHERE product_uuid = p_product_uuid;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION reserve_stock IS 'Reserve stock for an order (returns true if successful)';

-- ============================================================
-- FUNCTION: Release reserved stock
-- ============================================================

CREATE OR REPLACE FUNCTION release_stock(
    p_product_uuid UUID,
    p_quantity INTEGER
) RETURNS VOID AS $$
BEGIN
    UPDATE product_inventory
    SET reserved_quantity = GREATEST(0, reserved_quantity - p_quantity)
    WHERE product_uuid = p_product_uuid;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION release_stock IS 'Release reserved stock (e.g., when order is cancelled)';

-- ============================================================
-- FUNCTION: Confirm stock reservation (convert to sale)
-- ============================================================

CREATE OR REPLACE FUNCTION confirm_stock_sale(
    p_product_uuid UUID,
    p_quantity INTEGER,
    p_order_uuid UUID
) RETURNS VOID AS $$
BEGIN
    -- Decrease both stock and reserved quantities
    UPDATE product_inventory
    SET
        stock_quantity = stock_quantity - p_quantity,
        reserved_quantity = GREATEST(0, reserved_quantity - p_quantity)
    WHERE product_uuid = p_product_uuid;

    -- Log transaction
    INSERT INTO inventory_transactions (
        inventory_id,
        product_uuid,
        transaction_type,
        quantity_change,
        quantity_before,
        quantity_after,
        reference_type,
        reference_uuid
    )
    SELECT
        id,
        product_uuid,
        'sale',
        -p_quantity,
        stock_quantity + p_quantity,
        stock_quantity,
        'order',
        p_order_uuid
    FROM product_inventory
    WHERE product_uuid = p_product_uuid;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION confirm_stock_sale IS 'Confirm sale and decrease stock (called when order is paid)';

-- ============================================================
-- VIEWS
-- ============================================================

-- Combined inventory and pricing view (for API responses)
CREATE VIEW v_product_inventory_pricing AS
SELECT
    pi.product_uuid,
    pi.product_asin,
    pi.stock_quantity,
    pi.reserved_quantity,
    pi.available_quantity,
    pi.stock_status,
    pp.final_price,
    pp.initial_price,
    pp.currency,
    pp.discount_percentage,
    CASE
        WHEN pp.discount_percentage > 0 THEN CONCAT(pp.discount_percentage::TEXT, '%')
        ELSE NULL
    END as discount,
    pi.last_restocked_at,
    pp.updated_at as price_updated_at
FROM product_inventory pi
LEFT JOIN product_pricing pp ON pi.product_uuid = pp.product_uuid AND pp.is_active = true;

-- Low stock products
CREATE VIEW v_low_stock_products AS
SELECT
    product_uuid,
    product_asin,
    stock_quantity,
    reserved_quantity,
    available_quantity,
    reorder_level,
    stock_status
FROM product_inventory
WHERE stock_status IN ('low_stock', 'out_of_stock')
ORDER BY available_quantity ASC;

COMMENT ON VIEW v_product_inventory_pricing IS 'Combined inventory and pricing data for API responses';
COMMENT ON VIEW v_low_stock_products IS 'Products that need restocking';

-- ============================================================
-- SAMPLE DATA (for testing)
-- ============================================================

-- Note: Product UUIDs will come from Products Service
-- This is just a template for how data will be structured

-- Example: If a product in Products Service has uuid = '550e8400-e29b-41d4-a716-446655440000'
-- Then in Inventory Service:

/*
INSERT INTO product_inventory (product_uuid, product_asin, stock_quantity, reorder_level)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'B00ABC123', 100, 10);

INSERT INTO product_pricing (product_uuid, product_asin, final_price, initial_price, currency)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'B00ABC123', 29.99, 39.99, 'USD');
*/
