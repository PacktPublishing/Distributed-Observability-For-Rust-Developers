//! Order management API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use bigdecimal::BigDecimal;
use opentelemetry::KeyValue;
use sqlx::PgPool;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::{self, with_transaction, OrderTotals};
use crate::metrics::metrics;
use crate::models::{
    CreateOrderRequest, OrderQueryParams, OrdersResponse,
};
use crate::AppState;

/// Product detail response from products service
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)] // Fields deserialized from products service response, not all used directly
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
#[allow(dead_code)] // Fields deserialized from inventory service response, not all used directly
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

/// Order creation error type
#[derive(Debug)]
pub enum OrderError {
    Database(sqlx::Error),
}

impl From<sqlx::Error> for OrderError {
    fn from(err: sqlx::Error) -> Self {
        OrderError::Database(err)
    }
}

/// Create a new order
///
/// # Endpoint
/// `POST /orders`
///
/// Creates a new order with items, shipping address, and payment info.
/// Automatically generates order number and calculates totals.
///
/// Uses the `with_transaction` wrapper to ensure all database operations
/// are atomic and instrumented with OpenTelemetry semantic conventions.
pub async fn create_order(
    State(state): State<AppState>,
    Json(request): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    // Start timing and record a checkout attempt
    let start = Instant::now();
    metrics().checkout_attempts.add(1, &[]);
    
    // Business KPI: Conversion funnel started
    metrics().funnel_started.add(1, &[]);

    // Validate all products exist, are available, and have correct prices
    for item in &request.items {
        let product = match validate_product(&state, item.product_uuid).await {
            Ok(product) => product,
            Err(response) => {
                // Product validation failed — record checkout failure
                record_checkout_failure("product_validation", &start);
                return response;
            }
        };

        // Validate price matches actual product price (prevent price manipulation)
        if item.unit_price != product.final_price {
            record_checkout_failure("price_mismatch", &start);
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
    
    // Business KPI: Payment info validated (all products and prices confirmed)
    metrics().funnel_payment_info.add(1, &[]);

    // Reserve stock for all items
    // Track reserved items so we can release them if something fails
    let mut reserved_items: Vec<(Uuid, i32)> = Vec::new();

    for item in &request.items {
        match reserve_stock(&state, item.product_uuid, item.quantity).await {
            Ok(_) => {
                reserved_items.push((item.product_uuid, item.quantity));
            }
            Err(response) => {
                // Stock reservation failed — record failure and release already reserved stock
                record_checkout_failure("inventory_reservation", &start);
                release_all_reserved_stock(&state, &reserved_items).await;
                return response;
            }
        }
    }

    // Calculate totals
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
    
    let totals = OrderTotals {
        subtotal,
        tax_amount,
        shipping_amount,
        total: total.clone(),
    };

    // Clone data needed for the transaction closure
    let customer_email = request.customer_email.clone();
    let customer_phone = request.customer_phone.clone();
    let items = request.items.clone();
    let shipping_address = request.shipping_address.clone();
    let payment = request.payment.clone();

    // Execute all database operations within an instrumented transaction
    let result = with_transaction(state.pool(), "checkout", |tx| {
        // Move owned values into the closure
        let customer_email = customer_email.clone();
        let customer_phone = customer_phone.clone();
        let items = items.clone();
        let shipping_address = shipping_address.clone();
        let payment = payment.clone();
        let totals = totals.clone();
        let total = total.clone();

        Box::pin(async move {
            // Generate order number
            let order_number = db::generate_order_number(tx).await?;

            // Create order record
            let created_order = db::create_order(
                tx,
                &order_number,
                &customer_email,
                customer_phone.as_deref(),
                &totals,
            )
            .await?;

            // Create order items
            db::create_order_items(tx, created_order.id, &items).await?;

            // Create shipping address
            db::create_shipping_address(tx, created_order.id, &shipping_address).await?;

            // Generate payment reference and create payment
            let payment_reference = db::generate_payment_reference(tx).await?;
            db::create_payment(
                tx,
                created_order.id,
                &payment,
                &payment_reference,
                &total,
            )
            .await?;

            // Update order status to processing after successful payment
            db::update_order_payment_status(tx, created_order.id, "paid", "processing").await?;

            Ok::<_, OrderError>(created_order)
        })
    })
    .await;

    match result {
        Ok(created_order) => {
            // Transaction committed successfully!
            // Record successful checkout metrics
            let duration = start.elapsed().as_secs_f64();
            metrics().checkout_duration.record(duration, &[
                KeyValue::new("outcome", "success"),
            ]);
            
            // Business KPI: Conversion funnel completed
            metrics().funnel_completed.add(1, &[]);
            // Business KPI: Time from checkout start to completion
            metrics().time_to_checkout.record(duration, &[]);
            
            // Record the order total as a dollar-value histogram
            metrics().order_total_amount.record(
                bigdecimal_to_f64(&total),
                &[],
            );
            // Record how many line items were in this order
            metrics().order_items_count.record(request.items.len() as u64, &[]);

            // Now confirm the sale with inventory service to convert reserved stock to sold
            confirm_all_sales(&state, created_order.uuid, &reserved_items).await;

            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "success": true,
                    "order_number": created_order.order_number,
                    "order_uuid": created_order.uuid,
                    "message": "Order created successfully"
                })),
            )
                .into_response()
        }

        Err(e) => {
            // Transaction failed (rolled back automatically)
            // Release reserved stock and record failure
            record_checkout_failure("order_creation", &start);
            release_all_reserved_stock(&state, &reserved_items).await;

            let error_msg = match e {
                OrderError::Database(db_err) => {
                    tracing::error!(error = %db_err, "Database error during order creation");
                    format!("Database error: {}", db_err)
                }
            };

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create order",
                    "details": error_msg
                })),
            )
                .into_response()
        }
    }
}

