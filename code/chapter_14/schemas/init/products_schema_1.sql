-- ============================================================
-- PRODUCTS SCHEMA - PRODUCTS SERVICE
-- ============================================================
-- Handles: Product catalog, categories, specifications, ratings
-- Schema: products
-- Port: 3001
--
-- Updated: Added pricing, inventory, and hierarchical categories
--          to match UI requirements (ProductCard and Product interfaces)

-- Set search path to products schema
SET search_path TO products, public;

-- ============================================================
-- CATEGORIES TABLE (Hierarchical)
-- ============================================================
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Category info
    name VARCHAR(255) NOT NULL UNIQUE,
    slug VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,

    -- Hierarchical structure
    parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
    level INTEGER DEFAULT 0, -- 0 = root, 1 = child, 2 = grandchild, etc.
    path TEXT, -- Materialized path: '1/23/45' for quick ancestor queries

    -- Display order
    sort_order INTEGER DEFAULT 0,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_categories_uuid ON categories(uuid);
CREATE INDEX idx_categories_slug ON categories(slug);
CREATE INDEX idx_categories_parent_id ON categories(parent_id);
CREATE INDEX idx_categories_level ON categories(level);
CREATE INDEX idx_categories_path ON categories(path);

COMMENT ON TABLE categories IS 'Product categories with hierarchical support';
COMMENT ON COLUMN categories.uuid IS 'External UUID (exposed as eid in API)';
COMMENT ON COLUMN categories.parent_id IS 'Parent category for hierarchy (NULL for root)';
COMMENT ON COLUMN categories.level IS 'Depth in hierarchy: 0=root, 1=subcategory, etc.';
COMMENT ON COLUMN categories.path IS 'Materialized path for ancestor queries';

-- ============================================================
-- PRODUCTS TABLE (with pricing and inventory)
-- ============================================================
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- External IDs (from Amazon data)
    product_id VARCHAR(50) NOT NULL UNIQUE, -- External product identifier (e.g., PROD-001)
    asin VARCHAR(20) UNIQUE,
    sku VARCHAR(100) UNIQUE,
    gtin VARCHAR(50),

    -- Basic information
    product_name VARCHAR(500) NOT NULL,
    brand VARCHAR(255),
    description TEXT,

    -- Category
    category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL,

    -- PRICING (denormalized from inventory service for initial development)
    price DECIMAL(10, 2) NOT NULL DEFAULT 0, -- Current final price
    initial_price DECIMAL(10, 2), -- Original price before discount
    discount VARCHAR(10), -- Discount label: "20%", "$10 off", etc.
    currency VARCHAR(10) DEFAULT 'USD',

    -- INVENTORY (denormalized from inventory service for initial development)
    stock_quantity INTEGER NOT NULL DEFAULT 0, -- Available stock

    -- Product attributes (from Amazon)
    sizes TEXT[], -- Array of sizes: ["S", "M", "L", "XL"]
    colors TEXT[], -- Array of colors: ["Red", "Blue", "Green"]

    -- URLs and media
    url TEXT,
    image_url TEXT,

    -- Flags
    available_for_delivery BOOLEAN DEFAULT true,
    available_for_pickup BOOLEAN DEFAULT false,
    free_returns BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,

    -- Timestamps
    deleted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    data_timestamp TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_products_uuid ON products(uuid);
CREATE INDEX idx_products_product_id ON products(product_id);
CREATE INDEX idx_products_asin ON products(asin);
CREATE INDEX idx_products_sku ON products(sku);
CREATE INDEX idx_products_product_name ON products(product_name);
CREATE INDEX idx_products_category_id ON products(category_id);
CREATE INDEX idx_products_brand ON products(brand);
CREATE INDEX idx_products_price ON products(price);
CREATE INDEX idx_products_stock_quantity ON products(stock_quantity);
CREATE INDEX idx_products_is_active ON products(is_active);
CREATE INDEX idx_products_updated_at ON products(updated_at);

-- Full-text search index
CREATE INDEX idx_products_search ON products USING GIN(
    to_tsvector('english', product_name || ' ' || COALESCE(description, '') || ' ' || COALESCE(brand, ''))
);

COMMENT ON TABLE products IS 'Core product catalog with pricing and inventory';
COMMENT ON COLUMN products.uuid IS 'External UUID (exposed as eid in API)';
COMMENT ON COLUMN products.asin IS 'Amazon Standard Identification Number';
COMMENT ON COLUMN products.price IS 'Current selling price (denormalized from inventory)';
COMMENT ON COLUMN products.initial_price IS 'Original price before discount';
COMMENT ON COLUMN products.discount IS 'Discount label for UI display';
COMMENT ON COLUMN products.stock_quantity IS 'Available inventory (denormalized)';

