# Story 1.1: Project Skeleton & Orchestrator Setup

Status: ready-for-dev

## Story

As a Developer,
I want to set up the mono-repo structure and the Rust orchestrator (Axum/Tokio),
so that hexagonal architecture purity is enforced from the start.

## Acceptance Criteria

1. **Project structure** follows the boundaries: `orchestrator/`, `workers/`, `dashboard/`, `contracts/`, `deploy/`. [Source: architecture.md]
2. **Rust orchestrator** (Axum/Tokio) is initialized and starts on port 3000. [Source: epics.md]
3. **Hexagonal layers** created in `orchestrator/src/`: `domain/`, `application/`, `infrastructure/`, `interfaces/`. [Source: architecture.md]
4. **Domain Purity**: `orchestrator/src/domain/` contains only pure Rust logic with ZERO external framework dependencies (no axum, no serde, etc.). [Source: architecture.md]
5. **Initial Health Check**: A simple GET `/health` endpoint is implemented in the orchestrator.

## Tasks / Subtasks

- [ ] Initialize Mono-repo Structure (AC: 1)
  - [ ] Create top-level directories: `orchestrator`, `workers`, `dashboard`, `contracts`, `deploy`, `_bmad-output`.
- [ ] Initialize Rust Orchestrator (AC: 2, 3)
  - [ ] Run `cargo new orchestrator` in the root.
  - [ ] Configure `Cargo.toml` with Axum 0.7, Tokio 1.0 (full), Serde, and Tower-HTTP. [Source: project-context.md]
  - [ ] Create folder structure: `src/domain`, `src/application`, `src/infrastructure`, `src/interfaces`.
- [ ] Implement Basic Axum Server (AC: 2, 5)
  - [ ] Implement a basic Axum server in `orchestrator/src/main.rs`.
  - [ ] Add a `health_check` handler in `src/interfaces/http/health.rs`.
  - [ ] Map `GET /health` in `main.rs`.
- [ ] Domain Purity Validation (AC: 4)
  - [ ] Add a placeholder entity in `src/domain/entities/mod.rs` to verify no external dependencies are needed.

## Dev Notes

- **Technology Stack**: Axum 0.7, Tokio 1.0.
- **Port**: 3000.
- **Mono-repo**: This is a brownfield-like setup but we are creating the clean structure now.
- **Next.js Dashboard**: Will be initialized in a future story, but the folder `dashboard/` should exist.
- **Traceability**: Ensure `tracing` is initialized in `main.rs`. [Source: project-context.md]

### Project Structure Notes

- **orchestrator/**: The main Rust service.
- **workers/**: Future Python FastAPI services.
- **dashboard/**: Future Next.js frontend.
- **contracts/**: Shared OpenAPI/AsyncAPI files.

### References

- [Architecture: Mono-repo Structure](file:///home/joseph/git/Keryx/_bmad-output/planning-artifacts/architecture.md#L150)
- [Project Context: Rust Tech Stack](file:///home/joseph/git/Keryx/_bmad-output/project-context.md#L19)
- [Hexagonal Architecture Rules](file:///home/joseph/git/Keryx/_bmad-output/project-context.md#L47)

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
