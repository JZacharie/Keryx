---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-generate-stories', 'step-04-final-validation']
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/architecture.md'
status: 'complete'
completedAt: '2026-04-28'
---

# Keryx - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Keryx, decomposing the requirements from the PRD and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: Ingestion of YouTube URLs or local MP4 uploads via the dashboard or API.
FR2: Extraction of raw audio tracks and visual keyframes for subsequent processing.
FR3: Validation of input media (duration limit of 15 minutes for MVP).
FR4: Biometric voice cloning preserving original speaker identity (XTTS v2).
FR5: Frame-accurate synchronization of translated audio segments with visual keyframes.
FR6: Real-time job status monitoring via API and WebSocket streaming.
FR7: Phase-level idempotency allowing resumes from checkpoints in case of failure.
FR8: Dynamic GPU resource scaling based on pipeline demand (WorkerGuard).
FR9: Management and enforcement of custom technical glossaries to override AI translations.
FR10: Final delivery of localized MP4 video and editable PPTX files.

### NonFunctional Requirements

NFR1: End-to-end processing ratio < 1:1 (e.g., 10 min video in < 10 min).
NFR2: Security: AES-256 encryption at rest and TLS 1.3 in transit.
NFR3: Privacy: Automatic deletion of biometric assets 24 hours after completion.
NFR4: Scalability: Kubernetes node readiness < 2 minutes via pre-provisioned images.
NFR5: Reliability: 99.9% orchestrator uptime.
NFR6: Resilience: Fault isolation between AI workers and the core orchestrator.

### Additional Requirements

- **Starter Template:** Hybrid stack (Rust/Axum Orchestrator, Python/FastAPI Workers, Next.js Dashboard).
- **Architecture Pattern:** Hexagonal Architecture (Domain Purity enforced in `src/domain/`).
- **Data Persistence:** S3-Native (MinIO) using individual JSON files per job (Single-Writer Pattern).
- **Communication:** OpenAPI 3.1 contracts, REST for inter-service calls, WebSockets for UI progress.
- **Observability:** Mandatory `X-Trace-Id` propagation across all services.
- **Security Implementation:** JWT (RS256) for Dashboard/Orchestrator authentication.
- **Optimization:** Content-based caching via Redis/Hashing to avoid redundant AI inference.

### UX Design Requirements

(No separate UX document found; requirements integrated into FRs and Additional Requirements).

### FR Coverage Map

FR1: Epic 1 - Ingestion (YouTube/Upload)
FR2: Epic 2 - Audio Extraction & Transcription
FR3: Epic 1 - Media Validation
FR4: Epic 3 - Voice Cloning (XTTS v2)
FR5: Epic 3 - A/V Synchronization
FR6: Epic 2 - Status Monitoring & WS Feedback
FR7: Epic 1 - Phase-level Idempotency
FR8: Epic 5 - Dynamic GPU Scaling
FR9: Epic 2 - Glossary Management & Review
FR10: Epic 4 - Export (MP4/PPTX)

## Epic List

### Epic 1: Foundations, Security & Ingestion
Mettre en place le socle technique sécurisé (JWT), l'idempotence de l'orchestrateur et permettre l'ingestion sécurisée des premiers médias (YouTube/MP4).
**FRs covered:** FR1, FR3, FR7.

### Epic 2: Transcription, Review & Glossary
Générer la transcription via Whisper, permettre sa révision/correction par l'utilisateur et appliquer le glossaire technique pour garantir la précision du contenu.
**FRs covered:** FR2, FR6, FR9.

### Epic 3: AI Voice Cloning & Localization
Traduire le contenu et générer une nouvelle piste audio localisée préservant l'identité vocale originale (XTTS v2), synchronisée avec les visuels.
**FRs covered:** FR4, FR5.

### Epic 4: Visual Cleaning & Multi-format Export
Produire la vidéo finale nettoyée de ses textes sources et exporter les livrables finaux en formats MP4 et PPTX éditables.
**FRs covered:** FR10.

### Epic 5: GPU Scaling & Data Lifecycle
Optimiser les ressources GPU via WorkerGuard et assurer la purge automatique sécurisée des données biométriques sous 24h.
**FRs covered:** FR8, NFR3.

## Epic 1: Foundations, Security & Ingestion

L'objectif de cet Epic est de poser les bases techniques (Rust/Axum), la sécurité (JWT), le mécanisme d'idempotence et de permettre l'envoi des premières vidéos.

### Story 1.1: Project Skeleton & Orchestrator Setup

