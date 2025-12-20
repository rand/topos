# Topos Language Specification

**Version**: 1.0.0  
**Status**: Draft

## Introduction

Topos is a specification language that serves as a **semantic contract** between humans and AI. It captures what matters about software in a structured, human-readable format designed for review and verification.

### The Checkpoint Role

Topos serves as a checkpoint in two directions:

**Forward flow** (intent → code):
```
Human intent → AI interprets → Spec → Human reviews → AI generates → Code
                                            ↑
                                     "Did you understand me?"
```

**Reverse flow** (code → understanding):
```
Code → AI summarizes → Spec → Human reviews → Understanding
                                    ↑
                             "Is this what it does?"
```

The spec enables **human verification of AI understanding**—not machine verification of code correctness.

### Design Implications

Because the goal is human verification:

- **Readable over formal**: Prose-like syntax that humans can review without training
- **Incomplete is okay**: Holes `[?]` and informal markers `[~]` for things not yet known
- **Navigable at scale**: Structure, references, and tooling for large specs
- **Versionable**: Plain text, diffable, reviewable in PRs
- **Optionally formal**: Path to verification exists but isn't required

## Design Philosophy

### Specs as Semantic Contracts

A Topos spec captures *what matters* about code:

- What requirements exist and their acceptance criteria
- What entities exist and their relationships  
- What operations are available and their contracts
- What must always be true

It does NOT require complete formal specification. It explicitly supports graduated precision—start informal, tighten as understanding grows.

### Deterministic Structure, Rich Prose

The parser never guesses about structure. Keywords at line start and indentation define the AST. Prose is interpreted within known structural positions, bounded and recoverable.

### Markdown Compatibility

**Topos IS Markdown.** This is not compatibility—it's identity. Every valid Topos file is also a valid CommonMark file.

The rule is simple: **Structured content lives in fenced blocks and colon-indented blocks that still parse as Markdown.**

This is a deliberate design choice for adoption:

1. **Renders everywhere**: GitHub, VS Code preview, any Markdown viewer
2. **Familiar syntax**: Headings, lists, code blocks work as expected
3. **PR-friendly**: Standard diff tools work without special support
4. **Gradual adoption**: Start with plain Markdown, add structure incrementally
5. **Zero tooling required**: Useful documentation even without Topos installed

