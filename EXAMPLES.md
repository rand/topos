# Topos Examples

Real-world specifications demonstrating the full spec-driven workflow.

## Example 1: Task Management System

A complete specification showing all layers.

```topos
spec TaskManagement

# Principles

- Test-First: All implementation follows TDD
- Simplicity: No abstractions without documented justification
- Accessibility: All UI meets WCAG 2.1 AA standards
- Security: All inputs validated, no SQL injection


# Requirements

## REQ-1: Task Creation

As a team member, I want to create tasks so that I can track my work.

when: user submits task creation form with valid title
the system shall: create a new task with status "todo"

when: user submits task creation form with empty title
the system shall: display validation error "Title is required"

when: user submits task creation form while unauthenticated
the system shall: redirect to login page

acceptance:
  given: authenticated user on task list page
  when: user enters title "Fix login bug" and clicks Create
  then: new task appears in list with status "todo"
  then: task shows creator as current user

acceptance:
  given: authenticated user on task list page
  when: user clicks Create with empty title
  then: validation error displayed
  then: no task created


## REQ-2: Task Assignment

As a team lead, I want to assign tasks to team members so that work is distributed.

when: user assigns task to team member
the system shall: update task assignee and notify the member

when: user assigns task to non-existent member
the system shall: display error "User not found"

acceptance:
  given: task with no assignee
  when: user selects "Alice" from assignee dropdown
  then: task shows "Alice" as assignee
  then: Alice receives notification


## REQ-3: Task Completion

As a team member, I want to mark tasks as done so that I can show progress.

when: user marks task as done
the system shall: update status to "done" and record completion timestamp

when: user marks already-done task as done
the system shall: take no action (idempotent)

acceptance:
  given: task with status "in_progress"
  when: user clicks "Mark Done"
  then: task status becomes "done"
  then: completion timestamp recorded


## REQ-4: Task Filtering

As a team member, I want to filter tasks by status so I can focus on relevant work.

when: user selects status filter
the system shall: display only tasks matching selected status

acceptance:
  given: tasks exist with various statuses
  when: user selects filter "in_progress"
  then: only in-progress tasks displayed


# Design

## Architecture

Layered architecture with separation of concerns:

- `Controllers`: HTTP handling, request validation
- `Services`: Business logic, orchestration
- `Repositories`: Data persistence
- `Models`: Domain entities

## Components

- `TaskController`: REST API endpoints for task CRUD
- `TaskService`: Task business logic
- `TaskRepository`: PostgreSQL persistence
- `NotificationService`: Email/push notifications
- `UserService`: User lookup and validation

## Data Flow

Create Task:
User → TaskController → TaskService → TaskRepository → Database

Assign Task:
User → TaskController → TaskService → TaskRepository → Database
                            ↓
                   NotificationService → Email

## Technology Stack

- Runtime: Node.js 20
- Framework: Express.js
- Database: PostgreSQL 15
- ORM: Prisma
- Testing: Jest + Supertest


# Concepts

Concept Task:
  A unit of work to be tracked.
  
  field id (`Identifier`): unique
  field title (`String`): at least 1 character
  field description (`Optional` `String`)
  field status (`TaskStatus`): default: `todo`
  field creator (`User`)
  field assignee (`Optional` `User`)
  field created_at (`DateTime`)
  field completed_at (`Optional` `DateTime`)


Concept TaskStatus:
  one of: todo, in_progress, blocked, done


Concept User:
  A system user who can create and be assigned tasks.
  
  field id (`Identifier`): unique
  field email (`Email`): unique
  field name (`String`)


# Behaviors

Behavior create_task:
  Implements REQ-1.
  
  given:
    title (`String`)
    description (`Optional` `String`)
    creator (`User`)
    
  returns: `Task` or `ValidationError`
  
  when: `title` is not empty
  the system shall: create task with `status` = `todo`
  
  when: `title` is empty
  the system shall: return `ValidationError` with message "Title is required"
  
  ensures:
    `result.creator` = `creator`
    `result.created_at` = now
    `result.status` is `todo`


Behavior assign_task:
  Implements REQ-2.
  
  given:
    task (`Task`)
    assignee (`User`)
    
  returns: `Task` or `UserNotFoundError`
  
  requires:
    `task.status` ≠ `done`
    
  ensures:
    `result.assignee` = `assignee`
    notification sent to `assignee` [~]


Behavior complete_task:
  Implements REQ-3.
  
  given:
    task (`Task`)
    
  returns: `Task`
  
  when: `task.status` ≠ `done`
  the system shall: set `status` = `done` and record `completed_at`
  
  when: `task.status` is `done`
  the system shall: return task unchanged (idempotent)
  
  ensures:
    `result.status` is `done`
    `result.completed_at` exists


Behavior filter_tasks:
  Implements REQ-4.
  
  given:
    status_filter (`Optional` `TaskStatus`)
    
  returns: `List` of `Task`
  
  when: `status_filter` is provided
  the system shall: return tasks where `status` = `status_filter`
  
  when: `status_filter` is empty
  the system shall: return all tasks


# Invariants

Invariant completed_tasks_have_timestamp:
  for each `task` in `Task`:
    `task.status` is `done` → `task.completed_at` exists


Invariant tasks_have_creators:
  for each `task` in `Task`:
    `task.creator` exists


# Tasks

## TASK-1: Create Task model [REQ-1, REQ-2, REQ-3]

Implement Prisma schema and TypeScript model.

file: prisma/schema.prisma
file: src/models/task.ts
tests: src/models/task.test.ts
status: pending

subtasks:
  - Define Prisma Task model with all fields
  - Generate Prisma client
  - Create TypeScript Task interface
  - Add validation helpers


## TASK-2: Create TaskRepository [REQ-1, REQ-2, REQ-3, REQ-4]

Implement data access layer.

file: src/repositories/task-repository.ts
tests: src/repositories/task-repository.test.ts
depends: TASK-1
status: pending

subtasks:
  - Implement create method
  - Implement findById method
  - Implement update method
  - Implement findAll with filters
  - Write integration tests


## TASK-3: Create TaskService [REQ-1, REQ-2, REQ-3]

Implement business logic layer.

file: src/services/task-service.ts
tests: src/services/task-service.test.ts
depends: TASK-2
status: pending

subtasks:
  - Implement createTask with validation
  - Implement assignTask with notification
  - Implement completeTask with idempotency
  - Write unit tests with mocks


## TASK-4: Create TaskController [REQ-1, REQ-2, REQ-3, REQ-4]

Implement REST API endpoints.

file: src/controllers/task-controller.ts
tests: src/controllers/task-controller.test.ts
depends: TASK-3
status: pending

subtasks:
  - POST /api/tasks (create)
  - GET /api/tasks (list with filters)
  - PATCH /api/tasks/:id (update)
  - POST /api/tasks/:id/complete
  - Write integration tests


## TASK-5: Add notification on assignment [REQ-2]

Integrate with NotificationService.

file: src/services/notification-service.ts
tests: src/services/notification-service.test.ts
depends: TASK-3
status: pending
```

