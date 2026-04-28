---
project_name: 'Keryx'
user_name: 'autobot'
date: '2026-04-28'
sections_completed: ['technology_stack', 'language_rules', 'framework_rules', 'testing_rules', 'quality_rules', 'workflow_rules', 'anti_patterns']
status: 'complete'
rule_count: 22
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

### Rust (Services/Orchestrator, Keryx-Core)
- **Framework**: Axum 0.7
- **Async Runtime**: Tokio 1.0 (full features)
- **Serialization**: Serde 1.0, Serde_JSON 1.0, TOML 0.8
- **HTTP Client**: Reqwest 0.12 (rustls-tls)
- **Database/Cache**: Redis 0.25 (tokio-comp, json), AWS SDK S3 1.1
- **Observability**: Tracing 0.1, OpenTelemetry 0.24, OTLP 0.17
- **Kubernetes**: Kube 0.96, k8s-openapi 0.23 (v1.30)
- **Error Handling**: Anyhow 1.0, Thiserror 2.0
- **Dependency Locking**: Mandatory `Cargo.lock` and `requirements.txt` (with hashes) pinning in CI/CD.

### Python (AI/Processing Services)
- **Framework**: FastAPI 0.136.1
- **Async Server**: Uvicorn 0.30.6
- **Validation**: Pydantic 2.13.3
- **S3 Client**: aioboto3 13.4.0
- **AI Models**: openai-whisper 20240930
- **Translation**: deep-translator 1.11.4

### Infrastructure
- **Containerization**: Docker, Docker Compose
- **Orchestration**: Kubernetes v1.30
- **Observability Backend**: OpenObserve (via OTLP)

## Critical Implementation Rules

### Language-Specific Rules

#### Rust
- **Error Handling**: Use `anyhow` for application-level logic and `thiserror` for domain/infrastructure errors. Never use `panic!`.
- **Async Patterns**: Prefer `tokio` primitives. All repository traits must use `#[async_trait]`.
- **Hexagonal Integrity**: Core domain logic must remain pure Rust; infrastructure details (DB, S3) must be behind traits (ports).

#### Python
- **Type Safety**: Use Pydantic v2 models for all API requests and responses.
- **Async/IO**: Use `async/await` for all I/O operations (HTTP calls via `httpx`, S3 via `aioboto3`).
- **Error Handling**: Mandate `try/except` blocks for all AI model inferences (Whisper/Translation) to handle transient hardware or model errors.
- **Resource Management**: Large models (Whisper) must be loaded once via FastAPI `lifespan`. Implement a failure fallback or health check during loading (e.g., return 503 if GPU OOM).

### Framework-Specific Rules

#### Axum (Rust)
- **State Management**: Always use `State(state): State<AppState>` to access use cases. Use `Arc` for shared state to avoid unnecessary cloning.
- **Routing**: Keep `main.rs` as the routing orchestrator. Use `.route()` for logic and `.nest_service()` for static files.
- **Middleware**: Leverage `tower-http` layers for cross-cutting concerns (logging, CORS, compression).

#### FastAPI (Python)
- **Schema Validation**: Define all input/output schemas as Pydantic models in the service directory. Ensure error formats are standardized with the Rust services.
- **Model Synchronization**: Shared Pydantic/Serde models between services must be kept in sync via a shared repository or automated synchronization check in CI.
- **Startup/Shutdown**: Use the `lifespan` context manager for resource initialization (e.g., loading ML models).
- **Background Tasks**: Always use `try/except` and explicit logging inside `BackgroundTasks` to prevent silent failures.

### Testing Rules

#### General
- **Error Response Testing**: Every endpoint must have at least one test case for successful response (200/201) and one for error handling (400/404/500).
- **Contractual Integrity**: All cross-service communication (Rust ↔ Python) must have schema validation tests using shared Pydantic/Serde models.
- **Robustness Testing**: Critical data processing functions (audio slicing, translation) should implement property-based testing (e.g., `proptest` in Rust, `hypothesis` in Python). Set `max_examples=100` for CI runs to prevent timeouts.
- **Performance Gates**: AI processing endpoints must include baseline benchmarks. Maximum acceptable degradation per PR is 10%. Execute these in CI/CD environments that mirror production constraints.
- **Observability & Logging**: Implement structured logging (JSON) across all services. Trace IDs must be propagated through the `X-Trace-Id` header for cross-service debugging.
- **Scenario-Driven Integration**: Critical user journeys must be covered by end-to-end integration tests that span multiple units/modules.
- **Self-Documenting Tests**: Use descriptive test names following the `Given_When_Then` pattern or similar to ensure tests serve as living documentation.
- **Cross-Language Test Data**: Synchronize property-based testing generators between Rust and Python to ensure consistent edge-case handling.
- **Golden File Testing**: Implement "Golden File" tests for complex AI outputs. Golden files must be versioned in Git (or LFS for large assets) and approved via PR by a domain expert.
- **Circuit Breaker Testing**: Validate that services handle external dependency failures (S3, Redis) gracefully using circuit breaker patterns.
- **Resource Limit Testing**: Ensure services stay within Kubernetes resource limits (CPU/GPU/RAM) and handle exhaustion without cascading failures.
- **Idempotency Validation**: Verify that all processing jobs are idempotent and can be safely retried without data corruption or duplication.
- **Cleanup Assurance**: Tests must verify that temporary files and resources are cleaned up even after catastrophic failures. In CI, use resource labels or TTL to ensure test container cleanup on abort.
- **Containerized Integration Testing**: Use the same container images (Redis, Minio) as production for integration tests (e.g., via Testcontainers or local K8s).
- **Parity Validation**: Ensure that production environment variables, secrets, and volume mount patterns are accurately reflected in the integration test suite.
- **Dependency Version Pinning**: Testing infrastructure versions must be explicitly pinned and synchronized with the configurations found in `deploy/`.
- **Parallel Test Isolation**: Integration tests must be designed for parallel execution (e.g., unique Redis keys/S3 buckets per worker).
- **Resource Re-use**: Heavy AI models and DB connections should be initialized once as session fixtures and shared across multiple tests to minimize overhead.
- **Incremental Testing**: In PR environments, prioritize running tests for modified modules and their direct dependents to speed up the feedback loop.
- **In-Memory Overlays**: Use in-memory drivers or `tmpfs` for tests that require high frequency I/O but not necessarily persistent disk fidelity.

