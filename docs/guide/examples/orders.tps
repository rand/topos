spec OrderManagement

# Principles

- Consistency: Order state transitions are atomic
- Auditability: All changes are logged
- Idempotency: Duplicate requests are safe

# Requirements

## REQ-ORD-1: Shopping Cart

As a customer, I want to add items to my cart so I can purchase multiple items.

when: customer adds item to cart
the system shall: update cart with item and quantity

acceptance:
  given: product is in stock
  when: customer adds 2 units to cart
  then: cart shows product with quantity 2

acceptance:
  given: product already in cart with quantity 2
  when: customer adds 1 more unit
  then: cart shows product with quantity 3

## REQ-ORD-2: Checkout

As a customer, I want to checkout so I can complete my purchase.

when: customer submits checkout with valid payment
the system shall: create order and process payment

acceptance:
  given: cart has items and payment is valid
  when: customer completes checkout
  then: order is created with status "confirmed"

acceptance:
  given: payment fails
  when: customer completes checkout
  then: order is created with status "payment_failed"

## REQ-ORD-3: Order Status

As a customer, I want to track my order status.

when: customer views order
the system shall: display current status and history

## REQ-ORD-4: Order Cancellation

As a customer, I want to cancel my order if it hasn't shipped.

when: customer cancels unshipped order
the system shall: cancel order and initiate refund

acceptance:
  given: order status is "confirmed" or "processing"
  when: customer requests cancellation
  then: order status becomes "cancelled" and refund is initiated

acceptance:
  given: order status is "shipped"
  when: customer requests cancellation
  then: cancellation is rejected

# Concepts

Concept Cart:
  field id (`UUID`): unique
  field user_id (`UUID`): optional
  field session_id (`String`): optional
  field items (`List<CartItem>`)
  field created_at (`DateTime`)
  field updated_at (`DateTime`)

Concept CartItem:
  field product_id (`UUID`): required
  field quantity (`Int`): at least 1
  field price_at_addition (`Money`)

Concept Order:
  field id (`UUID`): unique
  field order_number (`String`): unique
  field user_id (`UUID`): required
  field status (`OrderStatus`): default: `pending`
  field items (`List<OrderItem>`)
  field subtotal (`Money`)
  field tax (`Money`)
  field shipping (`Money`)
  field total (`Money`)
  field shipping_address (`Address`)
  field billing_address (`Address`)
  field created_at (`DateTime`)
  field updated_at (`DateTime`)

Concept OrderItem:
  field product_id (`UUID`): required
  field product_name (`String`): required
  field quantity (`Int`): at least 1
  field unit_price (`Money`)
  field total (`Money`)

Concept Address:
  field line1 (`String`): required
  field line2 (`String`): optional
  field city (`String`): required
  field state (`String`): required
  field postal_code (`String`): required
  field country (`String`): required

# Behaviors

Behavior add_to_cart:
  input: `Cart`, `ProductId`, `Quantity`
  output: `Result<Cart, CartError>`

  requires:
    quantity > 0
    product exists and is available

  ensures:
    cart contains product with updated quantity
    cart.updated_at is current time

  errors:
    ProductNotFoundError: when product doesn't exist
    OutOfStockError: when insufficient inventory
    InvalidQuantityError: when quantity <= 0

Behavior remove_from_cart:
  input: `Cart`, `ProductId`
  output: `Result<Cart, CartError>`

  ensures:
    product is removed from cart
    cart.updated_at is current time

Behavior checkout:
  input: `Cart`, `PaymentMethod`, `ShippingAddress`, `BillingAddress`
  output: `Result<Order, CheckoutError>`

  requires:
    cart is not empty
    payment method is valid
    addresses are valid

  ensures:
    order is created with all items
    inventory is reserved
    payment is processed
    cart is cleared

  errors:
    EmptyCartError: when cart has no items
    PaymentFailedError: when payment processing fails
    InsufficientInventoryError: when items out of stock

Behavior cancel_order:
  input: `OrderId`, `UserId`
  output: `Result<Order, CancellationError>`

  requires:
    order belongs to user
    order status allows cancellation

  ensures:
    order.status == `cancelled`
    refund is initiated
    inventory is released

  errors:
    OrderNotFoundError: when order doesn't exist
    NotCancellableError: when order already shipped
    UnauthorizedError: when user doesn't own order

Behavior get_order_status:
  input: `OrderId`, `UserId`
  output: `Result<OrderStatus, OrderError>`

  requires:
    order belongs to user

  errors:
    OrderNotFoundError: when order doesn't exist
    UnauthorizedError: when user doesn't own order

# Tasks

## TASK-ORD-1: Implement Cart model [REQ-ORD-1]

Create Cart and CartItem entities.

file: src/models/cart.rs
tests: src/models/cart_test.rs
status: pending

## TASK-ORD-2: Implement Order model [REQ-ORD-2, REQ-ORD-3]

Create Order and OrderItem entities with status tracking.

file: src/models/order.rs
tests: src/models/order_test.rs
status: pending

## TASK-ORD-3: Implement cart endpoints [REQ-ORD-1]

POST/DELETE /api/cart/items endpoints.

file: src/api/cart.rs
tests: src/api/cart_test.rs
status: pending

## TASK-ORD-4: Implement checkout endpoint [REQ-ORD-2]

POST /api/checkout endpoint with payment integration.

file: src/api/checkout.rs
tests: src/api/checkout_test.rs
status: pending

## TASK-ORD-5: Implement order cancellation [REQ-ORD-4]

POST /api/orders/:id/cancel endpoint.

file: src/api/orders.rs
tests: src/api/orders_test.rs
status: pending