## Example 2: Authentication System

Security-focused specification with strong principles.

```topos
spec Authentication

# Principles

- Security-First: All security decisions documented and reviewed
- No Plaintext: Passwords never stored or logged in plaintext
- Defense-in-Depth: Multiple layers of protection
- Audit Trail: All auth events logged
- Test-First: Security tests before implementation


# Requirements

## REQ-1: User Registration

As a visitor, I want to create an account so I can access the system.

when: user submits registration with valid email and strong password
the system shall: create account with unverified status

when: user submits registration with weak password
the system shall: reject with password requirements message

when: user submits registration with existing email
the system shall: reject with "email already registered"

acceptance:
  given: visitor on registration page
  when: enters "user@example.com" and strong password
  then: account created with unverified status
  then: verification email sent


## REQ-2: Email Verification

As a registered user, I want to verify my email so I can fully access the system.

when: user clicks valid verification link
the system shall: mark account as verified

when: user clicks expired verification link
the system shall: offer to resend verification email

acceptance:
  given: unverified account exists
  when: user clicks verification link within 24 hours
  then: account status becomes verified


## REQ-3: User Login

As a verified user, I want to log in to access my account.

when: user submits correct credentials
the system shall: create session with JWT token

when: user submits incorrect password
the system shall: increment failed attempt counter

when: failed attempts exceed 5 within 15 minutes
the system shall: temporarily lock account

acceptance:
  given: verified user account
  when: user enters correct password
  then: JWT token returned
  then: login event logged


# Design

## Security Architecture

- Passwords: bcrypt with cost factor 12
- Tokens: JWT with RS256, 15-minute expiry
- Sessions: Redis-backed with refresh tokens
- Rate limiting: Per-IP and per-account

## Components

- `AuthService`: Authentication logic
- `UserRepository`: User persistence
- `TokenService`: JWT generation/validation
- `RateLimiter`: Brute-force protection


# Concepts

Concept User:
  field id (`Identifier`): unique
  field email (`Email`): unique
  field password_hash (`PasswordHash`)
  field status (`UserStatus`)
  field failed_attempts (`Natural`): default: 0
  field locked_until (`Optional` `DateTime`)


Concept UserStatus:
  one of: unverified, active, suspended, locked


Concept Session:
  field id (`Identifier`)
  field user_id (`Identifier`)
  field access_token (`JWT`)
  field refresh_token (`RefreshToken`)
  field expires_at (`DateTime`)


# Behaviors

Behavior register:
  Implements REQ-1.
  
  given:
    email (`Email`)
    password (`String`)
    
  returns: `User` or `RegistrationError`
  
  requires:
    `password` meets strength requirements
    no user exists with `email`
    
  ensures:
    `result.password_hash` = hash(`password`)
    `result.status` is `unverified`
    verification email queued [~]


Behavior login:
  Implements REQ-3.
  
  given:
    email (`Email`)
    password (`String`)
    
  returns: `Session` or `AuthError`
  
  requires:
    user exists with `email`
    user.status is `active`
    user not locked
    
  when: password matches
  the system shall: create session and reset failed attempts
  
  when: password does not match
  the system shall: increment failed attempts, check for lockout


# Invariants

Invariant no_plaintext_passwords:
  for each `user` in `User`:
    `user.password_hash` is bcrypt hash


Invariant locked_users_have_lockout_time:
  for each `user` in `User`:
    `user.status` is `locked` → `user.locked_until` exists


# Tasks

## TASK-1: Create User model [REQ-1]
file: src/models/user.ts
tests: src/models/user.test.ts
status: pending

## TASK-2: Create AuthService [REQ-1, REQ-3]
file: src/services/auth-service.ts
tests: src/services/auth-service.test.ts
depends: TASK-1
status: pending

## TASK-3: Create TokenService [REQ-3]
file: src/services/token-service.ts
tests: src/services/token-service.test.ts
status: pending

## TASK-4: Create RateLimiter [REQ-3]
file: src/middleware/rate-limiter.ts
tests: src/middleware/rate-limiter.test.ts
status: pending
```

