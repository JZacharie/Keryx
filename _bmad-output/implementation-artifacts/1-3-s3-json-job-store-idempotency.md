# Story 1.3: S3-JSON Job Store & Idempotency

Status: in-progress

## Story

As a Service,
I want to store job requests in an S3-compatible bucket as JSON files,
so that job data is persisted and can be retrieved by workers or for re-processing.

## Acceptance Criteria

1. **S3 Integration**: The orchestrator can connect to an S3-compatible bucket (configured via environment variables: `S3_BUCKET`, `S3_ENDPOINT`, `S3_ACCESS_KEY`, `S3_SECRET_KEY`).
2. **JSON Persistence**: Jobs are stored as JSON files with the naming convention `jobs/{job_id}.json`.
3. **Hexagonal Pattern (Port)**: A `JobRepository` trait is defined in `domain/ports/` with methods for `save`, `get_by_id`, and `exists`.
4. **Idempotency**: The system checks if a `job_id` already exists before creating a new entry to prevent duplicate processing.
5. **Serialization**: The `Job` domain entity is properly serialized/deserialized using `serde`.

## Tasks / Subtasks

- [ ] Setup S3 Dependencies (AC: 1)
  - [ ] Add `aws-config`, `aws-sdk-s3` to `services/orchestrator/Cargo.toml`.
- [ ] Define Domain Port (AC: 3)
  - [ ] Create `src/domain/ports/job_repository.rs`.
  - [ ] Define `async_trait` for `JobRepository`.
- [ ] Implement S3 Adapter (Infrastructure) (AC: 1, 2)
  - [ ] Create `src/infrastructure/repositories/s3_job_repository.rs`.
  - [ ] Implement the `JobRepository` trait using AWS SDK.
- [ ] Update Domain Entities (AC: 5)
  - [ ] Add `serde` derives to `Job` and `JobStatus` in `src/domain/entities/job.rs`.
- [ ] Implement Idempotency Logic (AC: 4)
  - [ ] Add a check in the (future) use case or service layer (or basic check in the adapter for now).

## Dev Notes

- **SDK**: Use `aws-sdk-s3` for compatibility with MinIO and other providers.
- **Async**: Ensure all repository methods are `async` using `async_trait`.
- **Environment**: Use `dotenvy` if needed or rely on standard env vars.

### Project Structure Notes

- New file: `services/orchestrator/src/domain/ports/job_repository.rs`
- New file: `services/orchestrator/src/infrastructure/repositories/s3_job_repository.rs`

### References

- [PRD: Job Persistence](file:///home/joseph/git/Keryx/_bmad-output/planning-artifacts/prd.md#L45)
- [Architecture: Repository Pattern](file:///home/joseph/git/Keryx/_bmad-output/planning-artifacts/architecture.md#L60)

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
