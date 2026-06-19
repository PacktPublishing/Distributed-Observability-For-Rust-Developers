//! Inventory repository functions with OpenTelemetry instrumentation
//!
//! This module contains database operations for inventory/stock management,
//! separated from HTTP handlers. Each function is instrumented with
//! OpenTelemetry semantic conventions for database spans.

use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use tracing::{instrument, Span};
use uuid::Uuid;

use crate::models::{InventoryQueryParams, InventoryWithPricing};

/// List inventory with pagination and filtering
///
/// Queries the v_product_inventory_pricing view which combines
/// inventory and pricing data. Supports filtering by stock status,
/// product UUID, and stock quantity range.
#[instrument(
    name = "SELECT inventory",
    skip(pool, params),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_inventory",
        db.query.text = "SELECT * FROM v_product_inventory_pricing WHERE ... LIMIT ... OFFSET ...",
        otelmart.page = page,
        otelmart.page_size = page_size,
        otelmart.stock_status = ?params.stock_status
    )
)]
pub async fn list_inventory(
    pool: &PgPool,
    params: &InventoryQueryParams,
    page: i32,
    page_size: i32,
    offset: i32,
) -> Result<(Vec<InventoryWithPricing>, i64), sqlx::Error> {
    // Build count query with QueryBuilder for safe parameter binding
    let mut count_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM v_product_inventory_pricing");

    let mut has_filters = false;

    // Apply filters to count query
    if let Some(ref status) = params.stock_status {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        count_builder.push("stock_status = ");
        count_builder.push_bind(status);
    }

    if let Some(product_uuid) = params.product_uuid {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        count_builder.push("product_uuid = ");
        count_builder.push_bind(product_uuid);
    }

    if let Some(min_stock) = params.min_stock {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        count_builder.push("available_quantity >= ");
        count_builder.push_bind(min_stock);
    }

    if let Some(max_stock) = params.max_stock {
        count_builder.push(if has_filters { " AND " } else { " WHERE " });
        count_builder.push("available_quantity <= ");
        count_builder.push_bind(max_stock);
    }

    // Execute count query
    let total_count: i64 = count_builder.build_query_scalar().fetch_one(pool).await?;

    // Build main query with same filters
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM v_product_inventory_pricing");

    let mut has_filters = false;

    if let Some(ref status) = params.stock_status {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        query_builder.push("stock_status = ");
        query_builder.push_bind(status);
    }

    if let Some(product_uuid) = params.product_uuid {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        query_builder.push("product_uuid = ");
        query_builder.push_bind(product_uuid);
    }

    if let Some(min_stock) = params.min_stock {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        has_filters = true;
        query_builder.push("available_quantity >= ");
        query_builder.push_bind(min_stock);
    }

    if let Some(max_stock) = params.max_stock {
        query_builder.push(if has_filters { " AND " } else { " WHERE " });
        query_builder.push("available_quantity <= ");
        query_builder.push_bind(max_stock);
    }

    // Add ordering and pagination
    query_builder.push(" ORDER BY stock_status DESC, available_quantity ASC LIMIT ");
    query_builder.push_bind(page_size);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute main query
    let inventory = query_builder.build_query_as().fetch_all(pool).await?;

    Ok((inventory, total_count))
}

/// Get inventory for a specific product by UUID
#[instrument(
    name = "SELECT inventory",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_inventory",
        db.query.text = "SELECT * FROM v_product_inventory_pricing WHERE product_uuid = $1",
        otelmart.product.uuid = %product_uuid
    )
)]
pub async fn get_inventory_by_product(
    pool: &PgPool,
    product_uuid: Uuid,
) -> Result<Option<InventoryWithPricing>, sqlx::Error> {
    sqlx::query_as::<_, InventoryWithPricing>(
        r#"
        SELECT * FROM v_product_inventory_pricing
        WHERE product_uuid = $1
        "#,
    )
    .bind(product_uuid)
    .fetch_optional(pool)
    .await
}

/// Update stock quantity for a product
///
/// Returns the new available_quantity if the product was found, None otherwise.
#[instrument(
    name = "UPDATE inventory",
    skip(pool),
    fields(
        otel.kind = "client",
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "UPDATE",
        db.collection.name = "product_inventory",
        db.query.text = "UPDATE product_inventory SET stock_quantity = $1 ... WHERE product_uuid = $4",
        otelmart.product.uuid = %product_uuid,
        otelmart.quantity = quantity,
        db.response.returned_rows = tracing::field::Empty
    )
)]
pub async fn update_stock(
    pool: &PgPool,
    product_uuid: Uuid,
    quantity: i32,
    reorder_level: Option<i32>,
    reorder_quantity: Option<i32>,
) -> Result<Option<i32>, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE product_inventory
        SET stock_quantity = $1,
            reorder_level = COALESCE($2, reorder_level),
            reorder_quantity = COALESCE($3, reorder_quantity),
            last_restocked_at = CURRENT_TIMESTAMP
        WHERE product_uuid = $4
        RETURNING available_quantity
        "#,
    )
    .bind(quantity)
    .bind(reorder_level)
    .bind(reorder_quantity)
    .bind(product_uuid)
    .fetch_optional(pool)
    .await?;

    let rows = if result.is_some() { 1 } else { 0 };
    Span::current().record("db.response.returned_rows", rows);

    Ok(result.map(|row| row.get("available_quantity")))
}

/// Reserve stock for an order using the database function
///
/// Calls the reserve_stock PostgreSQL function which atomically checks
/// available stock and creates a reservation. Returns true if successful.
#[instrument(
    name = "SELECT reserve_stock",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_inventory",
        db.query.text = "SELECT reserve_stock($1, $2)",
        otelmart.product.uuid = %product_uuid,
        otelmart.quantity = quantity
    )
)]
pub async fn reserve_stock(
    pool: &PgPool,
    product_uuid: Uuid,
    quantity: i32,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(r#"SELECT reserve_stock($1, $2)"#)
        .bind(product_uuid)
        .bind(quantity)
        .fetch_one(pool)
        .await
}

/// Release reserved stock using the database function
#[instrument(
    name = "SELECT release_stock",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_inventory",
        db.query.text = "SELECT release_stock($1, $2)",
        otelmart.product.uuid = %product_uuid,
        otelmart.quantity = quantity
    )
)]
pub async fn release_stock(
    pool: &PgPool,
    product_uuid: Uuid,
    quantity: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(r#"SELECT release_stock($1, $2)"#)
        .bind(product_uuid)
        .bind(quantity)
        .execute(pool)
        .await?;
    Ok(())
}

/// Confirm a sale and decrease stock using the database function
#[instrument(
    name = "SELECT confirm_stock_sale",
    skip(pool),
    fields(
        db.system.name = "postgresql",
        db.namespace = "inventory",
        db.operation.name = "SELECT",
        db.collection.name = "product_inventory",
        db.query.text = "SELECT confirm_stock_sale($1, $2, $3)",
        otelmart.product.uuid = %product_uuid,
        otelmart.order.uuid = %order_uuid,
        otelmart.quantity = quantity
    )
)]
pub async fn confirm_sale(
    pool: &PgPool,
    product_uuid: Uuid,
    quantity: i32,
    order_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(r#"SELECT confirm_stock_sale($1, $2, $3)"#)
        .bind(product_uuid)
        .bind(quantity)
        .bind(order_uuid)
        .execute(pool)
        .await?;
    Ok(())
}
