# 🏛️ Keryx - Documentation du Pipeline de Traitement Vidéo

Keryx est un système automatisé de localisation de vidéos techniques, capable de transformer une présentation (ex: YouTube) en une version localisée (audio + visuel) tout en préservant l'esthétique et la voix d'origine.

## 🔄 Flux de Travail Global

Le traitement suit une architecture hexagonale pilotée par le service `ingestor` (Rust/Axum) qui coordonne plusieurs moteurs d'IA spécialisés.

### Phase 0 : Orchestration & Mise à l'Échelle
Avant de commencer, le système effectue un "Warm-up" de l'infrastructure :
*   **Scale-up automatique** des services GPU via Kubernetes (Diffusion Engine, Whisper, Ollama, TTS, Voice Cloner).
*   **Health Checks** : Attente du signal "Ready" sur tous les endpoints API.

### Phase 1 : Ingestion des Assets
*   **Téléchargement** : Récupération de la vidéo et de l'audio via `yt-dlp`.
*   **Stockage Temporaire** : Les fichiers sont segmentés et stockés sur un stockage compatible S3 (**MinIO**).

### Phase 2 : Analyse Sémantique (STT)
*   **Transcription** : Utilisation de **Faster-Whisper** pour générer un script horodaté précis.
*   **Alignement** : Calcul de la durée totale et préparation de la matrice de segments pour la traduction.

### Phase 3 : Extraction Visuelle et Nettoyage
C'est ici que les diapositives (keyframes) sont isolées et nettoyées.
1.  **Scene Detection** : `ffmpeg` détecte les changements de plans pour extraire les images clés uniques.
2.  **Suppression des Watermarks** : 
    *   **Option NotebookLM** : Utilisation de `https://notebooklmstudio.com/` pour supprimer les watermarks si les ressources locales sont saturées ou pour des résultats spécifiques.
    *   **Fallback Local (Recommandé)** : Le **Keryx Diffusion Engine** intègre un algorithme spécifique (`remove_notebooklm_watermark`) qui utilise :
        *   Une ROI (Region of Interest) ciblée en bas à droite.
        *   Une détection par différence de flou médian (**Median Blur Difference**).
        *   Une reconstruction par inpainting (**OpenCV Telea**).

### Phase 4 : Transformation Linguistique
*   **Traduction Contextuelle** : Passage du texte vers la langue cible (ex: FR) via **Ollama (Llama 3)**. 
*   **Conservation Technique** : Le moteur est instruit pour préserver les termes techniques critiques.

### Phase 5 : Synthèse Vocale (TTS & Voice Cloning)
*   **Génération Audio** : Création des pistes vocales synchronisées.
    *   Version standard via **Qwen-TTS**.
    *   Version clone de voix via **Coqui XTTS v2** pour retrouver le timbre original de l'orateur.

### Phase 6 : Reconstruction Vidéo
*   **Assemblage temporel** : Utilisation de `MoviePy` pour étirer ou compresser les diapositives nettoyées afin de matcher la durée de la nouvelle narration audio.
*   **Composition Finale** : Fusion des pistes audio localisées avec la vidéo reconstruite.

### Phase 7 : Livraison & Export
*   **Multi-Exports** : Production de versions EN (originale nettoyée), FR (TTS), et FR (Voice Cloned).
*   **PPTX Builder** : Génération d'un fichier PowerPoint éditable à partir des diapositives extraites et nettoyées.
*   **Notification** : Envoi des liens de téléchargement vers **Slack**.

---

## 🛠️ Focus : Suppression des Watermarks

Le pipeline privilégie la solution locale pour la robustesse et la confidentialité, mais peut intégrer des services externes.

### Algorithme Local (Diffusion Engine)
Le module Rust appelle l'endpoint `/clean_watermark` du moteur de diffusion : 
```python
# Extrait du code de traitement local (diffusion-engine/main.py)
def remove_notebooklm_watermark(image):
    # 1. Définit la zone de recherche (Bottom-Right)
    # 2. Construit un masque précis via différence de flou médian
    # 3. Dilate le masque pour capturer l'anti-aliasing
    # 4. Inpainting de la zone avec l'algorithme Telea
    return cleaned_image
```

> [!TIP]
> **NotebookLM Studio** : Utiliser cette solution uniquement pour la suppression des watermarks en complément de la solution locale si nécessaire.
