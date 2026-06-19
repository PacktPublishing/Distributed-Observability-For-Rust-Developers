//! Order management API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use bigdecimal::BigDecimal;
use sqlx::{PgPool, Row};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::models::{
    CreateOrderRequest, OrderComplete, OrderQueryParams, OrdersResponse,
};
use crate::AppState;

/// Product detail response from products service
#[derive(Debug, serde::Deserialize)]
struct ProductDetail {
    eid: Uuid,
    product_name: String,
    final_price: BigDecimal,
    stock: i32,
    is_active: bool,
}

/// Request to reserve stock in inventory service
#[derive(Debug, serde::Serialize)]
struct ReserveStockRequest {
    product_uuid: Uuid,
    quantity: i32,
}

/// Request to release reserved stock in inventory service
#[derive(Debug, serde::Serialize)]
struct ReleaseStockRequest {
    product_uuid: Uuid,
    quantity: i32,
}

/// Request to confirm sale in inventory service
#[derive(Debug, serde::Serialize)]
struct ConfirmSaleRequest {
    product_uuid: Uuid,
    quantity: i32,
    order_uuid: Uuid,
}

/// Response from inventory stock operations
#[derive(Debug, serde::Deserialize)]
struct StockOperationResponse {
    success: bool,
    message: String,
    product_uuid: Uuid,
    available_quantity: Option<i32>,
}

/// Release reserved stock for a product in the inventory service
///
/// Calls the inventory service to release previously reserved stock.
/// Used for rollback when order creation fails.
async fn release_stock(state: &AppState, product_uuid: Uuid, quantity: i32) {
    let url = format!("{}/inventory/release", state.inventory_service_url());

    let request_body = ReleaseStockRequest {
        product_uuid,
        quantity,
    };

    let request = state
        .http_client()
        .post(&url)
        .json(&request_body);

    let response = request.send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                info!(
                    product_uuid = %product_uuid,
                    quantity = quantity,
                    "Successfully released stock"
                );
            } else {
                warn!(
                    product_uuid = %product_uuid,
                    status = %resp.status(),
                    "Failed to release stock"
                );
            }
        }
        Err(e) => {
            warn!(
                product_uuid = %product_uuid,
                error = %e,
                "Error calling inventory service to release stock"
            );
        }
    }
}

/// Release all reserved stock (for rollback scenarios)
///
/// Iterates through all reserved items and releases their stock.
/// Logs errors but doesn't fail - this is a best-effort cleanup.
async fn release_all_reserved_stock(state: &AppState, reserved_items: &[(Uuid, i32)]) {
    if reserved_items.is_empty() {
        return;
    }

    warn!(
        item_count = reserved_items.len(),
        "Rolling back stock reservations"
    );

    for (product_uuid, quantity) in reserved_items {
        release_stock(state, *product_uuid, *quantity).await;
    }
}

/// Confirm a sale with the inventory service
///
/// Calls the inventory service to confirm the sale and convert reserved stock to sold.
/// This is called after an order is successfully created and committed.
async fn confirm_sale(
    state: &AppState,
    product_uuid: Uuid,
    quantity: i32,
    order_uuid: Uuid,
) -> Result<(), String> {
    let url = format!("{}/inventory/confirm-sale", state.inventory_service_url());

    let request_body = ConfirmSaleRequest {
        product_uuid,
        quantity,
        order_uuid,
    };

    let request = state
        .http_client()
        .post(&url)
        .json(&request_body);

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to call inventory service: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Inventory service returned error status: {}",
            response.status()
        ))
    }
}

/// Confirm all sales for order items
///
/// Iterates through all order items and confirms the sale with inventory service.
/// Logs errors but doesn't fail the order since it's already committed.
async fn confirm_all_sales(state: &AppState, order_uuid: Uuid, items: &[(Uuid, i32)]) {
    if items.is_empty() {
        return;
    }

    info!(
        item_count = items.len(),
        order_uuid = %order_uuid,
        "Confirming sales with inventory service"
    );

    for (product_uuid, quantity) in items {
        match confirm_sale(state, *product_uuid, *quantity, order_uuid).await {
            Ok(_) => {
                info!(
                    product_uuid = %product_uuid,
                    quantity = quantity,
                    order_uuid = %order_uuid,
                    "Successfully confirmed sale"
                );
            }
            Err(e) => {
                // Log error but don't fail - order is already committed
                warn!(
                    product_uuid = %product_uuid,
                    order_uuid = %order_uuid,
                    error = %e,
                    "Failed to confirm sale - manual intervention may be required"
                );
            }
        }
    }
}

