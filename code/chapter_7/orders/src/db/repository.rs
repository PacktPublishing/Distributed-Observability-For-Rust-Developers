//! Order repository functions with OpenTelemetry instrumentation
//!
//! This module contains database operations for orders and shipments,
//! separated from HTTP handlers.
//! Each function is instrumented with OpenTelemetry semantic conventions for database spans.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction};
use tracing::{instrument, Span};
use uuid::Uuid;

use crate::models::{CreateOrderItemRequest, CreatePaymentRequest, CreateShippingAddressRequest, OrderComplete, Shipment};

/// Result of creating an order - contains the generated IDs
#[derive(Debug)]
pub struct CreatedOrder {
    pub id: i32,
    pub uuid: Uuid,
    pub order_number: String,
}

/// Totals calculated for an order
#[derive(Debug, Clone)]
pub struct OrderTotals {
    pub subtotal: BigDecimal,
    pub tax_amount: BigDecimal,
    pub shipping_amount: BigDecimal,
    pub total: BigDecimal,
}

/// Generate an order number using the database function
#[instrument(
    name = "SELECT generate_order_number",
    skip(tx),
    fields(
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "orders",
        db.query.text = "SELECT generate_order_number()"
    )
)]
pub async fn generate_order_number(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT generate_order_number()")
        .fetch_one(&mut **tx)
        .await
}

/// Create a new order record
///
/// Inserts the order header with customer info and totals.
/// Returns the generated order ID, UUID, and order number.
#[instrument(
    name = "INSERT orders",
    skip(tx, totals),
    fields(
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "INSERT",
        db.collection.name = "orders",
        db.query.text = "INSERT INTO orders (...) VALUES (...) RETURNING id, uuid",
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn create_order(
    tx: &mut Transaction<'_, Postgres>,
    order_number: &str,
    customer_email: &str,
    customer_phone: Option<&str>,
    totals: &OrderTotals,
) -> Result<CreatedOrder, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO orders (
            order_number, customer_email, customer_phone,
            subtotal, tax_amount, shipping_amount, total,
            status, payment_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', 'pending')
        RETURNING id, uuid
        "#,
    )
    .bind(order_number)
    .bind(customer_email)
    .bind(customer_phone)
    .bind(&totals.subtotal)
    .bind(&totals.tax_amount)
    .bind(&totals.shipping_amount)
    .bind(&totals.total)
    .fetch_one(&mut **tx)
    .await?;

    Span::current().record("db.response.returned_rows", 1);

    Ok(CreatedOrder {
        id: row.get("id"),
        uuid: row.get("uuid"),
        order_number: order_number.to_string(),
    })
}