**How it works**:
- Section markers (`# Requirements`, `# Concepts`) are standard Markdown headings
- Keywords (`Concept`, `Behavior`, etc.) are recognized at line start within sections
- Backtick references (`` `TypeName` ``) are standard inline code
- Foreign blocks (```` ```typespec ````) are standard fenced code blocks

**Graceful degradation**: Without Topos tooling, the spec renders as readable documentation. With tooling, it gains structure, navigation, and validation.

```markdown
<!-- This is valid Topos AND valid Markdown -->

# Requirements

## REQ-1: User Login

As a user, I want to log in so that I can access my account.

when: user enters valid credentials
the system shall: authenticate and redirect to dashboard

# Concepts

Concept User:
  field id (`Identifier`)
  field email (`Email`): unique
```

## Document Structure

A Topos document has five major sections, each optional but recommended:

```
spec Name

import from "path": `Type1`, `Type2`

# Principles
  Non-negotiable project guardrails

# Requirements  
  User stories and acceptance criteria

# Design
  Architecture and component structure

# Concepts / Behaviors / Invariants
  Type definitions and operations

# Tasks
  Implementation work units
```

## Multi-File Projects

Complex projects split specifications across multiple files. See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for detailed guidance.

### Import Syntax

```topos
# Import specific items
import from "./path/to/file.tps":
  `Concept1`, `Concept2`, `behavior_name`

# Import with alias
import from "./types.tps":
  `VeryLongName` as `Short`

# Import all exports
import from "./common.tps": *

# Import as namespace
import "./users.tps" as users
# Use as: `users.User`, `users.create_user`
```

### Export Control

```topos
# Public (default) - exported
Concept User:
  ...

# Private - not exported
private Concept InternalHelper:
  ...

# Re-export from submodules
export from "./concepts.tps":
  `User`, `UserStatus`
```

### Cross-File References

```topos
# Qualified reference (no import needed)
field customer (`/users/concepts.User`)

# After import
import from "./users.tps": `User`
field customer (`User`)

# Via namespace
import "./users.tps" as users
field customer (`users.User`)
```

## Lexical Structure

### Character Set

UTF-8 encoded text.

### Line Structure

Topos is line-oriented and indentation-sensitive:

- **Structural line**: Begins with keyword or heading
- **Continuation line**: Indented continuation of previous
- **Blank line**: Separates blocks
- **Comment line**: `//` prefix (reserved for future)

### Indentation

Spaces (recommended: 2 per level). Tabs converted to 4 spaces. Indentation determines scope.

### Keywords

**Top-level**:
```
spec  Concept  Behavior  Invariant  Unspecified
```

**Section markers**:
```
# Principles  # Requirements  # Design  # Tasks
```

**Block keywords**:
```
field  given  returns  requires  ensures  when  the system shall
acceptance  given  then  example  see also  implements
file  tests  depends  status
```

### Markers

```
[?]                              Hole (unnamed, untyped)
[? text]                         Hole with question
[? Type]                         Typed hole
[? Type -> Type]                 Typed hole with signature
[?name : Type -> Type]           Named typed hole
[? where: constraint]            Hole with constraints
[? involving: A, B, C]           Hole referencing concepts
[~]                              Informal marker
[REQ-N]                          Requirement reference
[TASK-N]                         Task reference
[→ location]                     Code anchor
```

See [TYPED_HOLES.md](TYPED_HOLES.md) for detailed typed hole documentation.

### References

Backtick-delimited entity references:

```
`Identifier`           Simple reference
`Order.status`         Field reference
`pending`              Enum variant
`REQ-1`                Requirement reference
```

### Headings

Markdown-style:
```
# Level 1
## Level 2
### Level 3
```

## Grammar

### Top-Level

```ebnf
spec          := 'spec' NAME import* section*
section       := heading content*
heading       := '#'+ TEXT
content       := principle | requirement | design_elem | concept | 
                 behavior | invariant | task | unspecified | prose

import        := 'import' import_source ':' import_list
              |  'import' STRING 'as' NAME
import_source := 'from' STRING
import_list   := '*' | import_item (',' import_item)*
import_item   := REF ('as' NAME)?

export        := 'export' 'from' STRING ':' export_list
export_list   := '*' | REF (',' REF)*

private_mod   := 'private'
```

### Imports and Exports

```topos
# Import syntax
import from "./path/file.tps":
  `Type1`, `Type2`, `behavior`

import from "./types.tps":
  `LongName` as `Short`

import from "./all.tps": *

import "./module.tps" as mod

# Export syntax  
export from "./submodule.tps":
  `Type1`, `Type2`

# Private modifier
private Concept Internal:
  ...
```

### Principles Section

```ebnf
principles    := '# Principles' principle*
principle     := '-' TEXT
```

Principles are non-negotiable guardrails:

```topos
# Principles

- Test-First: All implementation follows TDD
- Security: No secrets committed to repository
- Accessibility: All UI meets WCAG 2.1 AA
- Simplicity: Complexity requires documented justification
```

### Requirements Section

```ebnf
requirements  := '# Requirements' requirement*
requirement   := '##' REQ_ID ':' TEXT prose? ears_clause* acceptance?

ears_clause   := 'when:' TEXT 'the system shall:' TEXT
acceptance    := 'acceptance:' acc_clause+
acc_clause    := 'given:' TEXT | 'when:' TEXT | 'then:' TEXT
```

#### Stable ID Format

Requirement IDs should be **refactor-proof**—meaningful enough to survive reorganization:

| Format | Example | When to Use |
|--------|---------|-------------|
| `REQ-{N}` | `REQ-1` | Small projects, early exploration |
| `REQ-{DOMAIN}-{N}` | `REQ-AUTH-1` | Medium projects, clear domains |
| `REQ-{DOMAIN}-{FEATURE}-{N}` | `REQ-AUTH-SESSION-001` | Large projects, formal traceability |

**Recommendations:**
- Use semantic prefixes (`AUTH-`, `PAY-`, `SHIP-`) over bare numbers
- Zero-pad numbers for sorting (`001` not `1`)
- Never renumber—add new IDs instead
- Use `topos rename REQ-1 REQ-AUTH-1` to update all references

Requirements use EARS notation and BDD acceptance criteria:

```topos
# Requirements

## REQ-1: Task Creation

As a team member, I want to create tasks so that I can track my work.

when: user submits task form with valid title
the system shall: create task with status "todo"

when: user submits task form with empty title  
the system shall: display validation error

acceptance:
  given: authenticated user on task list page
  when: user enters "Fix bug" and clicks Create
  then: new task "Fix bug" appears with status "todo"


## REQ-2: Task Assignment

As a team lead, I want to assign tasks so that work is distributed.

when: user assigns task to team member
the system shall: update task assignee and notify member

acceptance:
  given: task exists with no assignee
  when: user selects team member from dropdown
  then: task shows assignee name
  then: assignee receives notification
```

### Design Section

```ebnf
design        := '# Design' design_elem*
design_elem   := '##' TEXT prose? component* dataflow?

component     := '-' '`' NAME '`' ':' TEXT
dataflow      := 'data flow:' TEXT
```

Design captures architecture and components:

```topos
# Design

## Components

- `TaskService`: Core business logic for task CRUD
- `TaskRepository`: Data persistence abstraction
- `TaskController`: HTTP API endpoints
- `NotificationService`: User notification delivery

## Data Flow

User Request → Controller → Service → Repository → Database
                              ↓
                    NotificationService → Email/Push

## Technology Decisions

- Framework: Express.js for API simplicity
- Database: PostgreSQL for relational integrity
- Queue: Redis for async notifications
```

### Concepts

```ebnf
concept       := 'Concept' NAME ':' prose? field*
field         := 'field' NAME type? (':' constraint*)?
type          := '(' type_expr ')'
type_expr     := REF | 'List' 'of' REF | 'Optional' REF | 
                 'one of:' variant (',' variant)*
constraint    := 'unique' | 'default:' expr | 'derived:' expr |
                 'invariant:' pred | 'at least' NUM | prose
```

```topos
Concept Task:
  A unit of work to be completed.
  
  field id (`Identifier`): unique
  field title (`String`): at least 1 character
  field description (`Optional` `String`)
  field status (`TaskStatus`): default: `todo`
  field assignee (`Optional` `User`)
  field created_at (`DateTime`)


Concept TaskStatus:
  one of: todo, in_progress, blocked, done
```

### Behaviors

```ebnf
behavior      := 'Behavior' NAME ':' implements? prose? behavior_body
implements    := 'Implements' REQ_REF (',' REQ_REF)* '.'
behavior_body := given? returns? requires? ensures? when_shall* example*

given         := 'given:' param+
param         := NAME type (':' constraint*)?
returns       := 'returns:' type_expr ('or' type_expr)?

requires      := 'requires:' pred+
ensures       := 'ensures:' pred+

when_shall    := 'when:' pred 'the system shall:' prose_or_hole
example       := 'example:' example_body
```

```topos
Behavior create_task:
  Implements REQ-1.
  
  Create a new task with the given title.
  
  given:
    title (`String`)
    creator (`User`)
    
  returns: `Task` or `ValidationError`
  
  when: `title` is not empty
  the system shall: create task with `status` = `todo`
  
  when: `title` is empty
  the system shall: return `ValidationError` with message "Title required"
  
  ensures:
    `result.creator` = `creator`
    `result.created_at` = now


Behavior assign_task:
  Implements REQ-2.
  
  given:
    task (`Task`)
    assignee (`User`)
    assigner (`User`)
    
  returns: `Task`
  
  requires:
    `task.status` ≠ `done`
    `assigner` has permission to assign
    
  ensures:
    `result.assignee` = `assignee`
    notification sent to `assignee` [~]
```

### Invariants

```ebnf
invariant     := 'Invariant' NAME ':' prose? quantifier? pred
quantifier    := 'for each' NAME 'in' REF ':'
```

```topos
Invariant tasks_have_creators:
  Every task has a creator.
  
  for each `task` in `Task`:
    `task.creator` exists


Invariant no_self_assignment:
  Users cannot assign tasks to themselves.
  
  for each `task` in `Task`:
    `task.assignee` ≠ `task.creator` or `task.assignee` is empty
```

### Aesthetic Blocks

Aesthetic blocks capture non-functional, subjective, or "vibe" requirements that are critical for AI-generated UI/UX but don't fit the boolean logic of traditional requirements.

```ebnf
aesthetic     := 'Aesthetic' NAME ':' aesthetic_field*
aesthetic_field := NAME ':' soft_constraint prose
soft_constraint := '[~]'?
```

```topos
Aesthetic AppTheme:
  palette: [~] "Warm earth tones with forest green accents"
  typography: [~] "Clean sans-serif, generous whitespace"
  motion: [~] "Snappy transitions, 200-300ms easing"
  feel: [~] "Professional but approachable"


Aesthetic CheckoutFlow:
  urgency: [~] "Clear but not pushy"
  trust: [~] "Security badges visible, clean forms"
  feedback: [~] "Immediate visual confirmation on actions"
  error_handling: [~] "Friendly, helpful error messages"
```

The `[~]` marker indicates a **soft constraint**—something the AI should aim for but that isn't formally verifiable. Aesthetics are:
- Included in context compilation for UI tasks
- Surfaced in LSP hover for related requirements
- Used by AI agents to maintain design consistency

### Soft Constraints `[~]`

The soft constraint marker `[~]` can appear anywhere a hard constraint could, indicating "approximately" or "aim for":

```topos
Behavior search_products:
  ensures:
    results returned in [~] "under 200ms"
    results ordered by [~] "relevance"


Concept Dashboard:
  field widgets (`List` of `Widget`): [~] "3-5 widgets recommended"
```

Soft constraints differ from typed holes `[?]`:

| Marker | Meaning | Tooling Support |
|--------|---------|-----------------|
| `[?]` | Unknown—needs refinement | LSP tracking, navigation |
| `[~]` | Approximate—good enough for humans | Context compilation |

#### Soft Constraint Guardrails

To prevent `[~]` from becoming an excuse-shaped blob, the tooling enforces guardrails:

**1. Soft-to-Hard Ratio Lint**

```bash
topos check --warn-soft-ratio=0.3
# Warning if >30% of constraints are soft
```

**2. Hardening Task Association**

Soft constraints that need eventual precision should link to a hardening task:

```topos
Behavior process_payment:
  ensures:
    response time [~] "under 500ms" [TASK-PERF-1]  # Links to hardening task
```

**3. Permanent Soft Marker**

For constraints that are intentionally soft forever (aesthetics, human judgment):

```topos
Aesthetic BrandVoice:
  tone: [~permanent] "Warm but professional"  # Explicitly never hardened
```

The `[~permanent]` variant tells tooling not to warn about this soft constraint.

**4. Soft Constraint Report**

```bash
topos trace --soft-constraints
# Lists all [~] markers with their hardening status
```

### Foreign Blocks (TypeSpec, CUE)

Topos embeds best-in-class specification languages for domains where they excel. Foreign blocks are fenced code blocks with special language identifiers.

```ebnf
foreign_block := '```' LANGUAGE NEWLINE content '```'
LANGUAGE      := 'typespec' | 'cue' | 'protobuf' | 'jsonschema'
```

#### TypeSpec for API Schemas

```topos
# API Types

The User API follows REST conventions with JSON payloads.

```typespec
import "@typespec/http";
import "@typespec/rest";

@route("/users")
namespace Users {
  model User {
    id: string;
    @minLength(1) name: string;
    email: string;
    createdAt: utcDateTime;
  }
  
  @get op list(): User[];
  @post op create(@body user: User): User;
  @get op read(@path id: string): User | NotFoundError;
}
```
```

#### CUE for Validation Rules

```topos
# Validation Constraints

Order validation rules enforced at the domain layer.

```cue
#Order: {
  id: string & =~"^ORD-[A-Z0-9]{8}$"
  
  items: [...#OrderItem] & len(items) > 0
  
  total: number & >0
  total: items.reduce(0, sum)
  
  status: "pending" | "paid" | "shipped" | "delivered" | "cancelled"
  
  // Shipped orders must have tracking
  if status == "shipped" {
    tracking: string & =~"^[A-Z0-9]{12,20}$"
  }
}

#OrderItem: {
  productId: string
  quantity: int & >0 & <=100
  price: number & >0
}
```
```

