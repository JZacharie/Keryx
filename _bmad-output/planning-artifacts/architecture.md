---
stepsCompleted: ['step-01-init', 'step-02-context', 'step-03-starter', 'step-04-decisions', 'step-05-patterns', 'step-06-structure', 'step-07-validation', 'step-08-complete']
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/project-context.md'
  - 'README.md'
  - 'KERYX_PROCESS.md'
  - 'TEST_PLAN.md'
workflowType: 'architecture'
project_name: 'Keryx'
user_name: 'Joseph'
date: '2026-04-28'
status: 'complete'
completedAt: '2026-04-28'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
Orchestration of a multi-stage asynchronous pipeline (YouTube -> Whisper -> Llama -> XTTS -> FFmpeg). The architecture must support long-running jobs with persistent state and granular fault recovery.

**Non-Functional Requirements:**
High-performance processing (Ratio < 1:1) and strict privacy (AES-256 + 24h Auto-purge). These drive a "Zero-Copy" data strategy and automated data lifecycle management.

**Scale & Complexity:**
- **Primary Domain:** API Backend / Media Orchestration (AI-centric)
- **Complexity Level:** High (Multi-modal coordination & GPU scaling)
- **Architectural Pattern:** Hexagonal Architecture (Rust Orchestrator) + Distributed AI Workers (Python).

### Technical Constraints & Dependencies
- **GPU Dependency:** Guaranteed VRAM allocation via Kubernetes WorkerGuard.
- **Data Locality:** Avoidance of large file transfers; orchestration via URL/Object references in S3/MinIO.
- **Hybrid Stack:** Strict boundary between Rust (Performance/Safety) and Python (AI Ecosystem) via standardized schemas.

### Cross-Cutting Concerns Identified
- **Content-Based Caching:** Caching strategy based on audio/video segment hashing to prevent redundant AI inference.
- **Distributed Traceability:** Mandatory `Trace-ID` propagation across Rust and Python services for observability.
- **Biometric TTL:** Native system capability for automatic 24-hour purging of sensitive voice signatures and temporary assets.

## Starter Template Evaluation

### Primary Technology Domain
**Full-Stack Media Pipeline** (Rust Orchestrator + Python Workers + Next.js Dashboard).

### Starter Options Considered

- **Orchestrator (Rust/Axum 0.7):** Alignement sur l'architecture hexagonale avec gestion asynchrone native via `tokio`.
- **Workers (Python/FastAPI):** Workers IA spécialisés utilisant Pydantic pour la validation stricte des contrats de données.
- **Dashboard (Next.js 14+ App Router):** Interface moderne utilisant **Tailwind CSS** et **Shadcn/ui** pour un rendu premium et réactif.

### Selected Reference Architecture: Keryx Full-Stack Engine

**Rationale for Selection:**
L'utilisation de Next.js complète l'orchestrateur Rust en fournissant une interface interactive "one-click" conforme à la vision du PRD, tout en maintenant une séparation claire des responsabilités (SOC).

**Initialization Command (Frontend Example):**
```bash
npx create-next-app@latest ./dashboard --typescript --tailwind --eslint --app --src-dir --import-alias "@/*"
```

**Architectural Decisions Provided:**
- **Language & Runtime:** Rust (Orchestrator), Python (AI), TypeScript (Frontend).
- **Styling Solution:** Tailwind CSS pour le dashboard.
- **Communication Layer:** Contrats de données synchronisés (JSON/OpenAPI) pour assurer la parité entre les services.
- **Observability:** `Trace-ID` partagé sur l'ensemble de la stack via des middlewares standardisés.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- **Data Persistence:** Abandon de PostgreSQL au profit d'une **Persistence S3-Native**. Chaque job est stocké sous forme de fichier **JSON** individuel dans MinIO.
- **Service Communication:** Communication synchrone via **REST API** (FastAPI) pour le MVP, orchestrée par Rust.
- **Contract Management:** **OpenAPI 3.1** comme source de vérité unique pour les schémas de données.

**Important Decisions (Shape Architecture):**
- **Security:** Authentification via **JWT (RS256)** pour sécuriser les échanges entre le Dashboard et l'Orchestrateur.
- **Concurrency Management:** Modèle **Single-Writer** (Orchestrateur uniquement) garantissant l'absence de conflits sur les fichiers JSON du S3.

### Data Architecture
- **Storage:** S3 / MinIO.
- **Format:** JSON Objects (per-job).
- **Driver:** `aws-sdk-s3` (Rust) pour la persistence des métadonnées et des actifs média.

### Authentication & Security
- **Auth Method:** JWT (JSON Web Tokens).
- **Public Key Infrastructure:** RS256 pour permettre aux services de valider les jetons sans partager le secret.

### API & Communication Patterns
- **API Spec:** OpenAPI v3.1 (utoipa côté Rust).
- **Protocol:** HTTP/REST.
- **Real-time:** WebSockets pour le feedback utilisateur sur le Dashboard.

### Infrastructure & Deployment
- **Deployment Platform:** Kubernetes (via Helm).
- **Persistence:** Stockage objet (S3) pour les médias et les métadonnées de jobs.

## Implementation Patterns & Consistency Rules

