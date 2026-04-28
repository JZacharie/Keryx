---
project_name: 'Keryx'
user_name: 'autobot'
date: '2026-04-28'
sections_completed: ['technology_stack']
existing_patterns_found: 5
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

_To be expanded in the next step._