## Example 3: Payment Integration

Third-party integration specification.

```topos
spec StripePayments

# Principles

- PCI-Compliance: No card data touches our servers
- Idempotency: All payment operations use idempotency keys
- Audit Trail: All transactions logged with Stripe IDs
- Graceful Degradation: Handle Stripe outages gracefully


# Requirements

## REQ-1: Process Payment

As a customer, I want to pay for my order so I can complete checkout.

when: customer submits valid payment method
the system shall: charge the payment method via Stripe

when: payment succeeds
the system shall: update order status to paid

when: payment fails
the system shall: display error and allow retry

acceptance:
  given: order pending payment
  when: customer submits valid card
  then: Stripe charge created
  then: order status becomes "paid"


## REQ-2: Refund Payment

As a support agent, I want to refund payments so I can resolve disputes.

when: agent initiates full refund
the system shall: create Stripe refund for full amount

when: agent initiates partial refund
the system shall: create Stripe refund for specified amount

acceptance:
  given: paid order
  when: agent clicks "Full Refund"
  then: Stripe refund created
  then: order status becomes "refunded"


# Design

## Integration Architecture

```
Customer → Checkout → PaymentService → Stripe API
                          ↓
                    OrderService (status update)
                          ↓
                    EventLog (audit)
```

## Stripe Integration

- API Version: 2023-10-16
- Webhook signing: STRIPE_WEBHOOK_SECRET
- Idempotency: Order ID as idempotency key


# Concepts

Concept Payment:
  field id (`Identifier`): unique
  field order_id (`Identifier`)
  field amount (`Money`)
  field status (`PaymentStatus`)
  field stripe_charge_id (`Optional` `String`)
  field stripe_refund_id (`Optional` `String`)


Concept PaymentStatus:
  one of: pending, processing, completed, failed, refunded


# Behaviors

Behavior process_payment:
  Implements REQ-1.
  
  given:
    order (`Order`)
    payment_method_id (`String`)  # Stripe token
    
  returns: `Payment` or `PaymentError`
  
  requires:
    `order.status` is `pending_payment`
    `order.total` > 0
    
  ensures:
    `result.stripe_charge_id` exists
    `order.status` becomes `paid` [~]
    payment event logged [~]


Behavior refund_payment:
  Implements REQ-2.
  
  given:
    payment (`Payment`)
    
  returns: `Payment` or `RefundError`
  
  requires:
    `payment.status` is `completed`
    `payment.stripe_charge_id` exists
    
  ensures:
    `result.status` is `refunded`
    `result.stripe_refund_id` exists
    refund event logged [~]


# Tasks

## TASK-1: Create Payment model [REQ-1, REQ-2]
file: src/models/payment.ts
status: pending

## TASK-2: Create Stripe client wrapper [REQ-1, REQ-2]
file: src/integrations/stripe-client.ts
tests: src/integrations/stripe-client.test.ts
status: pending

## TASK-3: Create PaymentService [REQ-1, REQ-2]
file: src/services/payment-service.ts
tests: src/services/payment-service.test.ts
depends: TASK-1, TASK-2
status: pending

## TASK-4: Create webhook handler [REQ-1]
file: src/webhooks/stripe-webhook.ts
tests: src/webhooks/stripe-webhook.test.ts
depends: TASK-3
status: pending
```

