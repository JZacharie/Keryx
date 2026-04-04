# 🏛️ Keryx

Named after the Ancient Greek herald (κῆρυξ), the inviolable messenger of truth. **Keryx** is an automated, event-driven pipeline designed to convert technical presentation videos into localized and re-stylized versions.

The system ensures that complex technical content remains accurate while the visual aesthetic and the speaker's voice are preserved and adapted to any target language.

## 🎯 Objective
Automate the end-to-end localization of YouTube presentation videos, including:
- **Slide Analysis**: Frame-accurate detection of slide transitions.
- **Audio Transcription**: High-fidelity STT using **Faster-Whisper**.
- **Contextual Translation**: Preservation of technical terms via **Ollama (Llama 3)**.
- **Visual Stylization**: Slide regeneration using **Stable Diffusion (ControlNet)**.
- **Voice Cloning**: Localized audio generation preserving the speaker's original voice.

## 🏗️ System Architecture
Keryx operates on a Kubernetes cluster using an event-driven pattern for asynchronous processing and S3-compatible storage for asset persistence.

### Components
1. **`keryx-ingestor` (Rust/Axum)**:
   - Downloads videos via `yt-dlp`.
   - Detects slide transitions using `ffmpeg` scene detection.
   - Orchestrates STT and Translation steps.
2. **`keryx-speech-to-text`**: Integrated with existing **Faster-Whisper** services.
3. **`keryx-translator-llm`**: Integrated with existing **Ollama** services.
4. **`keryx-diffusion-engine`** (Planned): **ComfyUI API / SDXL** for visual stylization.
5. **`keryx-voice-cloner`** (Planned): **Coqui XTTS v2** for speaker voice preservation.
6. **`keryx-video-composer`** (Planned): **MoviePy** for final assembly and time-stretching.

## 🚀 Getting Started

### Prerequisites
- **Rust** (1.75+)
- **Redis**
- **MinIO** or S3-compatible storage
- **FFmpeg** & **yt-dlp**
- Access to **Faster-Whisper** and **Ollama** endpoints

### Environment Variables
Configure the following in your environment or `.env` file:
```bash
REDIS_URL=redis://localhost:6379
S3_BUCKET=keryx-raw
S3_REGION=eu-west-1
S3_ENDPOINT=http://minio:9000
```

### Running the Ingestor
```bash
cd services/ingestor
cargo run --release
```

## 📜 Repository Structure
```
.
├── services/
│   ├── ingestor/         # Rust (Axum) Ingestion service
│   ├── speech-to-text/   # (External) Faster-Whisper
│   └── translator-llm/   # (External) Ollama
├── libs/                 # Shared logic
├── deploy/               # Kubernetes manifests
└── implementation_plan_phase_1.md
```

## 🚦 License
This project is licensed under the MIT License - see the LICENSE file for details.