/// Create order items (line items) for an order
///
/// Inserts all order items in sequence. Each item contains a snapshot
/// of the product data at the time of order.
#[instrument(
    name = "INSERT order_items",
    skip(tx, items),
    fields(
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "INSERT",
        db.collection.name = "order_items",
        db.query.text = "INSERT INTO order_items (...) VALUES (...)",
        otelmart.order.id = order_id,
        otelmart.items.count = items.len(),
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn create_order_items(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i32,
    items: &[CreateOrderItemRequest],
) -> Result<(), sqlx::Error> {
    let mut rows_inserted = 0;

    for item in items {
        let total_price = &item.unit_price * BigDecimal::from(item.quantity);
        sqlx::query(
            r#"
            INSERT INTO order_items (
                order_id, product_uuid, product_name, product_sku,
                quantity, unit_price, total_price
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(order_id)
        .bind(item.product_uuid)
        .bind(&item.product_name)
        .bind(&item.product_sku)
        .bind(item.quantity)
        .bind(&item.unit_price)
        .bind(&total_price)
        .execute(&mut **tx)
        .await?;

        rows_inserted += 1;
    }

    Span::current().record("db.response.returned_rows", rows_inserted);
    Ok(())
}

/// Create shipping address for an order
#[instrument(
    name = "INSERT shipping_addresses",
    skip(tx, address),
    fields(
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "INSERT",
        db.collection.name = "shipping_addresses",
        db.query.text = "INSERT INTO shipping_addresses (...) VALUES (...)",
        otelmart.order.id = order_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn create_shipping_address(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i32,
    address: &CreateShippingAddressRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO shipping_addresses (
            order_id, first_name, last_name, address_line1, address_line2,
            city, state, postal_code, country, phone
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(order_id)
    .bind(&address.first_name)
    .bind(&address.last_name)
    .bind(&address.address_line1)
    .bind(&address.address_line2)
    .bind(&address.city)
    .bind(&address.state)
    .bind(&address.postal_code)
    .bind(&address.country)
    .bind(&address.phone)
    .execute(&mut **tx)
    .await?;

    Span::current().record("db.response.returned_rows", 1);
    Ok(())
}

/// Generate a payment reference using the database function
#[instrument(
    name = "SELECT generate_payment_reference",
    skip(tx),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "payments"
    )
)]
pub async fn generate_payment_reference(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT generate_payment_reference()")
        .fetch_one(&mut **tx)
        .await
}

/// Create payment record for an order
///
/// NOTE: Payment is simulated - in production, this would call a payment gateway
/// and only insert the record after successful payment processing.
#[instrument(
    name = "INSERT payments",
    skip(tx, payment),
    fields(
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "INSERT",
        db.collection.name = "payments",
        db.query.text = "INSERT INTO payments (...) VALUES (...)",
        otelmart.order.id = order_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn create_payment(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i32,
    payment: &CreatePaymentRequest,
    payment_reference: &str,
    amount: &BigDecimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO payments (
            order_id, payment_method, amount, status,
            payment_reference, card_last4, card_brand, processed_at
        )
        VALUES ($1, $2, $3, 'paid', $4, $5, $6, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(order_id)
    .bind(&payment.payment_method)
    .bind(amount)
    .bind(payment_reference)
    .bind(&payment.card_last4)
    .bind(&payment.card_brand)
    .execute(&mut **tx)
    .await?;

    Span::current().record("db.response.returned_rows", 1);
    Ok(())
}

/// Update order status after successful payment
#[instrument(
    name = "UPDATE orders",
    skip(tx),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "UPDATE",
        db.collection.name = "orders",
        db.query.text = "UPDATE orders SET payment_status = $1, status = $2 WHERE id = $3",
        otelmart.order.id = order_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_order_payment_status(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i32,
    payment_status: &str,
    order_status: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE orders
        SET payment_status = $1, status = $2
        WHERE id = $3
        "#,
    )
    .bind(payment_status)
    .bind(order_status)
    .bind(order_id)
    .execute(&mut **tx)
    .await?;

    Span::current().record("db.response.returned_rows", result.rows_affected());
    Ok(())
}

/// Get order by UUID
#[instrument(
    name = "SELECT orders",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "orders",
        db.query.text = "SELECT * FROM v_orders_complete WHERE eid = $1",
        otelmart.order.uuid = %uuid
    )
)]
pub async fn get_order_by_uuid(
    pool: &PgPool,
    uuid: Uuid,
) -> Result<Option<OrderComplete>, sqlx::Error> {
    sqlx::query_as::<_, OrderComplete>(
        r#"SELECT * FROM v_orders_complete WHERE eid = $1"#,
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// Get order by email and order number (for guest users)
#[instrument(
    name = "SELECT orders",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "orders",
        db.query.text = "SELECT * FROM v_orders_complete WHERE customer_email = $1 AND order_number = $2"
    )
)]
pub async fn get_order_by_email_and_number(
    pool: &PgPool,
    email: &str,
    order_number: &str,
) -> Result<Option<OrderComplete>, sqlx::Error> {
    sqlx::query_as::<_, OrderComplete>(
        r#"
        SELECT * FROM v_orders_complete
        WHERE customer_email = $1 AND order_number = $2
        "#,
    )
    .bind(email)
    .bind(order_number)
    .fetch_optional(pool)
    .await
}

/// List orders for a user with pagination
#[instrument(
    name = "SELECT orders",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "orders",
        db.query.text = "SELECT * FROM v_orders_complete WHERE customer_email = $1 ...",
        otelmart.page = page,
        otelmart.page_size = page_size
    )
)]
pub async fn list_orders_by_email(
    pool: &PgPool,
    email: &str,
    status_filter: Option<&str>,
    payment_status_filter: Option<&str>,
    page: i32,
    page_size: i32,
) -> Result<(Vec<OrderComplete>, i64), sqlx::Error> {
    let offset = (page - 1) * page_size;

    // Build count query with proper parameterized filters
    let mut count_builder: QueryBuilder<Postgres> = 
        QueryBuilder::new("SELECT COUNT(*) FROM v_orders_complete WHERE customer_email = ");
    count_builder.push_bind(email);

    if let Some(status) = status_filter {
        count_builder.push(" AND status = ");
        count_builder.push_bind(status);
    }

    if let Some(payment_status) = payment_status_filter {
        count_builder.push(" AND payment_status = ");
        count_builder.push_bind(payment_status);
    }

    // Execute count query
    let total_count: (i64,) = count_builder
        .build_query_as()
        .fetch_one(pool)
        .await?;

    // Build main query with proper parameterized filters
    let mut query_builder: QueryBuilder<Postgres> = 
        QueryBuilder::new("SELECT * FROM v_orders_complete WHERE customer_email = ");
    query_builder.push_bind(email);

    if let Some(status) = status_filter {
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    if let Some(payment_status) = payment_status_filter {
        query_builder.push(" AND payment_status = ");
        query_builder.push_bind(payment_status);
    }

    query_builder.push(" ORDER BY ordered_at DESC LIMIT ");
    query_builder.push_bind(page_size);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute main query
    let orders: Vec<OrderComplete> = query_builder
        .build_query_as()
        .fetch_all(pool)
        .await?;

    Ok((orders, total_count.0))
}

/// Get the internal order ID from a public UUID
#[instrument(
    name = "SELECT orders",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "orders",
        db.query.text = "SELECT id FROM orders WHERE uuid = $1",
        otelmart.order.uuid = %order_uuid
    )
)]
pub async fn get_order_id_by_uuid(
    pool: &PgPool,
    order_uuid: Uuid,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM orders WHERE uuid = $1")
        .bind(order_uuid)
        .fetch_optional(pool)
        .await
}

