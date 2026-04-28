---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-02b-vision', 'step-02c-executive-summary', 'step-03-success']
inputDocuments:
  - '_bmad-output/project-context.md'
  - 'README.md'
  - 'KERYX_PROCESS.md'
  - 'TEST_PLAN.md'
  - 'CHANGELOG.md'
documentCounts:
  briefs: 0
  research: 0
  brainstorming: 0
  projectDocs: 5
classification:
  projectType: api_backend
  domain: scientific
  complexity: medium
  projectContext: brownfield
workflowType: 'prd'
---

# Product Requirements Document - Keryx

**Author:** autobot
**Date:** 2026-04-28

## Executive Summary

Keryx is an automated, event-driven video localization pipeline designed to transform technical YouTube presentations into localized versions while preserving the original speaker's vocal identity and the visual aesthetics of the slides. It addresses the high cost and complexity of manual localization for technical content, providing a seamless "one-click" transition from source URL to multilingual, high-fidelity exports. Target users include technical educators, developer advocates, and organizations seeking to globalize their knowledge base without losing the personal touch of their speakers.

### What Makes This Special

Keryx differentiates itself through its deep integration of generative AI models for both audio and visual preservation. Unlike traditional localization tools, Keryx utilizes Coqui XTTS v2 and GPT-SoVITS for precise voice cloning, ensuring the translated audio sounds like the original speaker. Visually, it uses ffmpeg-based scene detection to isolate keyframes, which are then cleaned and restylized via Stable Diffusion (ControlNet). The entire process is orchestrated by a high-performance Rust/Axum engine that manages distributed AI workers on a Kubernetes cluster, ensuring scalability, idempotency, and frame-accurate synchronization.

## Project Classification

- **Project Type:** API Backend (Media Processing & Orchestration Service)
- **Domain:** Scientific (AI/ML & Media Data Processing)
- **Complexity:** Medium (Distributed AI inference, GPU scaling, temporal synchronization)
- **Project Context:** Brownfield (Existing system with modular hexagonal architecture)

## Success Criteria

### User Success

- **Seamless Localization:** Users achieve a fully localized technical presentation where the original speaker's voice is preserved and synchronized with the slides, requiring zero manual intervention.
- **Speed to Market:** The end-to-end processing time for a standard 30-minute presentation is reduced from days (manual) to less than the video duration itself (automated).
- **Editability:** Users receive not only a video but also a fully editable localized PowerPoint (PPTX) file, enabling final adjustments if necessary.

### Business Success

- **Operational Efficiency:** Drastic reduction in localization costs by replacing human-in-the-loop translation and voice-over with an automated AI pipeline.
- **Scalability:** The system reliably handles a high volume of video processing requests (100+ per day) on the Kubernetes cluster without manual monitoring.
- **Knowledge Accessibility:** Increased globalization of technical training assets across multiple linguistic matrices (FR, ES, etc.).

### Technical Success

- **Inference Efficiency:** High cache hit rate in the idempotent S3/Redis layer, minimizing redundant calls to heavy LLM (Ollama/Llama 3) and STT (Whisper) services.
- **Temporal Accuracy:** 100% frame-accurate synchronization between detected slide transitions and localized audio segments.
- **Resource Management:** Optimized GPU utilization with automatic scale-up/scale-down via WorkerGuard patterns.

### Measurable Outcomes

- **Processing Latency:** Localization time < Video duration.
- **Transcription Quality:** Word Error Rate (WER) < 5% for technical content.
- **Pipeline Reliability:** Job success rate > 98% across all phases (Downloading to PPTX Generation).

## Product Scope

### MVP - Minimum Viable Product

- **Core Pipeline:** YouTube URL ingestion, Faster-Whisper transcription, LLM-based technical translation, and Coqui XTTS v2 voice cloning.
- **Visuals:** ffmpeg scene detection and basic watermark removal.
- **Outputs:** High-fidelity video exports (EN, FR) and localized PPTX files.
- **Infrastructure:** Rust orchestrator with hexagonal architecture and S3/Redis persistence.

### Growth Features (Post-MVP)

- **Multilingual Expansion:** Support for a wider range of target languages beyond FR and ES.
- **Advanced Visuals:** Improved AI-driven slide cleaning for complex backgrounds and watermarks.
- **Integration:** Real-time job notifications via Slack with direct download links.

### Vision (Future)

- **Cinematic Enhancements:** Integration of advanced Stable Video Diffusion (SVD) for dynamic slide intros.
- **Live Localization:** Low-latency pipeline for near real-time localization of live streams.
- **Interactive Feedback:** Web-based interface for fine-tuning transcription and translation before final composition.
