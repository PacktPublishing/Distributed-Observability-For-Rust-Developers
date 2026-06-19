// Global type definitions
declare global {
  interface Window {
    cartBtnPosition: () => DOMRect;
  }

  interface ProductCard {
    eid: string;
    product_name: string;
    description: string | null;
    category_name: string | null;
    final_price: string;
    initial_price: string | null;
    discount: string | null;
    stock: number;
    average_rating: string | null;
    reviews_count: number | null;
    brand: string | null;
  }

  interface Product {
    eid: string;
    product_id: string;
    sku: string;
    gtin: string | null;
    product_name: string;
    brand: string | null;
    description: string | null;
    url: string | null;
    final_price: string;
    initial_price: string | null;
    discount: string | null;
    currency: string;
    category_name: string | null;
    root_category_name: string | null;
    available_for_delivery: boolean | null;
    available_for_pickup: boolean | null;
    free_returns: boolean | null;
    sizes: string[];
    colors: string[];
    stock: number;
    is_active: boolean;
    deleted_at: string | null;
    created_at: string;
    updated_at: string;
    data_timestamp: string | null;
  }

  interface User {
    eid: string;
    email: string;
    is_active: boolean;
    deleted_at: string | null;
    created_at: string;
    updated_at: string;
  }

  // Order Models
  interface Order {
    id: number;
    eid: string;
    user_id: number | null;
    guest_user_id: number | null;
    order_number: string;
    customer_email: string;
    customer_phone: string | null;
    subtotal: string;
    tax_amount: string;
    shipping_amount: string;
    total: string;
    status: OrderStatus;
    payment_status: PaymentStatus;
    is_guest_order: boolean | null;
    created_at: string;
    updated_at: string;
  }

  interface OrderItem {
    id: number;
    eid: string;
    order_id: number;
    product_id: number;
    product_name: string;
    product_sku: string;
    quantity: number;
    unit_price: string;
    total_price: string;
    created_at: string;
  }

  interface ShippingAddress {
    id: number;
    eid: string;
    order_id: number;
    first_name: string;
    last_name: string;
    address_line1: string;
    address_line2: string | null;
    city: string;
    state: string;
    postal_code: string;
    country: string;
    phone: string | null;
    created_at: string;
    updated_at: string;
  }

  interface Payment {
    id: number;
    eid: string;
    order_id: number;
    payment_method: PaymentMethod;
    payment_reference: string | null;
    amount: string;
    status: PaymentStatus;
    processed_at: string | null;
    card_last4: string | null;
    card_brand: string | null;
    created_at: string;
    updated_at: string;
  }

  // Order Request/Response Types
  interface CreateShippingAddressRequest {
    first_name: string;
    last_name: string;
    address_line1: string;
    address_line2?: string | null;
    city: string;
    state: string;
    postal_code: string;
    country: string;
    phone?: string | null;
  }

  interface CreateOrderItemRequest {
    product_uuid: string;
    product_name: string;
    product_sku?: string | null;
    quantity: number;
    unit_price: string;
  }

  interface CreatePaymentRequest {
    payment_method: PaymentMethod;
    card_last4?: string | null;
    card_brand?: string | null;
  }

  interface CreateOrderRequest {
    customer_email: string;
    customer_phone?: string | null;
    shipping_address: CreateShippingAddressRequest;
    items: CreateOrderItemRequest[];
    payment: CreatePaymentRequest;
  }

  interface OrderResponse {
    order: Order;
    items: OrderItem[];
    shipping_address: ShippingAddress;
    payment: Payment;
  }

  interface PaginatedResponse<T = any> {
    products: T[];
    page: number;
    page_size: number;
    total: number;
    total_pages: number;
  }

  // Enums (matching Rust serde snake_case serialization)
  type OrderStatus = 'pending' | 'processing' | 'shipped' | 'delivered' | 'cancelled' | 'failed' | 'on_hold';
  type PaymentStatus = 'pending' | 'paid' | 'failed' | 'refunded' | 'partially_refunded' | 'cancelled' | 'authorized';
  type PaymentMethod = 'credit_card' | 'debit_card';

  interface AddtoCartEvent<T = HTMLElement> {
    product: ProductCard;
    quantity: number;
    element: T;
  }

  interface CartItem {
    product: ProductCard;
    quantity: number;
    addedAt?: Date;
  }
}

export {};
