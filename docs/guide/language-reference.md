# Topos Language Reference

Quick reference for Topos syntax. For complete details, see [LANGUAGE_SPEC.md](../../LANGUAGE_SPEC.md).

## File Structure

```topos
spec SpecName              # Required: specification name

# Principles                # Optional: guiding principles

# Requirements              # Optional: user requirements

# Concepts                  # Optional: domain models

# Behaviors                 # Optional: system behaviors

# Tasks                     # Optional: implementation tasks
```

## Spec Declaration

Every file starts with a spec name:

```topos
spec OrderManagement
```

## Principles

Guiding principles for the project:

```topos
# Principles

- Simplicity: Prefer simple solutions over complex ones
- Test-First: Write tests before implementation
- Security: Never store plaintext passwords
```

## Requirements

User-facing requirements with structured clauses:

```topos
# Requirements

## REQ-1: Descriptive Title

As a [role], I want [feature] so that [benefit].

when: [trigger condition]
the system shall: [expected behavior]

acceptance:
  given: [precondition]
  when: [action]
  then: [expected result]
```

### Multiple Acceptance Criteria

```topos
## REQ-2: Order Validation

acceptance:
  given: cart has items
  when: user proceeds to checkout
  then: order total is calculated

acceptance:
  given: cart is empty
  when: user proceeds to checkout
  then: error message is shown
```

## Concepts

Domain models with typed fields:

```topos
# Concepts

Concept User:
  field id (`UUID`): unique
  field email (`Email`): unique, required
  field name (`String`): at least 1 character
  field status (`UserStatus`): default: `active`
  field created_at (`DateTime`)
```

### Field Constraints

| Constraint | Example |
|------------|---------|
| `unique` | `field id: unique` |
| `required` | `field name: required` |
| `optional` | `field nickname: optional` |
| `default: X` | `field status: default: \`active\`` |
| `at least N` | `field name: at least 1 character` |
| `at most N` | `field items: at most 100` |

### Relations

```topos
Concept Order:
  field user (`User`): required          # Reference to User
  field items (`List<OrderItem>`)        # Collection
```

## Behaviors

System behaviors with signatures and constraints:

```topos
# Behaviors

Behavior create_order:
  input: `Cart`, `User`
  output: `Order`

  requires:
    cart is not empty
    user is authenticated

  ensures:
    order.status == `pending`
    order.user == user

  errors:
    EmptyCartError: when cart has no items
    UnauthorizedError: when user is not authenticated
```

### Behavior Clauses

| Clause | Purpose |
|--------|---------|
| `input:` | Input types |
| `output:` | Return type |
| `requires:` | Preconditions |
| `ensures:` | Postconditions |
| `errors:` | Possible errors |
| `invariant:` | Maintained conditions |

## Tasks

Implementation tracking with evidence:

```topos
# Tasks

## TASK-1: Implement User model [REQ-1]

Description of the task.

file: src/models/user.rs
tests: src/models/user_test.rs
evidence:
  pr: https://github.com/org/repo/pull/123
  commit: abc123f
  coverage: 94%
status: done
```

### Task Status

| Status | Meaning |
|--------|---------|
| `pending` | Not started |
| `in-progress` | Being worked on |
| `done` | Complete with evidence |
| `blocked` | Waiting on dependency |

### Linking Tasks to Requirements

Use `[REQ-X]` in the task title:

```topos
## TASK-1: Implement login flow [REQ-1, REQ-2]
```

## Typed Holes `[?]`

Mark unknowns explicitly:

```topos
[?]                                    # Unknown value
[? `Type`]                             # Unknown with type hint
[? `Input` -> `Output`]                # Unknown function signature
[?name : `Type`]                       # Named hole for tracking
[? involving: `Concept1`, `Concept2`]  # Related concepts
```

### Examples

```topos
Concept Payment:
  field method [?]                     # Type TBD
  field processor [? `PaymentGateway`] # Known type, value TBD

Behavior process_payment:
  input: `Payment`
  output: [? `Payment` -> `Receipt`]   # Signature TBD
```

## Soft Constraints `[~]`

For subjective or approximate requirements:

```topos
Aesthetic AppStyle:
  palette: [~] "Warm earth tones"
  motion: [~] "Snappy transitions"
  feel: [~] "Professional but approachable"

Behavior animate_login:
  ensures:
    duration [~] "under 300ms"
    feel [~] "smooth and responsive"
```

## Foreign Blocks

Embed TypeSpec or CUE for specialized syntax:

~~~topos
# API Types

```typespec
model User {
  id: string;
  @minLength(1) name: string;
}
```

# Validation Rules

```cue
#Order: {
  total: number & >0
  items: [...#Item] & len(items) > 0
}
```
~~~

## Comments

Standard Markdown syntax:

```topos
# This is a section header

<!-- This is an HTML comment, ignored by parser -->

// Line comments are NOT supported
/* Block comments are NOT supported */
```

## Type Syntax

Types are wrapped in backticks:

```topos
field name (`String`)
field items (`List<Item>`)
field status (`OrderStatus`)
input: `User`, `Cart`
output: `Result<Order, Error>`
```

## Identifiers

| Type | Pattern | Examples |
|------|---------|----------|
| Spec name | PascalCase | `OrderManagement` |
| Requirement ID | `REQ-` + number/name | `REQ-1`, `REQ-AUTH-1` |
| Task ID | `TASK-` + number/name | `TASK-1`, `TASK-AUTH-1` |
| Concept name | PascalCase | `User`, `OrderItem` |
| Behavior name | snake_case | `create_order`, `validate_user` |
| Field name | snake_case | `user_id`, `created_at` |

## Best Practices

1. **One spec per bounded context**: Don't mix unrelated domains
2. **Link everything**: Every task should reference requirements
3. **Use typed holes**: Better to mark unknowns than guess
4. **Evidence matters**: Tasks aren't done without proof
5. **Keep it readable**: Topos is for humans first
