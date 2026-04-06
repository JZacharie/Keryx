import os
import sys
import json
import asyncio
import tempfile
import torch
import requests
from moviepy.editor import ImageClip, AudioFileClip, concatenate_videoclips
from transformers import pipeline

VOICE_CLONER_URL = "http://localhost:9880"
# In local dev from host, it's 9880. If inside docker it's http://voice-cloner:9880.
# Assuming I run this script from host, targetting the docker container mapping.
REF_AUDIO_PATH = "/app/host/Joseph.wav" # Path INSIDE the container
MODEL_NAME = "Helsinki-NLP/opus-mt-fr-en"

async def generate_cloned_speech(text, language, output_path):
    params = {
        "text": text,
        "language": language,
        "speaker_wav": REF_AUDIO_PATH
    }
    try:
        response = requests.get(VOICE_CLONER_URL, params=params, timeout=120)
        if response.status_code == 200:
            with open(output_path, "wb") as f:
                f.write(response.content)
            return True
        else:
            print(f"Error: API returned {response.status_code}: {response.text}")
            return False
    except Exception as e:
        print(f"Error calling voice-cloner: {str(e)}")
        return False

async def process_manifest(manifest_path, keyframes_folder, output_video, lang="fr", translate=False):
    print(f"--- Processing Video ({lang.upper()}, translate={translate}) ---")
    with open(manifest_path, 'r') as f:
        manifest = json.load(f)

    segments = manifest["transcription"]["segments"]

    if translate:
        print("Loading translation model...")
        device = 0 if torch.cuda.is_available() else -1
        translator = pipeline("translation", model=MODEL_NAME, device=device)
        for seg in segments:
            fr_text = seg["text"].strip()
            if fr_text:
                seg["processed_text"] = translator(fr_text)[0]['translation_text']
            else:
                seg["processed_text"] = ""
    else:
        for seg in segments:
            seg["processed_text"] = seg["text"].strip()

    clips = []
    with tempfile.TemporaryDirectory() as tmp_dir:
        # Limit to first 10 segments for speed in this demonstration
        for i, seg in enumerate(segments[:10]):
            text = seg["processed_text"]
            if not text: continue

            if not seg.get("related_keyframes"):
                continue

            kf_id = seg["related_keyframes"][0]
            kf_info = next((k for k in manifest["keyframes"] if k["id"] == kf_id), None)
            if not kf_info: continue

            img_path = os.path.join(keyframes_folder, kf_info["filename"])
            audio_path = os.path.join(tmp_dir, f"speech_{i}.wav")

            print(f"Synthesizing segment {i} ({lang})...")
            success = await generate_cloned_speech(text, lang, audio_path)

            if not success:
                continue

            audio_clip = AudioFileClip(audio_path)
            img_clip = ImageClip(img_path).set_duration(audio_clip.duration).set_audio(audio_clip)
            clips.append(img_clip)

        if not clips:
            print("No clips generated.")
            return

        final_clip = concatenate_videoclips(clips, method="compose")
        final_clip.write_videofile(output_video, codec="libx264", audio_codec="aac", fps=24)

async def main():
    manifest_path = "outputs/keyframes/manifest.json"
    keyframes_folder = "outputs/keyframes/"

    # Version en Français (avant traduction)
    await process_manifest(manifest_path, keyframes_folder, "outputs/video_fr.mp4", lang="fr", translate=False)

    # Version en Anglais (après traduction)
    await process_manifest(manifest_path, keyframes_folder, "outputs/video_en.mp4", lang="en", translate=True)

if __name__ == "__main__":
    asyncio.run(main())
