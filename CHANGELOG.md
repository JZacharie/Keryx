# Changelog - Keryx

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-04-10

### Added
- **Architectural Leap**: Migration to a Cargo Workspace with modular crates.
- **Keryx-Core**: New domain-driven library isolating business logic from infrastructure.
- **Cinematic Intro**: Systematic integration of `begin.mp4` with a 1s freeze and a 2s crossfade transition for all exports (EN, FR, Joseph).
- **Auto-Scale Support**: Added `KubeScalingRepository` for dynamic worker orchestration (KEDA compatible).
- **Slack Notifications**: Real-time job conclusion notifications featuring download links and PPTX status.

### Changed
- **CI/CD Revolution**: 
  - Integrated `cargo-chef` for dependency layer caching.
  - Implemented `sccache` for persistent compilation cache.
  - Switched to `mold` linker for sub-second linking times.
  - Optimized base images and build context for smaller, faster deployments.
- **Stability**: 
  - Updated `k8s-openapi` to targeted `v1_30` for reduced memory footprint.
  - Enhanced health-check logic with redirect handling and increased timeouts (300s).
  - Standardized all AI services on Persistent Volume Claims (PVC) for model weight persistence.

### Fixed
- Fixed `pptx-builder` health check failures by correcting service port from 8002 to 80 in Kubernetes.
- Resolved "Phase 0b" pipeline hangs caused by rigid health probing.
- Fixed AWS SDK and OpenSSL linking issues in Docker builds.

## [0.1.0] - 2026-04-09
- Initial prototype release.
- Core ingestion pipeline using FFmpeg and AI models.
