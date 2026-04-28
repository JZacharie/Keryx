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