/// Reserve stock for a product in the inventory service
///
/// Calls the inventory service to reserve stock for an order.
/// Returns error if insufficient stock is available.
async fn reserve_stock(
    state: &AppState,
    product_uuid: Uuid,
    quantity: i32,
) -> Result<StockOperationResponse, axum::response::Response> {
    let url = format!("{}/inventory/reserve", state.inventory_service_url());

    let request_body = ReserveStockRequest {
        product_uuid,
        quantity,
    };

    let request = state
        .http_client()
        .post(&url)
        .json(&request_body);

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to call inventory service: {}", e);
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Inventory service unavailable",
                    "details": e.to_string()
                })),
            )
                .into_response());
        }
    };

    match response.status() {
        reqwest::StatusCode::OK => {
            let stock_response: StockOperationResponse = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to parse inventory response: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Failed to parse inventory response",
                            "details": e.to_string()
                        })),
                    )
                        .into_response());
                }
            };

            if !stock_response.success {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "Insufficient stock",
                        "product_uuid": product_uuid.to_string(),
                        "requested_quantity": quantity,
                        "available_quantity": stock_response.available_quantity,
                        "message": stock_response.message
                    })),
                )
                    .into_response());
            }

            Ok(stock_response)
        }
        reqwest::StatusCode::CONFLICT => {
            // Inventory service returned 409 - insufficient stock
            let error_response: serde_json::Value = response.json().await.unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Insufficient stock",
                    "product_uuid": product_uuid.to_string()
                })
            });

            Err((StatusCode::CONFLICT, Json(error_response)).into_response())
        }
        status => {
            eprintln!("Inventory service returned status: {}", status);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to reserve stock",
                    "status": status.as_u16()
                })),
            )
                .into_response())
        }
    }
}

/// Validate that a product exists and is available
///
/// Calls the products service to verify the product exists and is active.
/// Returns the product details for further validation (price, stock, etc.)
async fn validate_product(
    state: &AppState,
    product_uuid: Uuid,
) -> Result<ProductDetail, axum::response::Response> {
    let url = format!("{}/products/{}", state.products_service_url(), product_uuid);

    let request = state.http_client().get(&url);

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to call products service: {}", e);
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Products service unavailable",
                    "details": e.to_string()
                })),
            )
                .into_response());
        }
    };

    match response.status() {
        reqwest::StatusCode::OK => {
            let product: ProductDetail = match response.json().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse product response: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Failed to parse product data",
                            "details": e.to_string()
                        })),
                    )
                        .into_response());
                }
            };

            // Verify product is active
            if !product.is_active {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Product is not available",
                        "product_uuid": product_uuid.to_string(),
                        "product_name": product.product_name
                    })),
                )
                    .into_response());
            }

            Ok(product)
        }
        reqwest::StatusCode::NOT_FOUND => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Product not found",
                "product_uuid": product_uuid.to_string()
            })),
        )
            .into_response()),
        status => {
            eprintln!("Products service returned status: {}", status);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to validate product",
                    "status": status.as_u16()
                })),
            )
                .into_response())
        }
    }
}

