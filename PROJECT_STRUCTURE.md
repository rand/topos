# Topos Project Structure

This document describes how to organize Topos specifications across multiple files for complex projects.

## Overview

Real-world projects require splitting specifications across multiple files for:

- **Separation of concerns**: Different domains in different files
- **Team ownership**: Teams own their specs independently
- **Scalability**: Thousands of requirements don't fit in one file
- **Reusability**: Shared types and principles across projects

Topos supports multi-file projects with explicit imports, qualified references, and workspace-wide analysis.

## Project Layout

### Recommended Structure

```
project/
├── topos.toml                 # Project configuration
├── specs/
│   ├── common/
│   │   ├── principles.tps     # Shared principles
│   │   ├── types.tps          # Shared type definitions
│   │   └── errors.tps         # Common error types
│   ├── domains/
│   │   ├── users/
│   │   │   ├── mod.tps        # Module index
│   │   │   ├── requirements.tps
│   │   │   ├── concepts.tps
│   │   │   └── tasks.tps
│   │   ├── orders/
│   │   │   ├── mod.tps
│   │   │   ├── requirements.tps
│   │   │   ├── concepts.tps
│   │   │   └── tasks.tps
│   │   └── payments/
│   │       └── ...
│   └── main.tps               # Root spec (optional)
└── src/                       # Implementation
```

### Alternative: Layer-Based

```
project/
├── topos.toml
├── specs/
│   ├── principles.tps         # All principles
│   ├── requirements/
│   │   ├── users.tps
│   │   ├── orders.tps
│   │   └── payments.tps
│   ├── design/
│   │   ├── architecture.tps
│   │   └── components.tps
│   ├── concepts/
│   │   ├── users.tps
│   │   ├── orders.tps
│   │   └── payments.tps
│   └── tasks/
│       ├── sprint-1.tps
│       ├── sprint-2.tps
│       └── backlog.tps
└── src/
```

### Alternative: Feature-Based

```
project/
├── topos.toml
├── specs/
│   ├── common.tps
│   ├── features/
│   │   ├── authentication.tps  # All layers for auth
│   │   ├── checkout.tps        # All layers for checkout
│   │   └── admin.tps           # All layers for admin
│   └── integrations/
│       ├── stripe.tps
│       └── sendgrid.tps
└── src/
```

## Project Configuration

### topos.toml

```toml
[project]
name = "ecommerce"
version = "1.0.0"

[specs]
root = "specs"                    # Spec files directory
include = ["**/*.tps"]            # Files to include
exclude = ["**/drafts/**"]        # Files to exclude

[principles]
# Principles files are inherited by all specs
inherit = ["specs/common/principles.tps"]

[requirements]
# Requirement ID prefixes by domain
prefixes = { users = "USR", orders = "ORD", payments = "PAY" }

[workspace]
# Related projects for cross-project references
dependencies = [
  { name = "shared-types", path = "../shared-types" }
]
```

## Import System

### Explicit Imports

Use `import` to bring definitions into scope:

```topos
spec UserManagement

import from "./common/types.tps":
  `Email`, `Identifier`, `DateTime`

import from "./common/errors.tps":
  `ValidationError`, `NotFoundError`

import from "../orders/concepts.tps":
  `Order`  # Reference Order from another domain


# Now use imported types
Concept User:
  field id (`Identifier`)
  field email (`Email`)
  field orders (`List` of `Order`)
```

### Import Syntax

```topos
# Import specific items
import from "path/to/file.tps":
  `Name1`, `Name2`, `Name3`

# Import with alias
import from "path/to/file.tps":
  `LongConceptName` as `Short`

# Import all exports
import from "path/to/file.tps": *

# Import module for qualified access
import "path/to/file.tps" as users
# Then use: `users.User`, `users.create_user`
```

### Relative vs Absolute Paths

```topos
# Relative to current file
import from "./sibling.tps": `Type`
import from "../parent/file.tps": `Type`

# Relative to project root (specs/)
import from "/common/types.tps": `Type`

# From dependency project
import from "shared-types/core.tps": `Type`
```

## Exports

### Explicit Exports

By default, all top-level definitions are exported. Use `private` to hide:

```topos
spec InternalModule

# Exported (default)
Concept User:
  field id (`Identifier`)

# Not exported
private Concept InternalHelper:
  field temp (`String`)

# Exported behavior
Behavior create_user:
  ...

# Not exported
private Behavior validate_internal:
  ...
```

### Module Index Files

Use `mod.tps` as a module's public interface:

```topos
# users/mod.tps
spec Users

# Re-export from subfiles
export from "./concepts.tps":
  `User`, `UserStatus`

export from "./requirements.tps":
  `REQ-USR-*`  # Glob pattern for requirements

export from "./tasks.tps":
  `TASK-USR-*`

# Don't export internal details
# (concepts.tps might have private helpers)
```

Consumers import from the module:

```topos
import from "./users/mod.tps":
  `User`, `create_user`

# Or import the module
import "./users/mod.tps" as users
```