### Naming Patterns
- **Database:** `snake_case` pour les tables (pluriel) et les colonnes.
- **API JSON:** `camelCase` pour les clés (mapping Serde obligatoire).
- **Code:** Conventions standards (Rust: `snake_case`, TS: `camelCase`, Python: `snake_case`).

### Structure Patterns
- **Hexagonal Architecture:** Séparation stricte entre `domain`, `application`, `infrastructure` et `interfaces`.
- **Domain Purity:** Le dossier `src/domain/` ne doit avoir **aucune dépendance externe** (frameworks, drivers). Tout doit passer par des traits (ports).
- **Co-location:** Tests unitaires co-localisés avec le code source.

### Communication Patterns
- **WebSocket Messages:** Format standardisé : `{ "type": "status_update", "jobId": "...", "payload": { ... } }`.
- **Traceability:** Propagation obligatoire du header `X-Trace-Id` à chaque saut réseau.
- **Error Handling:** Utilisation des "Problem Details" (RFC 7807) pour les erreurs API.

### Enforcement Guidelines
**Tous les agents IA DOIVENT :**
1. Valider la pureté du domaine avant de soumettre un changement.
2. Utiliser les schémas OpenAPI comme unique source de vérité pour les types.
3. Implémenter des tests unitaires pour chaque nouveau Use Case.
## Project Structure & Boundaries

### Complete Project Directory Structure

```text
keryx/
├── orchestrator/ (Rust - Axum)
│   ├── src/
│   │   ├── domain/           # Entités métier & Ports (Traits)
│   │   ├── application/      # Use Cases (Ingestion, Localization, Export)
│   │   ├── infrastructure/   # Adapters (S3 Repository, Worker Clients)
│   │   ├── interfaces/       # Handlers (HTTP, WebSocket)
│   │   ├── main.rs           # Entrypoint
│   │   └── lib.rs
│   ├── tests/                # Tests d'intégration
│   └── Cargo.toml
├── workers/ (Python - FastAPI)
│   ├── voice-extractor/      # Service Whisper
│   ├── speaker-cloner/       # Service XTTS v2
│   ├── visual-cleaner/       # Service SD/ControlNet
│   ├── shared/               # Modèles Pydantic & Utils
│   └── requirements.txt
├── dashboard/ (Next.js)
│   ├── src/
│   │   ├── app/              # Routes (App Router)
│   │   ├── components/       # UI (Shadcn) & Features
│   │   ├── lib/              # API Clients & Hooks
│   │   └── types/            # Types TS générés depuis OpenAPI
│   └── package.json
├── contracts/                # Source de vérité (OpenAPI / JSON Schemas)
├── infrastructure/           # Helm Charts & K8s Manifests
└── docker-compose.yml        # Dev local
```

### Architectural Boundaries

**API Boundaries:**
L'orchestrateur Rust expose l'API publique `/api/v1` et gère l'authentification JWT. Les workers Python exposent des APIs internes non-publiques consommées par l'orchestrateur.

**Component Boundaries:**
Le `Domain` Rust est isolé et ne dépend d'aucun framework. Les interactions avec S3 et les workers passent par des traits définis dans le domaine et implémentés dans l'infrastructure.

**Data Boundaries:**
Single-Writer Pattern : Seul l'orchestrateur modifie les métadonnées des jobs (JSON sur S3). Les workers lisent les actifs et écrivent de nouveaux fichiers (ex: transcription), mais ne modifient pas l'état global du job.

### Requirements to Structure Mapping

**Feature/Epic Mapping:**
- **Media Ingestion :** `orchestrator/src/application/use_cases/ingestion/`
- **Transcription :** `workers/voice-extractor/` (Inférence) + `orchestrator/src/application/use_cases/localization/` (Orchestration)
- **Voice Cloning :** `workers/speaker-cloner/`
- **Dashboard :** `dashboard/src/app/`

**Cross-Cutting Concerns:**
- **Traceability :** Middleware `X-Trace-Id` dans chaque service.
- **Security :** Garde-fou JWT dans l'orchestrateur (`orchestrator/src/interfaces/http/middleware/auth.rs`).
- **Data Lifecycle (TTL) :** Script ou worker spécialisé dans `infrastructure/scripts/purge_s3_assets.sh`.

## Architecture Validation Results

### Coherence Validation ✅
Les choix technologiques (Axum, FastAPI, S3-JSON) sont compatibles et optimisés pour un pipeline asynchrone multi-modal. Les patterns de nommage assurent la parité des données entre Rust et TypeScript.

### Requirements Coverage Validation ✅
Chaque étape du pipeline est supportée par un worker spécialisé ou un cas d'usage orchestrateur. Les NFR de performance et de confidentialité sont adressés par la stratégie S3-Native.

### Architecture Readiness Assessment
**Overall Status:** READY FOR IMPLEMENTATION
**Confidence Level:** High (Basé sur l'alignement Kusanagi et la simplification S3).

### Implementation Handoff
**AI Agent Guidelines:**
- Suivre les décisions architecturales à la lettre.
- Respecter l'étanchéité absolue de `orchestrator/src/domain/`.
- Utiliser les schémas OpenAPI pour toute communication inter-services.
- Se référer à ce document pour toute question d'intégration.