/// Create a new order
///
/// # Endpoint
/// `POST /orders`
///
/// Creates a new order with items, shipping address, and payment info.
/// Automatically generates order number and calculates totals.
#[instrument(
    name = "create_order",
    skip(state, request),
    fields(
        customer_email = %request.customer_email,
        item_count = request.items.len()
    )
)]
pub async fn create_order(
    State(state): State<AppState>,
    Json(request): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    // Validate all products exist, are available, and have correct prices
    for item in &request.items {
        let product = match validate_product(&state, item.product_uuid).await {
            Ok(product) => product,
            Err(response) => {
                // Product validation failed - return error immediately
                return response;
            }
        };

        // Validate price matches actual product price (prevent price manipulation)
        if item.unit_price != product.final_price {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Price mismatch",
                    "product_uuid": item.product_uuid.to_string(),
                    "product_name": product.product_name,
                    "submitted_price": item.unit_price.to_string(),
                    "actual_price": product.final_price.to_string(),
                    "message": "The submitted price does not match the current product price"
                })),
            )
                .into_response();
        }
    }

    // Reserve stock for all items
    // Track reserved items so we can release them if something fails
    let mut reserved_items: Vec<(Uuid, i32)> = Vec::new();

    for item in &request.items {
        match reserve_stock(&state, item.product_uuid, item.quantity).await {
            Ok(_) => {
                reserved_items.push((item.product_uuid, item.quantity));
            }
            Err(response) => {
                // Stock reservation failed - release any already reserved stock
                release_all_reserved_stock(&state, &reserved_items).await;
                return response;
            }
        }
    }

    // Start a transaction
    let mut tx = match state.pool().begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            // Release reserved stock before returning error
            release_all_reserved_stock(&state, &reserved_items).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to start transaction",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Calculate subtotal from items
    let subtotal: BigDecimal = request
        .items
        .iter()
        .map(|item| &item.unit_price * BigDecimal::from(item.quantity))
        .sum();

    // Simple tax calculation (8% for demo)
    let tax_amount = &subtotal * BigDecimal::from(8) / BigDecimal::from(100);

    // Flat shipping rate for demo
    let shipping_amount = BigDecimal::from(10);

    let total = &subtotal + &tax_amount + &shipping_amount;

    // Generate order number
    let order_number: String = match sqlx::query_scalar("SELECT generate_order_number()")
        .fetch_one(&mut *tx)
        .await
    {
        Ok(num) => num,
        Err(e) => {
            eprintln!("Failed to generate order number: {}", e);
            // Release reserved stock before returning error
            release_all_reserved_stock(&state, &reserved_items).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to generate order number",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Insert order
    let order_result = sqlx::query(
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
    .bind(&order_number)
    .bind(&request.customer_email)
    .bind(&request.customer_phone)
    .bind(&subtotal)
    .bind(&tax_amount)
    .bind(&shipping_amount)
    .bind(&total)
    .fetch_one(&mut *tx)
    .await;

    let (order_id, order_uuid): (i32, Uuid) = match order_result {
        Ok(row) => (row.get("id"), row.get("uuid")),
        Err(e) => {
            eprintln!("Failed to create order: {}", e);
            let _ = tx.rollback().await;
            // Release reserved stock before returning error
            release_all_reserved_stock(&state, &reserved_items).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create order",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Insert order items
    for item in &request.items {
        let total_price = &item.unit_price * BigDecimal::from(item.quantity);
        if let Err(e) = sqlx::query(
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
        .execute(&mut *tx)
        .await
        {
            eprintln!("Failed to insert order item: {}", e);
            let _ = tx.rollback().await;
            // Release reserved stock before returning error
            release_all_reserved_stock(&state, &reserved_items).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to insert order item",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    }

    // Insert shipping address
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO shipping_addresses (
            order_id, first_name, last_name, address_line1, address_line2,
            city, state, postal_code, country, phone
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(order_id)
    .bind(&request.shipping_address.first_name)
    .bind(&request.shipping_address.last_name)
    .bind(&request.shipping_address.address_line1)
    .bind(&request.shipping_address.address_line2)
    .bind(&request.shipping_address.city)
    .bind(&request.shipping_address.state)
    .bind(&request.shipping_address.postal_code)
    .bind(&request.shipping_address.country)
    .bind(&request.shipping_address.phone)
    .execute(&mut *tx)
    .await
    {
        eprintln!("Failed to insert shipping address: {}", e);
        let _ = tx.rollback().await;
        // Release reserved stock before returning error
        release_all_reserved_stock(&state, &reserved_items).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to insert shipping address",
                "details": e.to_string()
            })),
        )
            .into_response();
    }

    // Generate payment reference
    let payment_reference: String = match sqlx::query_scalar("SELECT generate_payment_reference()")
        .fetch_one(&mut *tx)
        .await
    {
        Ok(ref_id) => ref_id,
        Err(e) => {
            eprintln!("Failed to generate payment reference: {}", e);
            let _ = tx.rollback().await;
            // Release reserved stock before returning error
            release_all_reserved_stock(&state, &reserved_items).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to generate payment reference",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Insert payment
    // NOTE: Payment is simulated - in production, this would call a payment gateway
    // If payment fails, we need to release reserved stock
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO payments (
            order_id, payment_method, amount, status,
            payment_reference, card_last4, card_brand, processed_at
        )
        VALUES ($1, $2, $3, 'paid', $4, $5, $6, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(order_id)
    .bind(&request.payment.payment_method)
    .bind(&total)
    .bind(&payment_reference)
    .bind(&request.payment.card_last4)
    .bind(&request.payment.card_brand)
    .execute(&mut *tx)
    .await
    {
        eprintln!("Failed to insert payment: {}", e);
        let _ = tx.rollback().await;
        // Payment failed - release reserved stock
        release_all_reserved_stock(&state, &reserved_items).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to process payment",
                "details": e.to_string()
            })),
        )
            .into_response();
    }

    // Update order payment status
    if let Err(e) = sqlx::query(
        r#"
        UPDATE orders
        SET payment_status = 'paid', status = 'processing'
        WHERE id = $1
        "#,
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await
    {
        eprintln!("Failed to update order status: {}", e);
        let _ = tx.rollback().await;
        // Release reserved stock before returning error
        release_all_reserved_stock(&state, &reserved_items).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to update order status",
                "details": e.to_string()
            })),
        )
            .into_response();
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        // Transaction commit failed - release reserved stock
        release_all_reserved_stock(&state, &reserved_items).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to commit transaction",
                "details": e.to_string()
            })),
        )
            .into_response();
    }

    // Transaction committed successfully!
    // Now confirm the sale with inventory service to convert reserved stock to sold
    confirm_all_sales(&state, order_uuid, &reserved_items).await;

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "order_number": order_number,
            "order_uuid": order_uuid,
            "message": "Order created successfully"
        })),
    )
        .into_response()
}

