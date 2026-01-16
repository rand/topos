# Typed Holes in Topos

Topos supports **typed holes**—placeholders for unspecified behavior that still carry partial information about their shape, constraints, and relationships. This enables gradual refinement from vague intent to precise specification.

## The Problem with Opaque Holes

A simple `[?]` marker says "something goes here" but nothing more:

```topos
Behavior process_order:
  given: order (`Order`)
  returns: `ProcessedOrder`
  
  ensures:
    payment handled [?]  # What does this mean? What types? What constraints?
```

This hole is opaque. You can't navigate to related concepts, can't see what's constrained, can't refine incrementally.

## Typed Holes

A typed hole declares what's known even when the full specification isn't:

```topos
Behavior process_order:
  given: order (`Order`)
  returns: `ProcessedOrder`
  
  ensures:
    payment handled [? `PaymentMethod` -> `PaymentResult`]
```

Now we know:
- Something processes a `PaymentMethod`
- Something produces a `PaymentResult`
- We can navigate to those types
- We can refine the hole with that signature in mind

## Syntax

### Basic Typed Hole

```topos
[? Type]                           # Hole of specific type
[? Type -> Type]                   # Hole with input -> output
[? (Type, Type) -> Type]           # Multiple inputs
[? Type -> Type | ErrorType]       # Output with error case
```

### Named Typed Hole

```topos
[?payment_processing : `PaymentMethod` -> `PaymentResult`]
[?queue_handler : `Message` -> `Acknowledgment`]
```

Named holes can be referenced elsewhere and tracked across the spec.

### Hole with Constraints

```topos
[? `Order` -> `ShippingLabel`
   where: `order.status` is `paid`
   where: `order.items` all have `weight`]
```

### Hole with Partial Signature

```topos
[? given:
     order (`Order`)
     carrier (`ShippingCarrier`)
   returns: `ShippingLabel` or `ShippingError`
   requires:
     `order.address` is valid
   ensures:
     [?] ]  # Postconditions unknown
```

### Hole Referencing Concepts

```topos
[? involving: `Payment`, `Order`, `Refund`]
```

This says "something happens here involving these concepts" without specifying the exact relationship.

## Semantics

### Type Information

Typed holes carry type information that:

1. **Constrains refinement**: When you fill the hole, it must be compatible with the declared types
2. **Enables navigation**: LSP can jump to referenced types
3. **Supports inference**: Adjacent constraints may narrow the hole's type
4. **Documents intent**: Readers understand the shape even without details

### Hole Identity

Named holes have identity across the spec:

```topos
Behavior initiate_payment:
  ensures:
    [?payment_flow : `PaymentIntent` -> `PaymentResult`]

Behavior complete_payment:
  requires:
    payment intent exists  # From [?payment_flow]
  ensures:
    [?payment_flow] completes  # Same hole, elaborated
```

### Refinement

Holes can be progressively refined:

```topos
# Initial: very vague
[?]

# Add type bounds
[? `Input` -> `Output`]

# Add constraints  
[? `Input` -> `Output` where: `input` is valid]

# Add partial behavior
[? given: input (`Input`)
   returns: `Output`
   requires: `input` is valid
   ensures: [?] ]

# Finally: full specification
Behavior process:
  given: input (`Input`)
  returns: `Output`
  requires: `input` is valid
  ensures: `result` corresponds to `input`
```

Each step preserves and extends what's known.

## Use Cases

### 1. External System Integration

You know the interface but not the implementation:

```topos
Behavior send_notification:
  Implements REQ-5.
  
  given:
    user (`User`)
    message (`NotificationContent`)
    
  returns: `NotificationResult`
  
  ensures:
    notification delivered via [?channel : `NotificationChannel` -> `DeliveryReceipt`
                                 involving: `Email`, `Push`, `SMS`]
```

### 2. Algorithm Placeholder

You know inputs/outputs but not the algorithm:

```topos
Behavior optimize_route:
  given:
    stops (`List` of `Location`)
    constraints (`RouteConstraints`)
    
  returns: `OptimizedRoute`
  
  ensures:
    route visits all `stops`
    route respects `constraints`
    optimization via [?routing_algorithm : (`List` of `Location`, `RouteConstraints`) -> `OptimizedRoute`
                      where: result is [~] "reasonably optimal"]
```

