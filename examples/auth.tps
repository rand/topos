spec AuthSystem

import from "./types.tps": `UUID`, `Timestamp`

# Principles

- Security first
- User experience matters

# Requirements

## REQ-1: User Login
Users must be able to authenticate with email and password.

when: a user submits valid credentials
the system shall: authenticate the user and create a session

## REQ-2: Session Expiry
Sessions must expire after inactivity.

when: a session is inactive for 30 minutes
the system shall: invalidate the session

# Tasks

## TASK-1: Implement Login [REQ-1]
file: src/api/auth.rs
tests: tests/auth_test.rs
status: pending

## TASK-2: Add Session Middleware [REQ-2]
file: src/middleware/session.rs
tests: tests/session_test.rs
status: pending

Concept User:
  field id (`UUID`)
  field email (`String`)
  field password_hash (`String`)

Concept Session:
  field id (`UUID`)
  field user_id (`UUID`)
  field created_at (`Timestamp`)
  field expires_at ([?])

Concept Credential:
  field email (`String`)
  field password (`String`)