#### Foreign Block Semantics

1. **Parsing**: Tree-sitter recognizes foreign blocks and preserves content verbatim
2. **Validation**: If the foreign language toolchain is available, Topos validates the embedded spec
3. **References**: Topos concepts can reference foreign definitions: `field order (`cue:#Order`)`
4. **Context**: Foreign blocks are included in context compilation when relevant

### Tasks Section

```ebnf
tasks         := '# Tasks' task*
task          := '##' TASK_ID ':' TEXT req_refs? task_body

req_refs      := '[' REQ_REF (',' REQ_REF)* ']'
task_body     := prose? task_field*
task_field    := 'file:' PATH | 'tests:' PATH | 
                 'depends:' TASK_REF (',' TASK_REF)* |
                 'evidence:' evidence_block |
                 'context:' context_block |
                 'status:' STATUS

evidence_block := NEWLINE INDENT evidence_field+ DEDENT
evidence_field := 'pr:' URL | 'commit:' HASH | 'coverage:' PERCENT |
                  'benchmark:' TEXT | 'review:' URL | 'diff:' URL

context_block  := NEWLINE INDENT context_field+ DEDENT  
context_field  := 'include:' ref_list | 'exclude:' ref_list | 'notes:' TEXT
```

Tasks are discrete implementation units with **evidence-based traceability**:

```topos
# Tasks

## TASK-1: Create Task model [REQ-1]

Implement the Task domain model with validation logic.

file: src/models/task.ts
tests: src/models/task.test.ts
evidence:
  pr: https://github.com/org/repo/pull/42
  commit: a1b2c3d
  coverage: 94%
  benchmark: p99 < 5ms
status: done


## TASK-2: Create TaskService [REQ-1, REQ-2]

Implement service layer with create and assign methods.

file: src/services/task-service.ts
tests: src/services/task-service.test.ts
depends: TASK-1
evidence:
  pr: https://github.com/org/repo/pull/57
  review: https://github.com/org/repo/pull/57#pullrequestreview-123
status: done


## TASK-3: Create TaskController [REQ-1]

Implement REST endpoints for task creation.

file: src/controllers/task-controller.ts
tests: src/controllers/task-controller.test.ts
depends: TASK-2
status: pending


## TASK-4: Add notification on assign [REQ-2]

Send notification when task is assigned.

file: src/services/notification-service.ts
depends: TASK-2
context:
  include: `NotificationService`, `EmailTemplate`
  notes: |
    Requires SendGrid API key in environment.
    See docs/notifications.md for setup.
status: pending
```

### Predicates

Predicates are recognized within clause contexts:

```ebnf
pred          := comparison | membership | quantified | 
                 boolean_op | state_change | prose_pred

comparison    := expr COMP_OP expr
membership    := expr 'in' '[' expr_list ']'
quantified    := ('for each' | 'some') NAME 'in' REF ':' pred
boolean_op    := pred ('and' | 'or') pred | 'not' pred
state_change  := expr 'becomes' expr | 'all other fields unchanged'

COMP_OP       := '=' | '≠' | '!=' | '>' | '<' | '≥' | '≤' | 'is' | 'is not'
```