/// List orders with pagination and filtering
///
/// # Endpoint
/// `GET /orders`
///
/// Supports two modes:
/// 1. Authenticated user: Requires X-User-Email header, returns all orders for that user
/// 2. Guest user: Requires both email and order_number query params, returns single order
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
    let result = db::list_orders_by_email(
        pool,
        user_email,
        params.status.as_deref(),
        params.payment_status.as_deref(),
        page,
        page_size,
    )
    .await;

    match result {
        Ok((orders, total_count)) => {
            let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i32;

            let response = OrdersResponse {
                orders,
                total_count,
                page,
                page_size,
                total_pages,
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching orders");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to fetch orders",
                    "details": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// Get a single order for a guest user
async fn get_guest_order(
    pool: &PgPool,
    email: &str,
    order_number: &str,
) -> axum::response::Response {
    let result = db::get_order_by_email_and_number(pool, email, order_number).await;

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
            tracing::error!(error = %e, "Database error fetching order");
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
pub async fn get_order_by_id(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> impl IntoResponse {
    let result = db::get_order_by_uuid(state.pool(), uuid).await;

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
            tracing::error!(error = %e, "Database error fetching order");
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

// ---------------------------------------------------------------------------
// Metric helper functions
// ---------------------------------------------------------------------------

/// Records a checkout failure with the given reason.
///
/// Increments the failure counter and records the duration histogram
/// with `outcome=failure` and a `failure.reason` attribute.
fn record_checkout_failure(reason: &str, start: &Instant) {
    let duration = start.elapsed().as_secs_f64();
    metrics().checkout_failures.add(1, &[
        KeyValue::new("failure.reason", reason.to_string()),
    ]);
    metrics().checkout_duration.record(duration, &[
        KeyValue::new("outcome", "failure"),
        KeyValue::new("failure.reason", reason.to_string()),
    ]);
}

/// Produces a short hex hash of the input string (e.g. an email address)
/// for use as a PII-safe span attribute.
#[allow(dead_code)]
fn hash_short(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Converts a `BigDecimal` to `f64` for metric recording.
/// Falls back to 0.0 if the conversion is lossy.
fn bigdecimal_to_f64(value: &BigDecimal) -> f64 {
    use std::str::FromStr;
    f64::from_str(&value.to_string()).unwrap_or(0.0)
}