## Example 4: Event Processing with Typed Holes

Specification for an event-driven system where handlers aren't fully designed yet.

```topos
spec OrderEventProcessing

# Principles

- Idempotency: All event handlers are idempotent
- Audit: All events logged before and after processing
- Resilience: Failed events are retried with backoff


# Requirements

## REQ-1: Order Event Processing

As the system, I need to process order lifecycle events to maintain consistency.

when: OrderPlaced event received
the system shall: validate inventory and initiate payment

when: PaymentCompleted event received
the system shall: confirm order and queue for fulfillment

when: ShipmentReady event received
the system shall: generate tracking and notify customer


# Concepts

Concept OrderEvent:
  one of: OrderPlaced, PaymentCompleted, PaymentFailed, 
          ShipmentReady, Delivered, Cancelled

Concept OrderPlaced:
  field order_id (`Identifier`)
  field items (`List` of `OrderItem`)
  field customer_id (`Identifier`)

Concept PaymentCompleted:
  field order_id (`Identifier`)
  field transaction_id (`Identifier`)
  field amount (`Money`)

Concept ShipmentReady:
  field order_id (`Identifier`)
  field warehouse_id (`Identifier`)
  field packages (`List` of `Package`)

Concept ProcessingResult:
  one of: Success, Retry, DeadLetter

Concept EventContext:
  field event_id (`Identifier`)
  field timestamp (`DateTime`)
  field retry_count (`Natural`)


# Behaviors

Behavior handle_event:
  Implements REQ-1.
  
  given:
    event (`OrderEvent`)
    context (`EventContext`)
    
  returns: `ProcessingResult`
  
  requires:
    `context.retry_count` < 5
    
  when: `event` is `OrderPlaced`
  the system shall: [?handle_placed : `OrderPlaced` -> `ProcessingResult`
                     involving: `Inventory`, `PaymentIntent`
                     where: inventory checked before payment initiated]
  
  when: `event` is `PaymentCompleted`
  the system shall: [?handle_payment : `PaymentCompleted` -> `ProcessingResult`
                     involving: `Order`, `Fulfillment`
                     where: order status updated atomically]
  
  when: `event` is `ShipmentReady`
  the system shall: [?handle_shipment : `ShipmentReady` -> `ProcessingResult`
                     involving: `Tracking`, `Notification`
                     where: customer notified after tracking generated]


# Partially Specified Handler

Behavior handle_placed:
  Implements REQ-1.
  
  The handler for OrderPlaced events.
  
  given:
    event (`OrderPlaced`)
    context (`EventContext`)
    
  returns: `ProcessingResult`
  
  requires:
    `event.items` not empty
    
  ensures:
    inventory check via [?inventory_check : `List` of `OrderItem` -> `InventoryResult`
                         where: all items checked atomically]
    
    if inventory available:
      payment initiated via [?payment_init : (`Order`, `Money`) -> `PaymentIntent`
                             where: amount matches order total
                             where: idempotency key = `event.order_id`]
    
    if inventory unavailable:
      [?backorder_flow : `OrderPlaced` -> `BackorderNotification`
       involving: `Customer`, `Inventory`, `Email`]


# Tasks

## TASK-1: Create event type definitions [REQ-1]
file: src/events/types.ts
status: done

## TASK-2: Create event router [REQ-1]
file: src/events/router.ts
depends: TASK-1
status: done

## TASK-3: Implement handle_placed [REQ-1]
file: src/handlers/order-placed.ts
depends: TASK-1, TASK-2
status: pending

note: Blocked on [?inventory_check] and [?payment_init] design

## TASK-4: Implement handle_payment [REQ-1]
file: src/handlers/payment-completed.ts
depends: TASK-1, TASK-2
status: pending

note: Blocked on [?handle_payment] design

## TASK-5: Implement handle_shipment [REQ-1]
file: src/handlers/shipment-ready.ts
depends: TASK-1, TASK-2
status: pending

note: Blocked on [?handle_shipment] design
```

