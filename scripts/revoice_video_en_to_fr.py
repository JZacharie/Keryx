import os
import sys
import json
import asyncio
import tempfile
import torch
import requests
import time
from moviepy.editor import ImageClip, AudioFileClip, concatenate_videoclips
from transformers import pipeline

# Configuration
# GPT-SoVITS API expects these:
VOICE_CLONER_URL = "http://voice-cloner:9880"
REF_AUDIO_PATH = "/app/host/Joseph.wav"
# Reference text transcribed from Joseph.wav
PROMPT_TEXT = (
    "Aujourd'hui, j'explore de nouveaux horizons avec l'intelligence artificielle. "
    "Est-ce que tu te rends compte de la précision nécessaire ? "
    "Car chaque mot compte, chaque silence à porte de l'Oriès ? "
    "J'articule avec un soir pour que ma signature local soit parfaitement capturée. "
    "C'est un exercice facilement en espac ?"
)
PROMPT_LANG = "fr"
# NEW: Translate from English to French
MODEL_NAME = "Helsinki-NLP/opus-mt-en-fr"

async def generate_cloned_speech(text, output_path):
    """Calls the GPT-SoVITS API for voice cloning."""
    params = {
        "text": text,
        "text_lang": "fr",  # generating French audio
        "ref_audio_path": REF_AUDIO_PATH,
        "prompt_text": PROMPT_TEXT,
        "prompt_lang": PROMPT_LANG
    }

    try:
        response = requests.get(VOICE_CLONER_URL, params=params, timeout=120)
        if response.status_code == 200:
            with open(output_path, "wb") as f:
                f.write(response.content)
            return True
        else:
            print(f"Error: GPT-SoVITS API returned {response.status_code}: {response.text}")
            return False
    except Exception as e:
        print(f"Error calling GPT-SoVITS: {str(e)}")
        return False

def translate_segments(segments, translator):
    print("--- Phase 1: Translating segments (English to French) ---")
    for seg in segments:
        text = seg["text"].strip()
        if text:
            translated = translator(text)[0]['translation_text']
            seg["translated_text"] = translated
            print(f"EN: {text} -> FR: {translated}")
        else:
            seg["translated_text"] = ""
    return segments

async def revoice(manifest_path, keyframes_folder, output_path):
    with open(manifest_path, 'r') as f:
        manifest = json.load(f)

    segments = manifest["transcription"]["segments"]

    # 1. Translation
    print("Loading translation model...")
    device = 0 if torch.cuda.is_available() else -1
    translator = pipeline("translation", model=MODEL_NAME, device=device)
    segments = translate_segments(segments, translator)

    # 2. GPT-SoVITS Synthesis & Clips creation
    print("--- Phase 2: Generating speech with GPT-SoVITS and assembly clips ---")
    clips = []

    with tempfile.TemporaryDirectory() as tmp_dir:
        for i, seg in enumerate(segments):
            text = seg["translated_text"]
            if not text: continue

            if not seg["related_keyframes"]:
                print(f"Warning: No keyframe for segment {i}. Skipping.")
                continue

            kf_id = seg["related_keyframes"][0]
            kf_info = next((k for k in manifest["keyframes"] if k["id"] == kf_id), None)
            if not kf_info:
                continue

            img_path = os.path.join(keyframes_folder, kf_info["filename"])
            audio_path = os.path.join(tmp_dir, f"speech_{i}.wav")

            # Check if image was systematically deleted
            if not os.path.exists(img_path):
                print(f"Warning: Image {img_path} not found (likely deleted). Skipping segment {i}.")
                continue

            print(f"Cloning voice for segment {i} ({text[:30]}...)...")
            success = await generate_cloned_speech(text, audio_path)

            if not success:
                print(f"Warning: Failed to synthesize segment {i} with cloning. Skipping segment.")
                continue

            audio_clip = AudioFileClip(audio_path)
            img_clip = ImageClip(img_path).set_duration(audio_clip.duration).set_audio(audio_clip)
            clips.append(img_clip)

        # 3. Assembly
        print("--- Phase 3: Assembly ---")
        if not clips:
            print("Error: No clips generated.")
            return

        final_clip = concatenate_videoclips(clips, method="compose")
        final_clip.write_videofile(output_path, codec="libx264", audio_codec="aac", fps=24, threads=4)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 revoice_video_en_to_fr.py <manifest_json> <keyframes_folder> [output_mp4]")
        sys.exit(1)

    m_path = sys.argv[1]
    k_folder = sys.argv[2]
    out_path = sys.argv[3] if len(sys.argv) > 3 else "/app/outputs/Industrializing_AI_FR.mp4"

    asyncio.run(revoice(m_path, k_folder, out_path))
