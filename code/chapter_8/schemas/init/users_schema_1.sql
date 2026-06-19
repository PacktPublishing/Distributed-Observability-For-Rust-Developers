-- ============================================================
-- USERS SCHEMA - USERS & AUTHENTICATION SERVICE
-- ============================================================
-- Handles: User management, authentication, sessions, addresses
-- Schema: users
-- Port: 3004

-- Set search path to users schema
SET search_path TO users, public;

-- ============================================================
-- USERS TABLE
-- ============================================================
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Authentication
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL, -- bcrypt hash

    -- Personal info
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    phone VARCHAR(50),

    -- Account status
    is_active BOOLEAN DEFAULT true,
    is_verified BOOLEAN DEFAULT false,

    -- Verification
    email_verification_token VARCHAR(255),
    email_verified_at TIMESTAMP WITH TIME ZONE,

    -- Password reset
    password_reset_token VARCHAR(255),
    password_reset_expires_at TIMESTAMP WITH TIME ZONE,

    -- Timestamps
    last_login_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_uuid ON users(uuid);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_is_active ON users(is_active);
CREATE INDEX idx_users_email_verification_token ON users(email_verification_token);
CREATE INDEX idx_users_password_reset_token ON users(password_reset_token);

COMMENT ON TABLE users IS 'Registered user accounts';
COMMENT ON COLUMN users.uuid IS 'External UUID (exposed as eid in API)';
COMMENT ON COLUMN users.password_hash IS 'Hashed password using bcrypt';

-- ============================================================
-- USER_SESSIONS TABLE
-- ============================================================
CREATE TABLE user_sessions (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- User reference
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Session data
    session_token VARCHAR(255) NOT NULL UNIQUE,
    refresh_token VARCHAR(255) UNIQUE,

    -- Session lifecycle
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_activity_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Status
    is_active BOOLEAN DEFAULT true,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_sessions_uuid ON user_sessions(uuid);
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_session_token ON user_sessions(session_token);
CREATE INDEX idx_user_sessions_is_active ON user_sessions(is_active);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);

COMMENT ON TABLE user_sessions IS 'Active user sessions for authentication';
COMMENT ON COLUMN user_sessions.session_token IS 'JWT or session token';
COMMENT ON COLUMN user_sessions.refresh_token IS 'Token for refresh';