/// Create a new shipment record for an order
///
/// Inserts a shipment with status 'shipped' and sets shipped_at to now.
#[instrument(
    name = "INSERT shipments",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "INSERT",
        db.collection.name = "shipments",
        db.query.text = "INSERT INTO shipments (...) VALUES (...) RETURNING *",
        otelmart.order.id = order_id,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn create_shipment(
    pool: &PgPool,
    order_id: i32,
    carrier: &str,
    tracking_number: Option<&str>,
    estimated_delivery_date: Option<NaiveDate>,
) -> Result<Shipment, sqlx::Error> {
    let shipment = sqlx::query_as::<_, Shipment>(
        r#"
        INSERT INTO shipments (
            order_id, carrier, tracking_number,
            estimated_delivery_date, status, shipped_at
        )
        VALUES ($1, $2, $3, $4, 'shipped', CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(order_id)
    .bind(carrier)
    .bind(tracking_number)
    .bind(estimated_delivery_date)
    .fetch_one(pool)
    .await?;

    Span::current().record("db.response.returned_rows", 1);
    Ok(shipment)
}

/// Update order status (e.g., to 'shipped' or 'delivered')
#[instrument(
    name = "UPDATE orders",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "UPDATE",
        db.collection.name = "orders",
        db.query.text = "UPDATE orders SET status = $1 WHERE id = $2",
        otelmart.order.id = order_id,
        otelmart.order.status = status,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_order_status(
    pool: &PgPool,
    order_id: i32,
    status: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE orders SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(order_id)
        .execute(pool)
        .await?;

    let rows = result.rows_affected();
    Span::current().record("db.response.returned_rows", rows);
    Ok(rows)
}

/// Update shipment status within a transaction
///
/// For 'delivered' status, also sets actual_delivery_date and delivered_at.
/// For other statuses, only updates the status field.
/// Returns true if a shipment was found and updated.
#[instrument(
    name = "UPDATE shipments",
    skip(tx),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "UPDATE",
        db.collection.name = "shipments",
        db.query.text = "UPDATE shipments SET status = $1 ... WHERE order_id = ...",
        otelmart.order.id = order_id,
        otelmart.shipment.status = status,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_shipment_status(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i32,
    status: &str,
    actual_delivery_date: Option<NaiveDate>,
) -> Result<bool, sqlx::Error> {
    let result = if status == "delivered" {
        sqlx::query(
            r#"
            UPDATE shipments
            SET status = $1, actual_delivery_date = $2, delivered_at = CURRENT_TIMESTAMP
            WHERE order_id = $3
            RETURNING id
            "#,
        )
        .bind(status)
        .bind(actual_delivery_date)
        .bind(order_id)
        .fetch_optional(&mut **tx)
        .await?
    } else {
        sqlx::query(
            r#"
            UPDATE shipments
            SET status = $1
            WHERE order_id = $2
            RETURNING id
            "#,
        )
        .bind(status)
        .bind(order_id)
        .fetch_optional(&mut **tx)
        .await?
    };

    let found = result.is_some();
    Span::current().record("db.response.returned_rows", if found { 1 } else { 0 });
    Ok(found)
}

/// Update order status within a transaction
#[instrument(
    name = "UPDATE orders",
    skip(tx),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "UPDATE",
        db.collection.name = "orders",
        db.query.text = "UPDATE orders SET status = $1 WHERE id = $2",
        otelmart.order.id = order_id,
        otelmart.order.status = status,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_order_status_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i32,
    status: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE orders SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(order_id)
        .execute(&mut **tx)
        .await?;

    let rows = result.rows_affected();
    Span::current().record("db.response.returned_rows", rows);
    Ok(rows)
}

/// Get order ID from UUID within a transaction
#[instrument(
    name = "SELECT orders",
    skip(tx),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "orders",
        db.operation.name = "SELECT",
        db.collection.name = "orders",
        db.query.text = "SELECT id FROM orders WHERE uuid = $1",
        otelmart.order.uuid = %order_uuid
    )
)]
pub async fn get_order_id_by_uuid_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    order_uuid: Uuid,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM orders WHERE uuid = $1")
        .bind(order_uuid)
        .fetch_optional(&mut **tx)
        .await
}