This example shows:
- **Typed holes for handlers**: We know inputs/outputs but not implementation
- **`involving:` for related concepts**: Documents what's connected
- **`where:` constraints**: Captures known invariants
- **Nested holes**: `handle_placed` has its own typed holes
- **Task blocking**: Tasks note which holes block them


## Patterns and Idioms

### Pattern: EARS Variations

```topos
# Event-driven (most common)
when: user clicks Submit
the system shall: save the form

# State-driven
while: system is in maintenance mode
the system shall: return 503 for all requests

# Unwanted behavior (errors)
if: file exceeds 10MB
the system shall: reject upload with size error

# Optional feature
where: user has premium subscription
the system shall: enable advanced features
```

### Pattern: Acceptance Criteria Variations

```topos
# Single scenario
acceptance:
  given: logged-in user
  when: clicks Logout
  then: session destroyed
  then: redirected to home

# Multiple scenarios (AND - all must pass)
acceptance:
  given: empty cart
  when: user adds item
  then: cart shows 1 item
  then: cart total updated

acceptance:
  given: cart with 3 items
  when: user removes 1 item
  then: cart shows 2 items
```

### Pattern: Task Dependencies

```topos
# Linear chain
## TASK-1: Model
## TASK-2: Repository
depends: TASK-1
## TASK-3: Service
depends: TASK-2
## TASK-4: Controller
depends: TASK-3

# Parallel tasks
## TASK-5: Frontend component
depends: TASK-3
## TASK-6: API documentation
depends: TASK-4

# Multiple dependencies
## TASK-7: Integration tests
depends: TASK-4, TASK-5
```

### Pattern: Informal to Formal Progression

```topos
# Start informal
ensures:
  user notified [~]
  
# Later, formalize
ensures:
  `NotificationService.send` called with `user.email` [~]
  
# Finally, fully formal
ensures:
  notification sent to `user.email` within 60 seconds
  notification contains order confirmation link
```

### Pattern: Typed Hole Refinement

```topos
# Start with untyped hole
ensures:
  payment processed [?]

# Add type bounds
ensures:
  payment processed [? `Payment` -> `Receipt`]

# Name it for tracking
ensures:
  payment processed [?payment_flow : `Payment` -> `Receipt`]

# Add constraints as you learn them
ensures:
  payment processed [?payment_flow : `Payment` -> `Receipt`
                     where: `payment.amount` > 0
                     where: `receipt.timestamp` = now]

# Eventually extract to full behavior
Behavior process_payment:
  given: payment (`Payment`)
  returns: `Receipt`
  requires: `payment.amount` > 0
  ensures: `result.timestamp` = now
```