As a Developer,
I want to set up the mono-repo structure and the Rust orchestrator (Axum/Tokio),
So that hexagonal architecture purity is enforced from the start.

**Acceptance Criteria:**

**Given** a new project repository
**When** I run the initial setup
**Then** the directory structure follows the architectural boundaries (orchestrator, workers, dashboard, contracts)
**And** the Rust orchestrator starts an Axum server on port 3000
**And** `src/domain/` has zero external framework dependencies.

### Story 1.2: JWT Authentication System (RS256)

As a User (Sarah/Alex),
I want to authenticate via a secure JWT token,
So that I can access ingestion and monitoring features for my jobs.

**Acceptance Criteria:**

**Given** an unauthorized request to `/api/v1/jobs`
**When** the orchestrator receives the request
**Then** it returns a 401 Unauthorized response
**And** when a valid JWT signed with RS256 is provided, the request is permitted
**And** the Dashboard includes the token in the `Authorization` header.

### Story 1.3: S3-JSON Job Store & Idempotency

As a System,
I want to store job states as individual JSON files on S3 (MinIO),
So that treatments can be resumed from checkpoints in case of failure.

**Acceptance Criteria:**

**Given** a new job submission
**When** the orchestrator processes the ingestion
**Then** a new JSON file is created at `s3://keryx/jobs/{job_id}.json`
**And** every phase update (e.g., 'ingested', 'transcribed') is atomically reflected in the JSON file
**And** the `S3JobRepository` implementation is strictly isolated in the infrastructure layer.

### Story 1.4: Media Ingestion API & Dashboard

As a User,
I want to submit a YouTube URL or an MP4 file via the dashboard,
So that I can start the localization pipeline.

**Acceptance Criteria:**

**Given** a media submission (URL or file)
**When** the duration is less than 15 minutes (MVP limit)
**Then** the media is uploaded to the S3 'raw' bucket
**And** a 201 Created response is returned with a unique `job_id`
**And** if the duration exceeds 15 minutes, a 400 Bad Request is returned with a clear error message.

## Epic 2: Transcription, Review & Glossary

Cet Epic transforme l'audio brut en un texte structuré, tout en permettant à l'utilisateur d'intervenir pour garantir la qualité technique du contenu.

### Story 2.1: Voice Extractor Worker (Whisper)

As a System,
I want to extract the audio from the video and generate a text transcription via Whisper,
So that the content is prepared for translation.

**Acceptance Criteria:**

**Given** a job in 'ingested' status
**When** the Voice Extractor worker picks up the job
**Then** it extracts the audio track using FFmpeg
**And** it produces a JSON transcription file with timestamps
**And** the file is saved to `s3://keryx/jobs/{job_id}/transcription.json`.

### Story 2.2: WebSocket Progress Monitoring

As a User,
I want to see the job progress (in %) and the current phase in real-time on the dashboard,
So that I know when my transcription is ready for review.

**Acceptance Criteria:**

**Given** an active localization job
**When** a phase or percentage update occurs in the orchestrator
**Then** a WebSocket message is pushed to the client following the standard contract
**And** the Dashboard UI updates the progress bar and status label without page refresh
**And** the `X-Trace-Id` is present in the WebSocket payload metadata.

### Story 2.3: Transcription Review Interface

As a User (Sarah/Alex),
I want to be able to view and edit the text generated by Whisper before translation,
So that I can correct any errors in technical terms.

**Acceptance Criteria:**

**Given** a completed transcription
**When** I open the review interface on the dashboard
**Then** I can see the text segments mapped to their timestamps
**And** any edits I make are saved back to the JSON job store on S3
**And** the job status remains 'reviewing' until I confirm the changes.

### Story 2.4: Technical Glossary Enforcement

As a User,
I want to define a list of technical terms to be kept as-is or translated specifically,
So that business accuracy of the localization is guaranteed.

**Acceptance Criteria:**

**Given** a confirmed transcription and a glossary file
**When** the translation phase begins
**Then** the orchestrator applies the glossary rules to override AI-generated translations
**And** the final translation reflects the specific terminology defined in the glossary.

## Epic 3: AI Voice Cloning & Localization

C'est ici que s'opère la magie de Keryx : la génération d'une voix naturelle qui ressemble à l'originale, mais dans la langue cible.

### Story 3.1: Speaker Cloner Worker (XTTS v2)

As a System,
I want to generate translated audio using the XTTS v2 model,
So that I can produce high-fidelity synthetic voice.

**Acceptance Criteria:**