/// List orders with pagination and filtering
///
/// # Endpoint
/// `GET /orders`
///
/// Supports two modes:
/// 1. Authenticated user: Requires X-User-Email header, returns all orders for that user
/// 2. Guest user: Requires both email and order_number query params, returns single order
#[instrument(
    name = "list_orders",
    skip(state, headers),
    fields(
        page = params.page,
        page_size = params.page_size
    )
)]
pub async fn list_orders(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<OrderQueryParams>,
) -> impl IntoResponse {
    // Check for authenticated user via X-User-Email header
    let user_email = headers
        .get("X-User-Email")
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    if let Some(email) = user_email {
        // Authenticated user: return all their orders
        return list_user_orders(state.pool(), &email, &params).await;
    }

    // Guest user: require both email and order_number
    match (&params.customer_email, &params.order_number) {
        (Some(email), Some(order_number)) => {
            get_guest_order(state.pool(), email, order_number).await
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Guest users must provide both 'customer_email' and 'order_number' query parameters"
            })),
        )
            .into_response(),
    }
}

/// List all orders for an authenticated user
async fn list_user_orders(
    pool: &PgPool,
    user_email: &str,
    params: &OrderQueryParams,
) -> axum::response::Response {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let mut query = String::from("SELECT * FROM v_orders_complete WHERE customer_email = $1");
    let mut count_query = String::from("SELECT COUNT(*) FROM v_orders_complete WHERE customer_email = $1");

    // Apply additional filters if provided
    if let Some(ref status) = params.status {
        let filter = format!(" AND status = '{}'", status.replace('\'', "''"));
        query.push_str(&filter);
        count_query.push_str(&filter);
    }

    if let Some(ref payment_status) = params.payment_status {
        let filter = format!(" AND payment_status = '{}'", payment_status.replace('\'', "''"));
        query.push_str(&filter);
        count_query.push_str(&filter);
    }

    query.push_str(&format!(
        " ORDER BY ordered_at DESC LIMIT {} OFFSET {}",
        page_size, offset
    ));

    // Execute count query
    let total_count: (i64,) = match sqlx::query_as(&count_query)
        .bind(user_email)
        .fetch_one(pool)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Database error counting orders: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to count orders",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    // Execute main query
    let orders: Vec<OrderComplete> = match sqlx::query_as(&query)
        .bind(user_email)
        .fetch_all(pool)
        .await
    {
        Ok(orders) => orders,
        Err(e) => {
            eprintln!("Database error fetching orders: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to fetch orders",
                    "details": e.to_string()
                })),
            )
                .into_response();
        }
    };

    let total_pages = ((total_count.0 as f64) / (page_size as f64)).ceil() as i32;

    let response = OrdersResponse {
        orders,
        total_count: total_count.0,
        page,
        page_size,
        total_pages,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Get a single order for a guest user
async fn get_guest_order(
    pool: &PgPool,
    email: &str,
    order_number: &str,
) -> axum::response::Response {
    let result = sqlx::query_as::<_, OrderComplete>(
        r#"
        SELECT * FROM v_orders_complete
        WHERE customer_email = $1 AND order_number = $2
        "#,
    )
    .bind(email)
    .bind(order_number)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(order)) => {
            // Return as single-item list for consistency
            let response = OrdersResponse {
                orders: vec![order],
                total_count: 1,
                page: 1,
                page_size: 1,
                total_pages: 1,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Order not found or email does not match"
            })),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Database error fetching order: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to fetch order",
                    "details": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// Get order details by UUID
///
/// # Endpoint
/// `GET /orders/{uuid}`
#[instrument(
    name = "get_order_by_id",
    skip(state),
    fields(order.uuid = %uuid)
)]
pub async fn get_order_by_id(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, OrderComplete>(
        r#"
        SELECT * FROM v_orders_complete WHERE eid = $1
        "#,
    )
    .bind(uuid)
    .fetch_optional(state.pool())
    .await;

    match result {
        Ok(Some(order)) => (StatusCode::OK, Json(order)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Order not found",
                "uuid": uuid.to_string()
            })),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Database error fetching order: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to fetch order",
                    "details": e.to_string()
                })),
            )
                .into_response()
        }
    }
}
