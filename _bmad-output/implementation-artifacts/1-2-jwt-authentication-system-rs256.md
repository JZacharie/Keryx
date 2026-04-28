# Story 1.2: JWT Authentication System (RS256)

Status: review

## Story

As a User/Service,
I want to authenticate via JWT (RS256) with public key verification,
so that only authorized requests can access the orchestrator APIs.

## Acceptance Criteria

1. **RS256 Support**: The orchestrator can verify JWT tokens signed with the RS256 algorithm. [Source: epics.md]
2. **Public Key Loading**: The public key used for verification is loaded from an environment variable `JWT_PUBLIC_KEY` or a file. [Source: architecture.md]
3. **Auth Middleware**: An Axum middleware is implemented to intercept requests and validate the `Authorization: Bearer <token>` header.
4. **Error Handling**: Requests with missing, invalid, or expired tokens return a `401 Unauthorized` response in JSON format.
5. **Claims Extraction**: The middleware extracts user identity (e.g., `sub`) and makes it available to handlers via Axum's `Extension` or `Request` state.

## Tasks / Subtasks

- [x] Setup JWT Dependencies (AC: 1)
  - [x] Add `jsonwebtoken` and `base64` to `orchestrator/Cargo.toml`.
- [x] Implement Token Verification Logic (AC: 1, 2)
  - [x] Create `src/infrastructure/auth/` directory.
  - [x] Implement a `JwtVerifier` that loads the public key and validates tokens.
- [x] Create Axum Auth Middleware (AC: 3, 5)
  - [x] Implement a `Layer` or `from_fn` middleware in `src/interfaces/http/middleware/auth.rs`.
  - [x] Handle bearer token extraction from headers.
- [x] Secure Health Check (Optional/Test) (AC: 4)
  - [x] Apply the middleware to a new test route `/api/secure-ping`.
- [x] Standardize Error Responses (AC: 4)
  - [x] Ensure the 401 response follows the project's error format.

## Dev Notes

- **Library**: Use the `jsonwebtoken` crate which is the standard in Rust for this.
- **Algorithm**: RS256 only.
- **Key Format**: PEM format for the public key.
- **Environment**: For local testing, use a mock PEM public key.

### Project Structure Notes

- New folder: `orchestrator/src/infrastructure/auth/`
- New folder: `orchestrator/src/interfaces/http/middleware/`

### References

- [Architecture: Security Patterns](file:///home/joseph/git/Keryx/_bmad-output/planning-artifacts/architecture.md#L80)
- [Project Context: Error Handling](file:///home/joseph/git/Keryx/_bmad-output/project-context.md#L48)

## Dev Agent Record

### Agent Model Used

Antigravity (Gemini 2.0)

### Debug Log References

- [Cargo Check Output](file:///home/joseph/git/Keryx/services/orchestrator/target/debug/.fingerprint/...)

### Completion Notes List

- Added `jsonwebtoken` and `base64` dependencies.
- Implemented `JwtVerifier` for RS256.
- Created `auth_middleware` with bearer token extraction.
- Secured `/api/*` routes.
- Created `test_public_key.pem` for dev context.

### File List

- `services/orchestrator/Cargo.toml`
- `services/orchestrator/src/infrastructure/auth/mod.rs`
- `services/orchestrator/src/infrastructure/auth/verifier.rs`
- `services/orchestrator/src/infrastructure/mod.rs`
- `services/orchestrator/src/interfaces/http/middleware/auth.rs`
- `services/orchestrator/src/interfaces/http/middleware/mod.rs`
- `services/orchestrator/src/interfaces/http/mod.rs`
- `services/orchestrator/src/main.rs`
- `services/orchestrator/test_public_key.pem`
