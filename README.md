# 🏛️ Keryx - Automated Video Localization Pipeline

Named after the Ancient Greek herald (κῆρυξ), the inviolable messenger of truth. **Keryx** is an automated, event-driven pipeline designed to convert technical presentation videos into localized and re-stylized versions with frame-accurate precision.

The system ensures that complex technical content remains accurate while the visual aesthetic and the speaker's original voice are preserved and adaptively translated to target linguistic matrices.

## 🎯 Objective
Automate the end-to-end localization of YouTube presentation videos, including:
- **Slide Analysis**: Frame-accurate detection of slide transitions using `ffmpeg` scene detection.
- **Audio Transcription**: High-fidelity STT using **Faster-Whisper**.
- **Contextual Translation**: Preservation of technical terms via **Ollama (Llama 3)**.
- **Visual Stylization**: Slide regeneration using **Stable Diffusion (ControlNet)**.
- **Voice Cloning** (Phase 3): **Coqui XTTS v2** for speaker voice preservation.
- **Video Composition**: **MoviePy** for final assembly and time-stretching.

## 🏗️ Technical Architecture
Keryx is built using **Hexagonal Architecture** (Ports & Adapters) in Rust to ensure strict isolation between domain logic and infrastructure (S3, Redis, AI endpoints).

### 🧡 Interfaces
- **Web UI**: A modern "Cyberpunk/Glassmorphism" interface inspired by the **Kusanagi** aesthetic, featuring real-time job status tracking and GITS-inspired visuals.
- **REST API**: Axum-based endpoints for job creation (`/api/jobs`) and health monitoring (`/health`).

### 💙 Domain logic
- **Job Entity**: State machine-driven job lifecycle (Downloading → Analyzing → Transcribing → Translating).
- **Assets Map**: Mapping detected slide frames to their corresponding transcribed segments for accurate localized overlays.

### 💛 Infrastructure (Adapters)
- **Job Repository**: Redis-backed persistence using **DragonflyDB**.
- **Storage Repository**: S3-compatible asset management via **MinIO** (Path-style addressing).
- **Video Pipeline**: `yt-dlp` (piloted with Node.js runtime) and `ffmpeg`.

## 🚀 Deployment & CI/CD
The project features a professional-grade automation pipeline:
- **Security Audit**: Automated **Gitleaks** scans on every push.
- **Optimized Builds**: Multi-arch Docker images built via GitHub Actions with high-performance Rust caching.
- **GitOps Management**: Integrated with **ArgoCD** for automated synchronization and deployment on the `jo3` cluster.
- **Observability**: Instant **Slack** notifications upon successful rollouts.

## 🛠️ Configuration
Keryx is optimized for cluster environments using these variables:
```bash
REDIS_URL=redis://:PASSWORD@dragonfly.dragonfly.svc:6379
S3_BUCKET=keryx
S3_ENDPOINT=https://minio-170-api.zacharie.org
AWS_ACCESS_KEY_ID=keryx
AWS_SECRET_ACCESS_KEY=REDACTED
```

## 📜 Repository Structure
```
.
├── services/
│   └── ingestor/         # Core Rust service (Axum)
│       ├── src/          # Hexagonal project structure
│       ├── static/       # Cyberpunk Web UI
│       └── Dockerfile    # Optimized multi-stage build
├── deploy/
│   └── helm/             # Kubernetes localized charts
├── TEST_PLAN.md          # Comprehensive verification strategy
└── README.md
```

## 🚦 Status: 🟢 Production Ready (Phase 1 & 2)
The Ingestor unit is currently active and capable of processing localized YouTube streams into the `keryx` asset bucket with full LLM-driven translation.

---
*Powered by Rust, Ollama, Whisper, and the Ancient Greek spirit.*
