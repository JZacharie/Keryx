# 🏛️ Keryx - Unified Test Plan (v2)

Keryx follows a structured testing pyramid to ensure fast feedback during development and robust validation before deployment.

## 1. Unit Tests (`keryx-core`)
Run with `cargo test -p keryx-core` to verify domain logic in isolation (< 5 seconds).

- **Scope**: Entities (`Job` state transitions), Ports, and pure logic.
- **Rules**: Zero I/O operations. Network or Database calls must use Mock implementations of the domain Repositories.

## 2. Integration Tests (`keryx-ingestor`)
Run with `cargo test -p keryx-ingestor` to verify adapters and HTTP routes.

- **Scope**: Axum Route Handlers, Redis Repository instantiation, and S3 Upload logic.
- **Methodology**: Use `testcontainers-rs` to automatically spin up temporary Redis and MinIO instances during the test run.
- **Mocking**: External AI services (Whisper, Diffusion, TTS) should be mocked via basic WireMock HTTP servers to avoid GPU requirements.

## 3. End-to-End (E2E) API Validation
Replaces manual UI click-testing. Executed via a Python script (`scripts/test_e2e.py`) against a local `docker-compose` cluster or the staging environment.

### 3.1 Job Submission Flow
- **Action**: Submit `POST /api/jobs` with a Youtube URL.
- **Expectation**: Receive `202 ACCEPTED` and a `job_id`.

### 3.2 Polling & Artifact Verification
- **Action**: Poll job status until it reaches `Completed`.
- **Validation 1**: Check MinIO `keryx-raw` for intermediate assets (`audio.wav`, `transcription.json`).
- **Validation 2**: Check MinIO `exports` directory for the final outputs (`video_en.mp4`, `video_fr.mp4`, `.pptx`).

## 4. CI/CD Validation Tracking
Automated in `.github/workflows/`.

- **Pull Requests (pr.yml)**: Fast validation (cargo fmt, clippy, unit tests only).
- **Master Branch (build.yml)**: Full image building with sccache and docker cache, followed by ArgoCD sync.
- **Post-Rollout**: ArgoCD Webhooks should trigger a smoke test (small 5s video ingestion) to validate the live cluster.

---
*Last Updated: 2026-04-10*