**Given** a translated text segment and a voice reference
**When** the Speaker Cloner worker processes the request
**Then** it generates an AI voice output in the target language
**And** the output maintains the speaker's original emotional tone and characteristics
**And** the audio file is saved to S3.

### Story 3.2: Voice Profile Management (Voice Signature)

As a System,
I want to extract a 5-second sample of the original voice,
So that I can create a reusable voice signature for cloning.

**Acceptance Criteria:**

**Given** the original audio track
**When** the pipeline starts the cloning phase
**Then** it automatically extracts a clear 5-second segment of speech
**And** it saves this 'voice signature' as a reference object in the job's S3 folder.

### Story 3.3: Audio/Visual Sync Engine (Time-Stretching)

As a System,
I want to adjust the speed of translated audio segments to exactly match the original visual sequence duration,
So that I can avoid lip-sync or timing drift.

**Acceptance Criteria:**

**Given** a generated audio segment and its original timestamp boundaries
**When** the audio duration differs from the original sequence duration
**Then** the sync engine applies time-stretching (without pitch shift) to align the two
**And** the resulting audio segment is frame-accurate with the visual keyframes.

## Epic 4: Visual Cleaning & Multi-format Export

Cette étape assure que le produit final est non seulement audible, mais aussi visuellement impeccable et prêt pour une utilisation en entreprise.

### Story 4.1: Visual Cleaner Worker (Inpainting)

As a System,
I want to remove source texts (subtitles, banners) from the original image using AI,
So that I can prepare a clean video for new content overlays.

**Acceptance Criteria:**

**Given** a video frame with text elements
**When** the Visual Cleaner worker processes the frame
**Then** it uses AI inpainting (e.g., SD/ControlNet) to remove the text and reconstruct the background
**And** the resulting image is saved as a 'cleaned' asset in S3.

### Story 4.2: Video Re-composition Engine (FFmpeg)

As a System,
I want to assemble the cleaned video with the new localized audio track,
So that I can generate the final MP4 file.

**Acceptance Criteria:**

**Given** the localized audio segments and cleaned visual frames
**When** the re-composition engine runs
**Then** it mixes the audio and video into a single MP4 file using FFmpeg
**And** the output uses H.264 codec and maintains original resolution.

### Story 4.3: Editable PPTX Generator

As a User (Alex),
I want to receive a PowerPoint presentation containing the keyframes and translated texts,
So that I can reuse the content in other materials.

**Acceptance Criteria:**

**Given** the localized transcription and keyframes
**When** the export phase completes
**Then** an editable `.pptx` file is generated
**And** each slide contains a keyframe image and its corresponding translated text.

### Story 4.4: Final Download Dashboard & Delivery

As a User,
I want to download my final deliverables (MP4 and PPTX) from a dedicated interface,
So that I can complete my localization project.

**Acceptance Criteria:**

**Given** a job in 'completed' status
**When** I access the download section on the dashboard
**Then** I can see links to download the final MP4 and PPTX files
**And** the links are secure and temporary (S3 presigned URLs).

## Epic 5: GPU Scaling & Data Lifecycle

Cet Epic garantit que Keryx est non seulement performant, mais aussi exemplaire en termes de gestion des ressources et de protection de la vie privée.

### Story 5.1: WorkerGuard Integration for GPU Scaling

As a System,
I want to dynamically adjust the number of AI workers based on demand and available VRAM,
So that I can ensure the pipeline stays fluid without wasting resources.

**Acceptance Criteria:**

**Given** a growing job queue
**When** VRAM availability allows
**Then** WorkerGuard scales the number of active worker pods in Kubernetes
**And** critical jobs are prioritized during GPU saturation.

### Story 5.2: Automatic 24h Biometric Data Purge

As a Security Officer,
I want all voice signatures and temporary files to be automatically deleted 24 hours after job completion,
So that Keryx's 'Privacy by Design' promise is fulfilled.

**Acceptance Criteria:**

**Given** a job completed or failed more than 24 hours ago
**When** the lifecycle cleanup script runs
**Then** all associated biometric signatures and temporary media assets are permanently deleted from S3
**And** the action is recorded in the audit logs.

### Story 5.3: Performance Audit & 1:1 Ratio Validation

As a System,
I want to measure total processing time compared to the original video duration,
So that I can validate that we meet the target performance ratio (< 1:1).

**Acceptance Criteria:**

**Given** a processed job
**When** the pipeline completes
**Then** the total duration (ingestion to export) is recorded
**And** for a 10-minute video, the total processing time is verified to be under 10 minutes.
