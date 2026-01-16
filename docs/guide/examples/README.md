# Example Specifications

This directory contains example Topos specifications for common domains.

## Available Examples

| File | Domain | Description |
|------|--------|-------------|
| [auth.tps](auth.tps) | Authentication | User registration, login, sessions, password reset |
| [orders.tps](orders.tps) | E-commerce | Shopping cart, checkout, order management |

## Using These Examples

1. **Copy and adapt**: Use these as starting points for your own specs
2. **Learn patterns**: See how requirements flow to concepts to behaviors to tasks
3. **Test the tooling**: Run `topos check` and `topos trace` on these files

```bash
# Check an example
topos check docs/guide/examples/auth.tps

# View traceability
topos trace docs/guide/examples/auth.tps

# Generate context for a task
topos context TASK-AUTH-1 docs/guide/examples/auth.tps
```

## Key Patterns Demonstrated

### auth.tps
- Security-focused requirements with non-enumeration patterns
- Multiple acceptance criteria per requirement
- Error handling in behaviors
- Rate limiting and account locking

### orders.tps
- State machine patterns (order status)
- Aggregate concepts (Order with OrderItems)
- Idempotency considerations
- Multi-step workflows (cart → checkout → order)

## Creating Your Own

Start with the [getting started guide](../getting-started.md) and use the [language reference](../language-reference.md) as you build your spec.