## Cross-File References

### Qualified References

Reference definitions from other files without importing:

```topos
spec Orders

# Full path reference
Concept Order:
  field customer (`/users/concepts.User`)
  field payment (`/payments/concepts.Payment`)
```

### Reference Resolution Order

1. Local scope (current file)
2. Explicit imports
3. Inherited principles
4. Project-wide (if enabled in config)

### Requirement References

Requirements have project-wide unique IDs:

```topos
# In users/requirements.tps
## REQ-USR-001: User Registration

# In orders/requirements.tps  
## REQ-ORD-001: Order Creation

# Cross-reference in any file
Behavior create_order:
  Implements REQ-ORD-001.
  
  requires:
    user is registered  # See REQ-USR-001
```

### Task References

Tasks can depend on tasks in other files:

```topos
# In users/tasks.tps
## TASK-USR-001: Create User model [REQ-USR-001]

# In orders/tasks.tps
## TASK-ORD-001: Create Order model [REQ-ORD-001]
depends: TASK-USR-001  # Cross-file dependency
```

## Principle Inheritance

### Global Principles

Principles in inherited files apply to all specs:

```topos
# common/principles.tps
spec CommonPrinciples

# Principles

- Test-First: All implementation follows TDD
- Security: All inputs validated
- Accessibility: WCAG 2.1 AA compliance
```

```toml
# topos.toml
[principles]
inherit = ["specs/common/principles.tps"]
```

All specs in the project inherit these principles.

### Domain-Specific Principles

Domains can add additional principles:

```topos
# payments/principles.tps
spec PaymentPrinciples

# Inherits from common, adds:

# Principles

- PCI-Compliance: No card data on our servers
- Idempotency: All payment operations are idempotent
- Audit-Trail: All transactions logged
```

```topos
# payments/concepts.tps
spec PaymentConcepts

import principles from "./principles.tps"

# This file has both common + payment principles
```

## Namespacing

### Requirement ID Namespacing

Use prefixes to avoid ID collisions:

```toml
# topos.toml
[requirements]
prefixes = { 
  users = "USR",
  orders = "ORD", 
  payments = "PAY",
  admin = "ADM"
}
```

Generated IDs:
- `REQ-USR-001`, `REQ-USR-002` (users domain)
- `REQ-ORD-001`, `REQ-ORD-002` (orders domain)

### Task ID Namespacing

Similar pattern for tasks:

```
TASK-USR-001, TASK-ORD-001, TASK-PAY-001
```

### Concept Namespacing

Concepts are namespaced by file path:

```topos
# users/concepts.tps defines users.User
# orders/concepts.tps defines orders.Order

# Explicit qualification when ambiguous
field user (`users.User`)
field order (`orders.Order`)
```

## Workspace Analysis

### Cross-File Diagnostics

The LSP analyzes the entire workspace:

```
Workspace Analysis
═══════════════════════════════════════════════════════

Files: 24 spec files
Definitions: 45 concepts, 78 behaviors, 156 requirements

Cross-File Issues:

  ⚠ W201: Circular import detected
    users/concepts.tps → orders/concepts.tps → users/concepts.tps
    
  ⚠ W104: Task without requirement link
    payments/tasks.tps:45 - TASK-PAY-012
    
  ✗ E101: Undefined reference
    orders/concepts.tps:23 - `users.UserProfile` not found
    Did you mean `users.User`?

Traceability Summary:

  Domain    Requirements  With Behaviors  With Tasks  Coverage
  ────────────────────────────────────────────────────────────
  users     12            12 (100%)       10 (83%)    83%
  orders    18            15 (83%)        12 (67%)    67%
  payments  8             8 (100%)        4 (50%)     50%
  ────────────────────────────────────────────────────────────
  Total     38            35 (92%)        26 (68%)    68%
```

### Cross-File Navigation

LSP supports workspace-wide navigation:

- **Go to Definition**: Jumps across files
- **Find All References**: Searches entire workspace
- **Rename Symbol**: Updates all files
- **Workspace Symbols**: Search all definitions

## Splitting Strategies

### By Domain (Recommended for DDD)

Best when you have clear bounded contexts:

```
specs/
├── common/           # Shared across domains
├── identity/         # Auth, users, permissions
├── catalog/          # Products, categories
├── ordering/         # Orders, fulfillment
├── billing/          # Payments, invoices
└── shipping/         # Delivery, tracking
```

Each domain has:
- Own requirements (REQ-DOM-NNN)
- Own concepts and behaviors
- Own tasks
- Defined interfaces to other domains

### By Layer (Recommended for Phased Work)

Best when different roles work on different layers:

```
specs/
├── principles.tps     # Architects
├── requirements/     # Product managers
├── design/          # Architects  
├── concepts/        # Domain experts
└── tasks/           # Engineering leads
```

### By Feature (Recommended for Feature Teams)

Best when teams own features end-to-end:

```
specs/
├── common.tps
└── features/
    ├── user-onboarding.tps    # Team A
    ├── checkout-flow.tps      # Team B
    ├── subscription.tps       # Team C
    └── analytics.tps          # Team D
```