### Typed Holes

Holes support gradual refinement through partial type information:

```ebnf
hole          := '[?' hole_body? ']'
hole_body     := hole_name? (':' hole_type)? hole_clause*
hole_name     := IDENTIFIER
hole_type     := simple_hole_type | function_hole_type
simple_hole_type := type_expr
function_hole_type := type_expr '->' type_expr ('|' type_expr)?
                   |  '(' type_expr (',' type_expr)* ')' '->' type_expr

hole_clause   := 'where:' predicate
              |  'involving:' ref_list
              |  'given:' param+
              |  'returns:' type_expr
              |  'requires:' predicate
              |  'ensures:' predicate

ref_list      := REF (',' REF)*
```

**Examples:**

```topos
# Untyped hole
[?]

# Typed hole with signature
[? `PaymentMethod` -> `PaymentResult`]

# Named typed hole
[?payment_flow : `PaymentMethod` -> `PaymentResult`]

# Hole with constraints
[? `Order` -> `ShippingLabel`
   where: `order.status` is `paid`]

# Hole referencing related concepts
[? involving: `Payment`, `Refund`, `Order`]

# Hole with partial signature
[?queue_processor
  given:
    message (`QueueMessage`)
  returns: `Acknowledgment` or `ProcessingError`
  requires:
    `message` is valid
  ensures:
    [?]]  # Postconditions still unknown
```

Typed holes enable:
- **Navigation**: Ctrl+Click on types within holes
- **Tracking**: Named holes are tracked across the spec
- **Refinement**: Progressively add information as understanding grows
- **Validation**: Refinements must be compatible with declared types

See [TYPED_HOLES.md](TYPED_HOLES.md) for detailed documentation.

### EARS Patterns

EARS (Easy Approach to Requirements Syntax) clauses:

| Pattern | Template | Example |
|---------|----------|---------|
| Ubiquitous | The system shall [behavior] | The system shall log all errors |
| Event-driven | When [event], the system shall [behavior] | When user clicks Save, the system shall persist data |
| State-driven | While [state], the system shall [behavior] | While offline, the system shall queue requests |
| Optional | Where [condition], the system shall [behavior] | Where user is admin, the system shall show controls |
| Unwanted | If [condition], the system shall [behavior] | If file exceeds limit, the system shall reject upload |

In Topos, these are expressed as:

```topos
when: [trigger/condition]
the system shall: [required behavior]
```

