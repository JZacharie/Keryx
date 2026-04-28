# Story 1.4: Media Ingestion API & Dashboard

Status: in-progress

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

- [ ] Create Job Use Case (AC: 1, 3)
  - [ ] Implement `CreateJobUseCase` in `src/application/use_cases/create_job.rs`.
  - [ ] Inject `JobRepository` into the use case.
- [ ] Implement Job Handler (AC: 1, 2, 4)
  - [ ] Create `src/interfaces/http/job_handlers.rs`.
  - [ ] Implement `create_job_handler` using Axum.
  - [ ] Map `POST /api/jobs` in `main.rs`.
- [ ] Implement Basic Dashboard (AC: 5)
  - [ ] Create a simple `index.html` in `services/orchestrator/static/`.
  - [ ] Configure Axum to serve static files from the `static/` directory.
- [ ] End-to-End Test (AC: 1, 3, 4)
  - [ ] Submit a job via `curl` with a valid JWT and verify it appears in S3.

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

### Debug Log References

### Completion Notes List

### File List