-- ============================================================
-- RATINGS TABLE (simple user ratings)
-- ============================================================
CREATE TABLE ratings (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,

    -- User reference (from users schema)
    user_id UUID NOT NULL,

    -- Rating (1-5 stars)
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),

    -- Optional review text
    review TEXT,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- One rating per user per product
    UNIQUE(product_id, user_id)
);

CREATE INDEX idx_ratings_uuid ON ratings(uuid);
CREATE INDEX idx_ratings_product_id ON ratings(product_id);
CREATE INDEX idx_ratings_user_id ON ratings(user_id);
CREATE INDEX idx_ratings_rating ON ratings(rating);

COMMENT ON TABLE ratings IS 'User product ratings (1-5 stars)';
COMMENT ON COLUMN ratings.user_id IS 'References users.uuid from users schema';
COMMENT ON COLUMN ratings.rating IS 'Star rating: 1 (poor) to 5 (excellent)';

-- ============================================================
-- PRODUCT_SPECIFICATIONS TABLE (for detailed specs)
-- ============================================================
CREATE TABLE product_specifications (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    spec_name VARCHAR(255) NOT NULL,
    spec_value TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_product_specs_product_id ON product_specifications(product_id);

COMMENT ON TABLE product_specifications IS 'Product technical specifications';

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

CREATE TRIGGER update_categories_updated_at BEFORE UPDATE ON categories
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_products_updated_at BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_ratings_updated_at BEFORE UPDATE ON ratings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- HELPER FUNCTIONS
-- ============================================================

-- Function to get root category name for a product
CREATE OR REPLACE FUNCTION get_root_category_name(cat_id INTEGER)
RETURNS TEXT AS $$
DECLARE
    root_name TEXT;
BEGIN
    WITH RECURSIVE category_tree AS (
        -- Start with the given category
        SELECT id, name, parent_id, 0 as depth
        FROM categories
        WHERE id = cat_id

        UNION ALL

        -- Recursively get parent categories
        SELECT c.id, c.name, c.parent_id, ct.depth + 1
        FROM categories c
        INNER JOIN category_tree ct ON c.id = ct.parent_id
    )
    SELECT name INTO root_name
    FROM category_tree
    ORDER BY depth DESC
    LIMIT 1;

    RETURN root_name;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_root_category_name IS 'Get the root category name for a given category';

-- ============================================================
-- SAMPLE DATA - ROOT CATEGORIES
-- ============================================================

-- Insert root categories first
INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order) VALUES
('Electronics', 'electronics', 'Electronic devices and accessories', NULL, 0, '1', 1),
('Clothing', 'clothing', 'Apparel and fashion items', NULL, 0, '2', 2),
('Home & Garden', 'home-garden', 'Home improvement and garden supplies', NULL, 0, '3', 3),
('Sports & Outdoors', 'sports-outdoors', 'Sports equipment and outdoor gear', NULL, 0, '4', 4),
('Books & Media', 'books-media', 'Books, movies, and media', NULL, 0, '5', 5),
('Health & Beauty', 'health-beauty', 'Health and beauty products', NULL, 0, '6', 6),
('Toys & Games', 'toys-games', 'Toys, games, and hobbies', NULL, 0, '7', 7),
('Automotive', 'automotive', 'Auto parts and accessories', NULL, 0, '8', 8)
ON CONFLICT (name) DO NOTHING;

-- Insert subcategories
INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Smartphones', 'smartphones', 'Mobile phones and smartphones', id, 1, '1/' || id::text, 1
FROM categories WHERE slug = 'electronics'
ON CONFLICT (name) DO NOTHING;

INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Laptops', 'laptops', 'Laptop computers', id, 1, '1/' || id::text, 2
FROM categories WHERE slug = 'electronics'
ON CONFLICT (name) DO NOTHING;

INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Accessories', 'accessories', 'Phone and computer accessories', id, 1, '1/' || id::text, 3
FROM categories WHERE slug = 'electronics'
ON CONFLICT (name) DO NOTHING;

INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Smart Home', 'smart-home', 'Smart home devices', id, 1, '1/' || id::text, 4
FROM categories WHERE slug = 'electronics'
ON CONFLICT (name) DO NOTHING;

INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Men''s Clothing', 'mens-clothing', 'Clothing for men', id, 1, '2/' || id::text, 1
FROM categories WHERE slug = 'clothing'
ON CONFLICT (name) DO NOTHING;

INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Women''s Clothing', 'womens-clothing', 'Clothing for women', id, 1, '2/' || id::text, 2
FROM categories WHERE slug = 'clothing'
ON CONFLICT (name) DO NOTHING;

INSERT INTO categories (name, slug, description, parent_id, level, path, sort_order)
SELECT 'Shoes', 'shoes', 'Footwear', id, 1, '2/' || id::text, 3
FROM categories WHERE slug = 'clothing'
ON CONFLICT (name) DO NOTHING;

-- Note: Actual product data will be generated and imported separately
