# Story 1.4: Media Ingestion API & Dashboard

Status: done

## Story

As a User,
I want to submit media files (video/audio) via an API or a dashboard,
so that the orchestrator can start the processing pipeline.

## Acceptance Criteria

1. **Job Ingestion API**: A `POST /api/jobs` endpoint is implemented to receive new job requests.
2. **Payload Validation**: The API validates the JSON payload (required fields: `media_url`, `language`, etc.).
3. **Persistence Integration**: Submitted jobs are assigned a UUID and saved to S3 using the `JobRepository`.
4. **JWT Protection**: The endpoint is protected by the JWT middleware (AC from Story 1.2).
5. **Basic Dashboard**: A static HTML dashboard is served at `/` listing recent jobs (mocked or retrieved from S3).

## Tasks / Subtasks

- [x] Create Job Use Case (AC: 1, 3)
  - [x] Implement `CreateJobUseCase` in `src/application/use_cases/create_job.rs`.
  - [x] Inject `JobRepository` into the use case.
- [x] Implement Job Handler (AC: 1, 2, 4)
  - [x] Create `src/interfaces/http/job_handlers.rs`.
  - [x] Implement `create_job_handler` using Axum.
  - [x] Map `POST /api/jobs` in `main.rs`.
- [x] Implement Basic Dashboard (AC: 5)
  - [x] Create a simple `index.html` in `services/orchestrator/static/`.
  - [x] Configure Axum to serve static files from the `static/` directory.
- [x] End-to-End Test (AC: 1, 3, 4)
  - [x] Submit a job via `curl` with a valid JWT and verify it appears in S3.

## Dev Notes

- **UUID**: Use the `uuid` crate for generating job IDs.
- **Static Files**: Use `tower_http::services::ServeDir`.
- **Validation**: Use `serde` validation if possible or manual checks.

### Project Structure Notes

- New file: `services/orchestrator/src/application/use_cases/create_job.rs`
- New file: `services/orchestrator/src/interfaces/http/job_handlers.rs`
- New file: `services/orchestrator/static/index.html`

### References

- [PRD: Media Ingestion](file:///home/joseph/git/Keryx/_bmad-output/planning-artifacts/prd.md#L50)
- [Architecture: Use Case Pattern](file:///home/joseph/git/Keryx/_bmad-output/planning-artifacts/architecture.md#L55)

## Dev Agent Record

### Agent Model Used

Antigravity (Gemini 2.0)

### Debug Log References

- [Cargo Check Output](file:///home/joseph/git/Keryx/services/orchestrator/target/debug/...)

### Completion Notes List

- Added `uuid` dependency.
- Implemented `CreateJobUseCase` for business logic.
- Created `create_job_handler` (POST /api/jobs).
- Added static dashboard with premium CSS.
- Integrated static serving in `main.rs`.

### File List

- `services/orchestrator/Cargo.toml`
- `services/orchestrator/src/application/use_cases/create_job.rs`
- `services/orchestrator/src/application/use_cases/mod.rs`
- `services/orchestrator/src/interfaces/http/job_handlers.rs`
- `services/orchestrator/src/interfaces/http/mod.rs`
- `services/orchestrator/src/main.rs`
- `services/orchestrator/static/index.html`