#### Rust
- **Test Types**: Implement unit tests in `src/` modules and integration tests in the `tests/` directory.
- **Asynchronous Testing**: Always use `#[tokio::test]` for async handlers and use cases.
- **Mocking Strategy**: Use traits and manual dependency injection or `mockall` to isolate business logic from infrastructure.

#### Python
- **Test Framework**: Use `pytest` with `pytest-asyncio` for service testing.
- **Isolation**: Mock all external API calls and heavy AI model operations during unit tests.
- **Fixtures**: Leverage shared fixtures for creating test clients and mocking S3 storage.

### Code Quality & Style Rules

#### Linting & Formatting
- **Rust**: Mandatory `rustfmt` and `clippy`. Treat all clippy warnings as errors in CI. Use `cargo doc` to ensure documentation is buildable.
- **Python**: Use `black` for formatting and `ruff` for linter. Follow PEP 8 strictly. Mandatory type hints for all function signatures.

#### Naming Conventions
- **Files & Folders**: Always use `kebab-case`.
- **Semantic Naming**: Use names that describe intent rather than data type. Common abbreviations (`ctx`, `req`, `resp`, `id`) are allowed if used consistently.
- **Standard**: `snake_case` for variables/functions, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.

#### Documentation
- **Module Level**: Every file must start with a module-level doc comment (`//!` in Rust, triple quotes in Python) explaining its responsibility.
- **Comments**: Focus on "Why" rather than "How". Code should be self-documenting for "How"; comments should explain business logic or non-obvious decisions.

#### Structure
- **Nesting**: Limit function nesting to a maximum of 3 levels. Refactor if complexity exceeds this.
- **Responsibility**: Follow the Single Responsibility Principle. Files **must** be kept under 500 lines.
### Development Workflow Rules

#### Git & Commits
- **Branching Strategy**: Use short-lived feature branches. Merge within one week to avoid integration debt.
- **Commit Messages**: Follow Conventional Commits. Commits must be **atomic**: do not mix refactoring and new features in the same commit.
- **PR Process**: All CI checks must pass. Squash and merge is preferred. Code must be verified locally via `docker-compose` before opening a PR.

#### Environment & Security
- **Local Dev**: Maintain `docker-compose` for local orchestration. Ensure parity between local and production environments by using the same base images and Dockerfiles.
- **Secret Management**: Never commit credentials. Use Kubernetes Secrets or environment variables. Enable `git-secrets` or similar pre-commit hooks to prevent leaks.

#### Deployment & Infrastructure
- **Infrastructure as Code**: All Kubernetes manifests must reside in the `deploy/` directory. No manual changes to the cluster.
- **Service Health**: All new services must implement and configure **Liveness** and **Readiness** probes in their deployment manifests.
- **Container Tags**: Use git commit hashes for container image tags in CI/CD.

### Critical Don't-Miss Rules

#### Anti-Patterns
- **No Global State**: Never use global mutable variables. All state must be injected via `AppState` or service context.
- **No Blocking I/O**: Never use blocking calls within async contexts. Use `tokio` or `asyncio` equivalents.
- **No Unsafe Error Handling**: Avoid `unwrap()` and `expect()` in Rust. Always handle `Option` and `Result` properly.

#### Resource Management
- **CUDA/GPU Locking**: Always use `asyncio.Lock` when accessing the Whisper model in Python to prevent concurrent memory exhaustion if needed.
- **File Cleanup**: Use `finally` or context managers to ensure temporary files are deleted after processing, even if an error occurs.

#### Security & Robustness
- **Non-Root Containers**: Ensure Dockerfiles use a non-privileged user for running services.
- **Input Validation**: Sanitize all external inputs (URLs, paths, IDs). Validate S3 objects exist and have non-zero size before processing.
- **Timeout Enforcement**: Every network or model-intensive call must have a mandatory timeout configured.

---

## Usage Guidelines

**For AI Agents:**

- Read this file before implementing any code
- Follow ALL rules exactly as documented
- When in doubt, prefer the more restrictive option
- Update this file if new patterns emerge

**For Humans:**

- Keep this file lean and focused on agent needs
- Update when technology stack changes
- Review quarterly for outdated rules
- Remove rules that become obvious over time

Last Updated: 2026-04-28