-- ============================================================
-- USER_ADDRESSES TABLE
-- ============================================================
CREATE TABLE user_addresses (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- User reference
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Address type
    address_type VARCHAR(50) DEFAULT 'shipping', -- 'shipping', 'billing', 'both'
    address_label VARCHAR(100), -- 'Home', 'Work', etc.

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

    -- Flags
    is_default BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_addresses_uuid ON user_addresses(uuid);
CREATE INDEX idx_user_addresses_user_id ON user_addresses(user_id);
CREATE INDEX idx_user_addresses_address_type ON user_addresses(address_type);
CREATE INDEX idx_user_addresses_is_default ON user_addresses(is_default);

COMMENT ON TABLE user_addresses IS 'User saved addresses';
COMMENT ON COLUMN user_addresses.is_default IS 'Default address for this type';

-- ============================================================
-- USER_PROFILES TABLE
-- ============================================================
CREATE TABLE user_profiles (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- User reference (one-to-one)
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,

    -- Profile information
    avatar_url TEXT,
    date_of_birth DATE,

    -- Preferences
    email_notifications BOOLEAN DEFAULT true,
    marketing_emails BOOLEAN DEFAULT false,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_profiles_uuid ON user_profiles(uuid);
CREATE INDEX idx_user_profiles_user_id ON user_profiles(user_id);

COMMENT ON TABLE user_profiles IS 'User profile information and preferences';

-- ============================================================
-- GUEST_USERS TABLE
-- ============================================================
CREATE TABLE guest_users (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,

    -- Guest identification
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(50),

    -- Conversion tracking
    converted_to_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    converted_at TIMESTAMP WITH TIME ZONE,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_guest_users_uuid ON guest_users(uuid);
CREATE INDEX idx_guest_users_email ON guest_users(email);
CREATE INDEX idx_guest_users_converted_to_user_id ON guest_users(converted_to_user_id);

COMMENT ON TABLE guest_users IS 'Guest users who can later register';
COMMENT ON COLUMN guest_users.converted_to_user_id IS 'User ID if guest registered';

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

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_sessions_updated_at BEFORE UPDATE ON user_sessions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_addresses_updated_at BEFORE UPDATE ON user_addresses
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_profiles_updated_at BEFORE UPDATE ON user_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_guest_users_updated_at BEFORE UPDATE ON guest_users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- TRIGGER: Ensure only one default address per user per type
-- ============================================================

CREATE OR REPLACE FUNCTION ensure_single_default_address()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_default = true THEN
        -- Unset other default addresses of the same type for this user
        UPDATE user_addresses
        SET is_default = false
        WHERE user_id = NEW.user_id
          AND address_type = NEW.address_type
          AND id != NEW.id
          AND is_default = true;
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER ensure_single_default_address_trigger
BEFORE INSERT OR UPDATE ON user_addresses
    FOR EACH ROW
    WHEN (NEW.is_default = true)
    EXECUTE FUNCTION ensure_single_default_address();

-- ============================================================
-- FUNCTION: Generate session token
-- ============================================================

CREATE OR REPLACE FUNCTION generate_session_token()
RETURNS TEXT AS $$
BEGIN
    RETURN 'SES-' ||
           EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT::TEXT || '-' ||
           SUBSTRING(MD5(RANDOM()::TEXT), 1, 32);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION generate_session_token IS 'Generate unique session token';

-- ============================================================
-- FUNCTION: Revoke user session
-- ============================================================

CREATE OR REPLACE FUNCTION revoke_user_session(p_session_token VARCHAR)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated INTEGER;
BEGIN
    UPDATE user_sessions
    SET is_active = false
    WHERE session_token = p_session_token
      AND is_active = true;

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION revoke_user_session IS 'Revoke a session (logout)';

-- ============================================================
-- FUNCTION: Convert guest to registered user
-- ============================================================

CREATE OR REPLACE FUNCTION convert_guest_to_user(
    p_guest_uuid UUID,
    p_user_id INTEGER
)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated INTEGER;
BEGIN
    UPDATE guest_users
    SET converted_to_user_id = p_user_id,
        converted_at = CURRENT_TIMESTAMP
    WHERE uuid = p_guest_uuid
      AND converted_to_user_id IS NULL;

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION convert_guest_to_user IS 'Mark guest as converted to registered user';

-- ============================================================
-- VIEWS
-- ============================================================

-- Users with default addresses
CREATE VIEW v_users_with_addresses AS
SELECT
    u.id,
    u.uuid as eid,
    u.email,
    u.first_name,
    u.last_name,
    u.phone,
    u.is_verified,
    u.last_login_at,

    -- Default shipping address (as JSON)
    (
        SELECT jsonb_build_object(
            'eid', ua.uuid,
            'address_label', ua.address_label,
            'first_name', ua.first_name,
            'last_name', ua.last_name,
            'address_line1', ua.address_line1,
            'address_line2', ua.address_line2,
            'city', ua.city,
            'state', ua.state,
            'postal_code', ua.postal_code,
            'country', ua.country,
            'phone', ua.phone
        )
        FROM user_addresses ua
        WHERE ua.user_id = u.id
          AND ua.address_type IN ('shipping', 'both')
          AND ua.is_default = true
          AND ua.is_active = true
        LIMIT 1
    ) as default_shipping_address,

    -- Default billing address (as JSON)
    (
        SELECT jsonb_build_object(
            'eid', ua.uuid,
            'address_label', ua.address_label,
            'first_name', ua.first_name,
            'last_name', ua.last_name,
            'address_line1', ua.address_line1,
            'address_line2', ua.address_line2,
            'city', ua.city,
            'state', ua.state,
            'postal_code', ua.postal_code,
            'country', ua.country,
            'phone', ua.phone
        )
        FROM user_addresses ua
        WHERE ua.user_id = u.id
          AND ua.address_type IN ('billing', 'both')
          AND ua.is_default = true
          AND ua.is_active = true
        LIMIT 1
    ) as default_billing_address,

    u.created_at
FROM users u
WHERE u.is_active = true;

-- Guest users with conversion status
CREATE VIEW v_guest_users AS
SELECT
    g.uuid as eid,
    g.email,
    g.phone,
    g.converted_to_user_id IS NOT NULL as is_converted,
    u.email as registered_email,
    g.converted_at,
    g.created_at
FROM guest_users g
LEFT JOIN users u ON g.converted_to_user_id = u.id;

COMMENT ON VIEW v_users_with_addresses IS 'Users with their default addresses';
COMMENT ON VIEW v_guest_users IS 'Guest users with conversion status';

-- ============================================================
-- CONSTRAINTS & CHECKS
-- ============================================================

-- Email format validation
ALTER TABLE users ADD CONSTRAINT check_email_format
    CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$');

-- Password hash must not be empty
ALTER TABLE users ADD CONSTRAINT check_password_hash_not_empty
    CHECK (LENGTH(password_hash) > 0);

-- Session token must not be empty
ALTER TABLE user_sessions ADD CONSTRAINT check_session_token_not_empty
    CHECK (LENGTH(session_token) > 0);

-- ============================================================
-- SAMPLE DATA (for testing)
-- ============================================================

/*
-- Example: Create a registered user with profile and address

-- 1. Create user
INSERT INTO users (
    email,
    password_hash,
    first_name,
    last_name,
    phone,
    is_verified
) VALUES (
    'john.doe@example.com',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzpLaEiL4e', -- bcrypt hash
    'John',
    'Doe',
    '+1-555-0100',
    true
) RETURNING id, uuid;

-- 2. Create profile
INSERT INTO user_profiles (
    user_id,
    email_notifications
) VALUES (1, true);

-- 3. Add shipping address
INSERT INTO user_addresses (
    user_id,
    address_type,
    address_label,
    first_name,
    last_name,
    address_line1,
    city,
    state,
    postal_code,
    country,
    phone,
    is_default
) VALUES (
    1,
    'shipping',
    'Home',
    'John',
    'Doe',
    '123 Main St',
    'New York',
    'NY',
    '10001',
    'US',
    '+1-555-0100',
    true
);

-- 4. Create session (login)
INSERT INTO user_sessions (
    user_id,
    session_token,
    expires_at
) VALUES (
    1,
    generate_session_token(),
    CURRENT_TIMESTAMP + INTERVAL '7 days'
);

-- 5. Logout (revoke session)
SELECT revoke_user_session('SES-...');

-- Example: Guest user registration

-- 1. Create guest user (during guest checkout)
INSERT INTO guest_users (email, phone)
VALUES ('guest@example.com', '+1-555-0200')
RETURNING uuid;

-- 2. Later, guest decides to register
INSERT INTO users (email, password_hash, first_name, last_name, phone)
VALUES ('guest@example.com', 'hash...', 'Jane', 'Smith', '+1-555-0200')
RETURNING id;

-- 3. Link guest to registered user
SELECT convert_guest_to_user('guest-uuid', 2);
*/
