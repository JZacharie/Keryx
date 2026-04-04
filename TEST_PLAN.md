# 🏛️ Keryx - Unified Test Plan

Keryx is a distributed video localization pipeline using Axum, Dragonfly (Redis), MinIO (S3), Whisper, and Ollama. This plan outlines both manual and automated verification strategies.

## 1. Unit Tests (Rust)
Run with `cargo test` to verify domain logic.

| Component | Target | Status |
|-----------|--------|--------|
| `Job` Entity | State transitions and asset mapping | ✅ Implemented |
| `Ingestor` Use Case | Logic flow (Orchestration) | 🛠️ In-progress (Mocked) |
| `Redis` Repos | Connection and serialization | ✅ Implemented |

## 2. Integration Tests (API + Infrastructure)
Verifies the full request lifecycle. Requires Local Redis/MinIO or Cluster access.

### 2.1 Job Submission (UI -> Backend)
- **Action**: Use the web interface at `https://keryx.p.zacharie.org` (or `http://localhost:3000`).
- **Input**: `https://www.youtube.com/watch?v=PsPqWLoZaMc`
- **Languages**: Select FR, EN.
- **Expectation**: `API 202 ACCEPTED` with a unique UUID. Check Redis (`KEYS *`) to see the entry.

### 2.2 Health & Probes
- **Path**: `/health`
- **Method**: GET
- **Expectation**: `200 OK` (Plain text string "OK").

### 2.3 Asset Management
Check MinIO (`keryx-raw` bucket) during ingestion phases:
- `jobs/<id>/raw/audio.wav` (Ingest phase)
- `jobs/<id>/raw/frame_*.png` (Analysis phase)

### 3.4 Diffusion Engine (Stylizer)
- **Path**: `/style`
- **Method**: POST
- **Input**:
```json
{
  "image_url": "https://minio.zacharie.org/keryx/raw/frame_0001.jpg",
  "prompt": "Cyberpunk glassmorphism style, vibrant neon highlights",
  "strength": 0.5
}
```
- **Expectation**: `200 OK` with `url` of the stylized image. Verify visually in MinIO.

## 4. UI / UX Audit (Aesthetics & Interaction)
Verified using modern browser standards.

- [ ] **Responsive Design**: Test on mobile vs desktop (grid layout).
- [ ] **Cyberpunk Effects**: Verify scanlines and flickering on the logo.
- [ ] **Error Handling**: Disconnect the network and verify "NETWORK_BRIDGE_COLLAPSE" message.
- [ ] **Styling Preview**: Verify the "Style" button correctly triggers the diffusion engine.

## 5. CI/CD Validation
GitHub Actions pipeline tracking.

- [ ] **Gitleaks**: Ensure no secrets are committed.
- [ ] **Docker Push**: Verify `ghcr.io/jzacharie/keryx-ingestor:latest` is updated on push.
- [ ] **ArgoCD Sync**: Confirm green status in `https://argocd.p.zacharie.org`.

---
*Last Updated: 2026-04-04*
