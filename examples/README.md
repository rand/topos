# Topos Examples

This directory contains example Topos specifications for testing and demonstration.

## Files

- `auth.tps` - Original authentication system spec (v1)
- `auth_v2.tps` - Modified authentication system spec (v2) with semantic changes

## Drift Detection Demo

Compare the two versions to see semantic drift detection in action:

```bash
# Structural comparison only (fast, no API key needed)
topos drift examples/auth.tps examples/auth_v2.tps --structural

# Semantic comparison (requires ANTHROPIC_API_KEY)
topos drift examples/auth.tps examples/auth_v2.tps
```

### What Changed Between v1 and v2?

| Element | v1 | v2 | Drift Type |
|---------|----|----|------------|
| REQ-1 | "must" authenticate with email/password | "should" authenticate with email/password or SSO | ConstraintWeakened, MeaningChanged |
| REQ-2 | 30 minute timeout, "invalidate session" | 15 minute timeout, "terminate session and log out" | ConstraintStrengthened, MeaningChanged |
| Session.expires_at | `[?]` (typed hole) | `Timestamp` (resolved) | Resolved |

### Expected Output

With semantic analysis enabled:

```
Drift Report (strategy: hybrid, semantic: available)
==================================================

## Structural Changes

Found 2 change(s):

## Requirements

  ~ REQ-1 (EARS 'when' clause changed)
  ~ REQ-2 (EARS 'when' clause changed)


## Semantic Analysis

- **REQ-1** (requirement): 70% aligned ~ minor drift
    - [high] ConstraintWeakened: Modal verb changed from 'must' to 'should'
    - [medium] MeaningChanged: Added SSO as alternative authentication method

- **REQ-2** (requirement): 70% aligned ~ minor drift
    - [medium] ConstraintStrengthened: Session timeout reduced from 30 to 15 minutes
    - [medium] MeaningChanged: Added explicit user logout behavior

Overall alignment: 70% (confidence: 92%)
```

## Typed Hole Suggestions Demo

The `auth.tps` file contains a typed hole in `Session.expires_at`. To get LLM suggestions:

```bash
# Run the suggest_hole example (requires ANTHROPIC_API_KEY)
cargo run --example suggest_hole -p topos-mcp
```

Expected suggestions:
1. `Timestamp` - Consistent with `created_at` field pattern
2. `Duration` - Alternative representation as time-to-live