### BDD Acceptance Criteria

```topos
acceptance:
  given: [precondition - starting state]
  when: [action - what the user does]
  then: [outcome - observable result]
  then: [additional outcome]
```

Multiple `then` clauses are conjunctive (all must hold).

## Reference Resolution

References resolve through layered scopes:

1. **Contextual**: `result`, `old(x)`, parameters
2. **Local**: Same behavior/concept
3. **Section**: Same heading scope
4. **Spec**: All definitions
5. **Builtins**: `List`, `String`, `Boolean`, etc.

Requirement references (`REQ-1`) resolve to Requirements section.
Task references (`TASK-1`) resolve to Tasks section.

## Traceability

### Forward Traceability

Requirements → Behaviors → Tasks → Code

```
REQ-1: Task Creation
  ↓ Implements
Behavior create_task
  ↓ Links to
TASK-1, TASK-2, TASK-3
  ↓ Produces
src/models/task.ts, src/services/task-service.ts
```

### Backward Traceability

Code → Tasks → Behaviors → Requirements

Every **externally observable behavior** and its **tests** should trace back to a requirement.

**Note**: Internal implementation details (utility functions, glue code, infrastructure) do not require direct requirement links. Traceability is at boundaries—public APIs, domain operations, user-visible features—not every line.

### Coverage Analysis

The tooling shall report:

- Requirements without implementing behaviors
- Requirements without linked tasks  
- Tasks without requirement links
- Behaviors without requirement links
- **Orphan thresholds**: Configurable lint for acceptable "untraced" internal code

## File Format

**Extension**: `.tps` or `.topos`

**MIME Type**: `text/topos`

**Encoding**: UTF-8

## Complete Example

```topos
spec TaskManagement

# Principles

- Test-First: All code has tests before implementation
- Simplicity: No abstractions without justification
- Accessibility: WCAG 2.1 AA compliance for all UI


# Requirements

## REQ-1: Task Creation

As a team member, I want to create tasks so I can track work.

when: user submits valid task form
the system shall: create task with status "todo"

when: user submits empty title
the system shall: show validation error

acceptance:
  given: authenticated user
  when: user creates task "Fix bug"
  then: task appears in list with status "todo"


## REQ-2: Task Completion

As a team member, I want to complete tasks so I can show progress.

when: user marks task as done
the system shall: update status to "done" and record completion time

acceptance:
  given: task with status "in_progress"
  when: user clicks "Mark Done"
  then: task status becomes "done"
  then: completion timestamp is recorded


# Design

## Components

- `Task`: Domain model with validation
- `TaskService`: Business logic
- `TaskRepository`: Persistence
- `TaskController`: HTTP API


# Concepts

Concept Task:
  field id (`Identifier`): unique
  field title (`String`): at least 1 character
  field status (`TaskStatus`): default: `todo`
  field completed_at (`Optional` `DateTime`)

Concept TaskStatus:
  one of: todo, in_progress, done


# Behaviors

Behavior create_task:
  Implements REQ-1.
  
  given:
    title (`String`)
    
  returns: `Task` or `ValidationError`
  
  when: `title` is not empty
  the system shall: create task with `status` = `todo`


Behavior complete_task:
  Implements REQ-2.
  
  given:
    task (`Task`)
    
  returns: `Task`
  
  requires:
    `task.status` ≠ `done`
    
  ensures:
    `result.status` is `done`
    `result.completed_at` = now


# Invariants

Invariant completed_tasks_have_timestamp:
  for each `task` in `Task`:
    `task.status` is `done` → `task.completed_at` exists


# Tasks

## TASK-1: Implement Task model [REQ-1, REQ-2]
file: src/models/task.ts
tests: src/models/task.test.ts
status: pending

## TASK-2: Implement TaskService [REQ-1, REQ-2]
file: src/services/task-service.ts
tests: src/services/task-service.test.ts
depends: TASK-1
status: pending

## TASK-3: Implement API endpoints [REQ-1, REQ-2]
file: src/controllers/task-controller.ts
tests: src/controllers/task-controller.test.ts
depends: TASK-2
status: pending
```

## Conformance

A conforming implementation must:

1. Parse all valid Topos documents
2. Produce AST with accurate source spans
3. Resolve references per scoping rules
4. Report undefined references as errors
5. Track requirement-behavior-task traceability
6. Provide LSP capabilities as specified

A conforming implementation may:

- Provide additional diagnostic rules
- Support domain-specific extensions
- Provide additional predicate patterns