### 3. Future Feature

You know it's needed but not designed yet:

```topos
Concept Order:
  field id (`Identifier`)
  field items (`List` of `OrderItem`)
  field discounts [?discount_system : `List` of `Discount`
                   involving: `Coupon`, `Promotion`, `LoyaltyPoints`]
```

### 4. Queue/Event Processing

You know the message flow but not the handlers:

```topos
Behavior handle_order_events:
  given:
    event (`OrderEvent`)
    
  returns: `Acknowledgment`
  
  when: `event` is `OrderPlaced`
  the system shall: [?placed_handler : `OrderPlaced` -> `ProcessingStarted`]
  
  when: `event` is `PaymentReceived`  
  the system shall: [?payment_handler : `PaymentReceived` -> `FulfillmentQueued`]
  
  when: `event` is `ShipmentReady`
  the system shall: [?shipment_handler : `ShipmentReady` -> `TrackingGenerated`]
```

### 5. Cross-Cutting Concern

You know something applies but not exactly how:

```topos
Invariant audit_trail:
  for each mutation in [`create_order`, `update_order`, `cancel_order`]:
    [?audit_logging : `Mutation` -> `AuditRecord`
     where: `record.timestamp` = now
     where: `record.actor` = current user
     where: `record.before` and `record.after` captured]
```

## LSP Support

### Hover on Typed Hole

```
[?payment_flow : `PaymentMethod` -> `PaymentResult`]

┌─ Typed Hole ─────────────────────────────────────────┐
│ Name: payment_flow                                   │
│                                                      │
│ Signature:                                           │
│   Input:  PaymentMethod                              │
│   Output: PaymentResult                              │
│                                                      │
│ Referenced in:                                       │
│   • initiate_payment (line 45)                       │
│   • complete_payment (line 67)                       │
│                                                      │
│ Related concepts:                                    │
│   • PaymentMethod (Ctrl+Click to navigate)          │
│   • PaymentResult (Ctrl+Click to navigate)          │
│                                                      │
│ Status: Unresolved                                   │
└──────────────────────────────────────────────────────┘
```

### Go to Definition

Ctrl+Click on types within a hole navigates to their definitions:

```topos
[? `PaymentMethod` -> `PaymentResult`]
       ↑                    ↑
       │                    └── Ctrl+Click → Concept PaymentResult
       └── Ctrl+Click → Concept PaymentMethod
```

### Find All References

"Find references to `PaymentMethod`" includes typed holes that reference it:

```
References to PaymentMethod (5):

  ✓ Concept PaymentMethod (definition)
      concepts.tps:45
      
  ✓ Behavior process_payment
      behaviors.tps:23 - field payment_method (`PaymentMethod`)
      
  ✓ Typed Hole [?payment_flow]
      behaviors.tps:67 - [? `PaymentMethod` -> `PaymentResult`]
      
  ...
```

### Diagnostics

```
I002: Typed hole with partial specification

  45 │ [?payment_flow : `PaymentMethod` -> `PaymentResult`]
         ^^^^^^^^^^^^^
     │ 
     │ Hole has type constraints but no implementation.
     │ 
     │ Input type:  PaymentMethod (defined at concepts.tps:12)
     │ Output type: PaymentResult (defined at concepts.tps:34)
     │ 
     │ Referenced by: 2 behaviors
     │ 
     │ Actions:
     │   • Expand to full behavior specification
     │   • Mark as [future] if intentionally deferred
```

### Completion

After typing `[?`, completion offers:

```
Suggestions:
  [? `Type` -> `Type`]           Insert typed hole
  [?name : `Type`]               Insert named typed hole
  [? given: ... returns: ...]    Insert hole with signature
  [? involving: ...]             Insert hole with concept references
```

### LLM-Powered Suggestions (MCP)

When an Anthropic API key is configured, the `suggest_hole` MCP tool provides intelligent suggestions for filling typed holes based on:

- **Type constraints**: If the hole has a type hint, suggestions respect it
- **Parent context**: The concept field, behavior parameter, or invariant where the hole appears
- **Related concepts**: Other types referenced in the surrounding spec
- **Adjacent constraints**: Requirements or invariants that constrain the hole

Example:

```topos
Concept Session:
  field id (`UUID`)
  field user_id (`UUID`)
  field created_at (`Timestamp`)
  field expires_at ([?])  # What type should this be?
```

The LLM analyzes the context and suggests:

```
1. `Timestamp` (confidence: 95%)
   Consistent with created_at field pattern and session expiration semantics
   Type-based: true

2. `Duration` (confidence: 75%)
   Alternative representation as time-to-live rather than absolute time
   Type-based: false
```

To enable LLM suggestions, set `ANTHROPIC_API_KEY` in your environment or `.env` file. See the [README Configuration section](README.md#configuration) for details.

## Refinement Workflow

### Step 1: Identify the Gap

```topos
Behavior checkout:
  ensures:
    order processed [?]  # Vague
```

### Step 2: Add Type Bounds

```topos
Behavior checkout:
  ensures:
    order processed [? `Cart` -> `Order`]
```

### Step 3: Name It

```topos
Behavior checkout:
  ensures:
    order processed [?cart_to_order : `Cart` -> `Order`]
```

### Step 4: Add Constraints

```topos
Behavior checkout:
  ensures:
    order processed [?cart_to_order : `Cart` -> `Order`
                     where: `cart.items` not empty
                     where: `order.total` = sum of `cart.items`]
```

### Step 5: Expand to Partial Signature

```topos
Behavior checkout:
  ensures:
    order processed via:
      [?cart_to_order
        given:
          cart (`Cart`)
          payment (`PaymentMethod`)
        returns: `Order` or `CheckoutError`
        requires:
          `cart.items` not empty
          `payment` is valid
        ensures:
          `result.total` = sum of `cart.items`
          `result.payment_status` is `captured`
          [?inventory_adjustment]]  # Nested hole for remaining unknown
```

### Step 6: Extract to Full Behavior

```topos
Behavior cart_to_order:
  Implements REQ-CHECKOUT-1.
  
  given:
    cart (`Cart`)
    payment (`PaymentMethod`)
    
  returns: `Order` or `CheckoutError`
  
  requires:
    `cart.items` not empty
    `payment` is valid
    
  ensures:
    `result.total` = sum of `cart.items`
    `result.payment_status` is `captured`
    inventory adjusted [?inventory_adjustment : `OrderItems` -> `InventoryDeltas`]
```

The hole has been refined to a full behavior, with a smaller hole remaining for the next iteration.

## Grammar

```ebnf
hole          := '[?' hole_body? ']'
hole_body     := hole_name? (':' hole_type)? hole_clause*
hole_name     := IDENTIFIER
hole_type     := type_expr ('->' type_expr)?
              |  '(' type_expr (',' type_expr)* ')' '->' type_expr
              |  type_expr '->' type_expr '|' type_expr
hole_clause   := 'where:' predicate
              |  'involving:' ref_list
              |  'given:' param+
              |  'returns:' type_expr
              |  'requires:' predicate
              |  'ensures:' predicate

ref_list      := REF (',' REF)*
```

## Best Practices

### Do

- Add type bounds as soon as you know them
- Name holes that are referenced multiple times
- Use `involving:` to document related concepts
- Refine holes incrementally as understanding grows
- Link holes to requirements when the feature is planned

### Don't

- Leave holes completely untyped if you know anything about them
- Create deeply nested holes (flatten or extract instead)
- Use holes for things that should be informal markers `[~]`
- Forget to revisit holes during refinement passes

### Holes vs Informal Markers

| Use Hole `[?]` | Use Informal `[~]` |
|----------------|-------------------|
| Structure unknown | Structure known, precision low |
| Need to refine later | Acceptable as-is for human reader |
| Want LSP tracking | Don't need tooling support |
| Has type constraints | Just prose approximation |

```topos
# Hole: We don't know how this works yet
ensures: payment processed [?payment_flow : `Payment` -> `Receipt`]

# Informal: We know roughly, precision isn't critical
ensures: user notified within reasonable time [~]
```