Each feature file contains all layers for that feature.

### By Lifecycle (Recommended for Long Projects)

Best for evolving projects:

```
specs/
├── stable/           # Shipped, stable specs
│   ├── v1/
│   └── v2/
├── current/          # Current release
└── planned/          # Future releases
    ├── q1-2025/
    └── q2-2025/
```

## Example: Multi-File Project

### common/types.tps

```topos
spec CommonTypes

Concept Identifier:
  A unique identifier (UUID v4).

Concept Email:
  A validated email address.

Concept Money:
  field amount (`Decimal`)
  field currency (`Currency`)

Concept Currency:
  one of: USD, EUR, GBP, JPY
```

### common/principles.tps

```topos
spec CorePrinciples

# Principles

- Test-First: All code has tests before implementation
- Security: All inputs validated, no SQL injection
- Accessibility: WCAG 2.1 AA for all UI
- Performance: API responses under 200ms p95
```

### users/requirements.tps

```topos
spec UserRequirements

## REQ-USR-001: User Registration

As a visitor, I want to register so I can access the platform.

when: user submits valid email and password
the system shall: create account with unverified status

acceptance:
  given: visitor on registration page
  when: enters valid credentials
  then: account created
  then: verification email sent


## REQ-USR-002: User Login

As a registered user, I want to log in to access my account.

when: user submits correct credentials
the system shall: create session and return token

acceptance:
  given: verified user account
  when: enters correct password
  then: JWT token returned
  then: session created
```

### users/concepts.tps

```topos
spec UserConcepts

import from "/common/types.tps":
  `Identifier`, `Email`, `DateTime`

Concept User:
  field id (`Identifier`): unique
  field email (`Email`): unique
  field password_hash (`Hash`)
  field status (`UserStatus`): default: `unverified`
  field created_at (`DateTime`)

Concept UserStatus:
  one of: unverified, active, suspended

Behavior register:
  Implements REQ-USR-001.
  
  given:
    email (`Email`)
    password (`String`)
  
  returns: `User` or `RegistrationError`
  
  ensures:
    `result.status` is `unverified`


Behavior login:
  Implements REQ-USR-002.
  
  given:
    email (`Email`)
    password (`String`)
  
  returns: `Session` or `AuthError`
```

### users/tasks.tps

```topos
spec UserTasks

# Tasks

## TASK-USR-001: Create User model [REQ-USR-001]
file: src/domains/users/models/user.ts
tests: src/domains/users/models/user.test.ts
status: done

## TASK-USR-002: Create AuthService [REQ-USR-001, REQ-USR-002]
file: src/domains/users/services/auth-service.ts
tests: src/domains/users/services/auth-service.test.ts
depends: TASK-USR-001
status: in-progress

## TASK-USR-003: Create auth endpoints [REQ-USR-001, REQ-USR-002]
file: src/domains/users/controllers/auth-controller.ts
depends: TASK-USR-002
status: pending
```

### orders/concepts.tps

```topos
spec OrderConcepts

import from "/common/types.tps":
  `Identifier`, `Money`, `DateTime`

import from "/users/concepts.tps":
  `User`

Concept Order:
  field id (`Identifier`): unique
  field customer (`User`)          # Cross-domain reference
  field items (`List` of `OrderItem`)
  field total (`Money`)
  field status (`OrderStatus`)

Concept OrderItem:
  field product_id (`Identifier`)
  field quantity (`Natural`)
  field unit_price (`Money`)

Concept OrderStatus:
  one of: pending, confirmed, shipped, delivered, cancelled

Behavior create_order:
  Implements REQ-ORD-001.
  
  given:
    customer (`User`)
    items (`List` of `OrderItem`)
  
  returns: `Order` or `OrderError`
  
  requires:
    `customer.status` is `active`  # Cross-domain constraint
    `items` is not empty
```

## CLI Commands

```bash
# Initialize project structure
topos init

# Validate all specs in workspace
topos check

# Show cross-file traceability
topos trace

# Generate traceability report
topos report --format html --output report.html

# Find all references to a definition
topos references "users.User"

# Show dependency graph
topos deps --format mermaid
```

## Best Practices

### Do

- Use consistent naming conventions across files
- Keep related specs in the same directory
- Use `mod.tps` to define public interfaces
- Namespace requirement and task IDs by domain
- Document cross-domain dependencies explicitly

### Don't

- Create circular imports between domains
- Duplicate definitions across files
- Use absolute paths when relative works
- Mix unrelated concepts in one file
- Ignore cross-file traceability warnings

### File Size Guidelines

| File Type | Recommended Max | Split When |
|-----------|-----------------|------------|
| Requirements | 20 requirements | > 30 requirements |
| Concepts | 10 concepts | > 15 concepts |
| Behaviors | 15 behaviors | > 20 behaviors |
| Tasks | 30 tasks | > 50 tasks |
| Single feature | 200 lines | > 300 lines |
