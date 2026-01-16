spec Authentication

# Principles

- Security: Never store plaintext passwords
- Privacy: Minimize data collection
- Simplicity: Standard OAuth2/OIDC patterns

# Requirements

## REQ-AUTH-1: User Registration

As a visitor, I want to create an account so I can access the application.

when: visitor submits registration form with valid email and password
the system shall: create a new user account and send verification email

acceptance:
  given: email is not already registered
  when: visitor submits valid email and strong password
  then: account is created with status "pending_verification"

acceptance:
  given: email is already registered
  when: visitor attempts to register
  then: generic error message shown (no email enumeration)

## REQ-AUTH-2: User Login

As a user, I want to log in so I can access my account.

when: user submits valid credentials
the system shall: create an authenticated session

acceptance:
  given: user has verified account
  when: user enters correct credentials
  then: session is created and user is redirected to dashboard

acceptance:
  given: user enters wrong password 5 times
  when: user attempts 6th login
  then: account is temporarily locked for 15 minutes

## REQ-AUTH-3: Session Management

As a user, I want my session to persist so I don't have to log in repeatedly.

when: user has valid session
the system shall: allow access to protected resources

acceptance:
  given: session token is valid and not expired
  when: user accesses protected resource
  then: access is granted

acceptance:
  given: session token is expired
  when: user accesses protected resource
  then: user is redirected to login

## REQ-AUTH-4: Password Reset

As a user, I want to reset my password if I forget it.

when: user requests password reset
the system shall: send reset link to registered email

acceptance:
  given: email exists in system
  when: user requests password reset
  then: reset email is sent (same response for non-existent emails)

# Concepts

Concept User:
  field id (`UUID`): unique
  field email (`Email`): unique, required
  field password_hash (`String`): required
  field status (`UserStatus`): default: `pending_verification`
  field created_at (`DateTime`)
  field updated_at (`DateTime`)
  field failed_login_attempts (`Int`): default: `0`
  field locked_until (`DateTime`): optional

Concept Session:
  field id (`UUID`): unique
  field user_id (`UUID`): required
  field token (`String`): unique
  field created_at (`DateTime`)
  field expires_at (`DateTime`)
  field ip_address (`String`): optional
  field user_agent (`String`): optional

Concept PasswordResetToken:
  field id (`UUID`): unique
  field user_id (`UUID`): required
  field token (`String`): unique
  field expires_at (`DateTime`)
  field used (`Boolean`): default: `false`

# Behaviors

Behavior register_user:
  input: `Email`, `Password`
  output: `Result<User, RegistrationError>`

  requires:
    email is valid format
    password meets strength requirements

  ensures:
    user.status == `pending_verification`
    password is hashed with bcrypt
    verification email is queued

  errors:
    EmailTakenError: when email already exists
    WeakPasswordError: when password doesn't meet requirements

Behavior login:
  input: `Email`, `Password`
  output: `Result<Session, LoginError>`

  requires:
    user exists and is not locked

  ensures:
    session.expires_at is 24 hours from now
    failed_login_attempts reset on success

  errors:
    InvalidCredentialsError: when email/password don't match
    AccountLockedError: when too many failed attempts
    UnverifiedAccountError: when email not verified

Behavior validate_session:
  input: `SessionToken`
  output: `Result<User, SessionError>`

  requires:
    token exists in database

  ensures:
    session is not expired

  errors:
    InvalidSessionError: when token not found
    ExpiredSessionError: when session expired

Behavior logout:
  input: `SessionToken`
  output: `Result<(), LogoutError>`

  ensures:
    session is deleted from database

Behavior request_password_reset:
  input: `Email`
  output: `Result<(), ()>`

  ensures:
    reset token created if user exists
    email sent if user exists
    same response regardless of user existence (security)

# Tasks

## TASK-AUTH-1: Implement User model [REQ-AUTH-1]

Create User entity with password hashing.

file: src/models/user.rs
tests: src/models/user_test.rs
status: pending

## TASK-AUTH-2: Implement Session model [REQ-AUTH-2, REQ-AUTH-3]

Create Session entity with token generation.

file: src/models/session.rs
tests: src/models/session_test.rs
status: pending

## TASK-AUTH-3: Implement registration endpoint [REQ-AUTH-1]

POST /api/auth/register endpoint.

file: src/api/auth.rs
tests: src/api/auth_test.rs
status: pending

## TASK-AUTH-4: Implement login endpoint [REQ-AUTH-2]

POST /api/auth/login endpoint with rate limiting.

file: src/api/auth.rs
tests: src/api/auth_test.rs
status: pending

## TASK-AUTH-5: Implement session validation middleware [REQ-AUTH-3]

Auth middleware for protected routes.

file: src/middleware/auth.rs
tests: src/middleware/auth_test.rs
status: pending

## TASK-AUTH-6: Implement password reset flow [REQ-AUTH-4]

Password reset request and confirmation endpoints.

file: src/api/auth.rs
tests: src/api/auth_test.rs
status: pending
