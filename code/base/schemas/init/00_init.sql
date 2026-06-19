-- ============================================================
-- INITIALIZATION SCRIPT
-- ============================================================
-- Creates database extensions and schemas for microservices

-- Enable UUID extension globally
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================
-- CREATE SCHEMAS (PostgreSQL namespaces)
-- ============================================================

-- Products Schema - for Products Service
CREATE SCHEMA IF NOT EXISTS products;
COMMENT ON SCHEMA products IS 'Products microservice schema - handles catalog, categories, specifications, reviews';

-- Inventory Schema - for Inventory Service
CREATE SCHEMA IF NOT EXISTS inventory;
COMMENT ON SCHEMA inventory IS 'Inventory microservice schema - handles stock management, pricing, discounts';

-- Orders Schema - for Orders/Checkout Service
CREATE SCHEMA IF NOT EXISTS orders;
COMMENT ON SCHEMA orders IS 'Orders microservice schema - handles orders, payments, shipping, checkout';

-- Users Schema - for Users/Authentication Service
CREATE SCHEMA IF NOT EXISTS users;
COMMENT ON SCHEMA users IS 'Users microservice schema - handles user management, authentication, sessions, addresses';

-- ============================================================
-- SET DEFAULT SEARCH PATH
-- ============================================================
-- This allows cross-schema references to work properly
-- Services will override this with their own schema as default

-- Note: Cannot use CURRENT_DATABASE() in ALTER DATABASE
-- Each connection should set search_path as needed
