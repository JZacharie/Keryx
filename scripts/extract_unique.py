import os
import sys
import subprocess
import shutil
import tempfile
import re
import json
import torch
import numpy as np
from PIL import Image

try:
    import whisper
    WHISPER_AVAILABLE = True
except ImportError:
    WHISPER_AVAILABLE = False

import requests
DIFFUSION_ENGINE_URL = os.getenv("DIFFUSION_ENGINE_URL", "http://diffusion-engine:8000")

def clean_watermark_local(image_path):
    """Call Diffusion Engine to remove watermark from a local image file."""
    print(f"Removing watermark from {image_path}...")
    try:
        url = f"{DIFFUSION_ENGINE_URL}/clean_watermark"
        payload = {
            "image_url": image_path,
            "target_path": image_path
        }
        response = requests.post(url, json=payload, timeout=60)
        if response.status_code == 200:
            print(f"Cleaned {image_path} successfully.")
            return True
        else:
            print(f"Cleaning Error {response.status_code}: {response.text}")
            return False
    except Exception as e:
        print(f"Cleaning Exception: {str(e)}")
        return False


def calculate_diff(img1_path, img2_path):
    """Calculates the Mean Absolute Error between two images after resizing."""
    try:
        with Image.open(img1_path) as i1, Image.open(img2_path) as i2:
            arr1 = np.array(i1.convert('L').resize((64, 64))).astype(float)
            arr2 = np.array(i2.convert('L').resize((64, 64))).astype(float)
            return np.mean(np.abs(arr1 - arr2))
    except Exception as e:
        return float('inf')

def crop_to_target(image_path, target_w=1253, target_h=720):
    """Crops the image to match target resolution, usually 1253x720 from 1280x720 (removing 12 left, 15 right)."""
    try:
        with Image.open(image_path) as img:
            w, h = img.size
            if w == target_w and h == target_h:
                return
            
            print(f"Cropping {image_path} to {target_w}x{target_h} (12px left, 15px right)...")
            left = 12
            top = 0
            right = w - 15
            bottom = h
            
            img = img.crop((left, top, right, bottom))
            img.save(image_path)
    except Exception as e:
        print(f"Crop Exception for {image_path}: {e}")

def process_video(video_path, output_folder, scene_threshold=0.01, dedup_threshold=10.0, run_asr=True):
    if not os.path.exists(video_path):
        print(f"Error: Video not found at {video_path}")
        return

    # 1. Prepare output folder
    if os.path.exists(output_folder):
        shutil.rmtree(output_folder)
    os.makedirs(output_folder, exist_ok=True)

    # 2. Extract keyframes with timestamps
    with tempfile.TemporaryDirectory() as tmp_dir:
        print(f"--- Phase 1: Extracting candidates from {video_path} ---")

        # We always force the inclusion of the first frame (pts 0)
        ffmpeg_cmd = [
            "ffmpeg", "-y", "-i", video_path,
            "-vf", f"select='eq(n,0)+gt(scene,{scene_threshold})',showinfo",
            "-vsync", "vfr",
            os.path.join(tmp_dir, "frame_%04d.jpg")
        ]

        process = subprocess.Popen(ffmpeg_cmd, stderr=subprocess.PIPE, stdout=subprocess.DEVNULL)
        timestamps = []
        pts_time_pattern = re.compile(r"pts_time:([\d\.]+)")

        while True:
            line = process.stderr.readline().decode('utf-8')
            if not line:
                break
            if "pts_time" in line:
                match = pts_time_pattern.search(line)
                if match:
                    timestamps.append(float(match.group(1)))
        process.wait()

        candidate_files = sorted([f for f in os.listdir(tmp_dir) if f.endswith('.jpg')])
        if not candidate_files:
            print("No frames extracted by FFmpeg.")
            return

        if len(timestamps) < len(candidate_files):
            timestamps.extend([0.0] * (len(candidate_files) - len(timestamps)))

        print(f"Found {len(candidate_files)} candidates. Deduplicating...")

        keyframes = []
        last_kept_path = None

        for i, filename in enumerate(candidate_files):
            candidate_path = os.path.join(tmp_dir, filename)
            timestamp = timestamps[i]

            is_different = True
            if last_kept_path is not None:
                diff = calculate_diff(last_kept_path, candidate_path)
                if diff < dedup_threshold:
                    is_different = False

            if is_different:
                k_id = len(keyframes) + 1
                final_name = f"key_{k_id:04d}_ts_{timestamp:.3f}.jpg"
                final_path = os.path.join(output_folder, final_name)

                shutil.copy(candidate_path, final_path)

                # Crop to goal resolution (1257x720) before watermark cleaning
                crop_to_target(final_path)

                # Help with clean watermark
                clean_watermark_local(final_path)

                last_kept_path = final_path

                keyframes.append({
                    "id": k_id,
                    "filename": final_name,
                    "timestamp": timestamp
                })

        print(f"--- Phase 2: Audio extraction and ASR (Whisper) ---")
        if run_asr and WHISPER_AVAILABLE:
            audio_path = os.path.join(tmp_dir, "audio.mp3")
            print("Extracting audio...")
            subprocess.run([
                "ffmpeg", "-y", "-i", video_path,
                "-vn", "-acodec", "libmp3lame", "-q:a", "2",
                audio_path
            ], check=True, capture_output=True)

            print("Loading Whisper model (base)...")
            device = "cuda" if torch.cuda.is_available() else "cpu"
            model = whisper.load_model("base", device=device)
            print(f"Transcribing {audio_path} on {device}...")
            result = model.transcribe(audio_path)
            segments = result["segments"]

            # Sync text with keyframes
            for seg in segments:
                s_start = seg["start"]
                s_end = seg["end"]

                # Rule 1: Keyframes that appear DURING the segment
                during = [k["id"] for k in keyframes if s_start <= k["timestamp"] <= s_end]

                # Rule 2: The keyframe that was already there at s_start
                before = [k["id"] for k in keyframes if k["timestamp"] <= s_start]
                initial_k = [before[-1]] if before else []

                related = []
                for kid in (initial_k + during):
                    if kid not in related:
                        related.append(kid)

                seg["related_keyframes"] = related

            output_manifest = {
                "video": os.path.basename(video_path),
                "keyframes": keyframes,
                "transcription": {
                    "text": result["text"],
                    "segments": segments
                }
            }
        else:
            if run_asr:
                print("Warning: Whisper library not found. Skipping ASR.")
            output_manifest = {
                "video": os.path.basename(video_path),
                "keyframes": keyframes
            }

        # Save manifest.json
        with open(os.path.join(output_folder, "manifest.json"), "w") as f:
            json.dump(output_manifest, f, indent=2)

        print(f"--- Success! ---")
        print(f"Final Report: {len(keyframes)} unique frames and transcription saved to {output_folder}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 extract_unique.py <video_path> [output_folder] [dedup_threshold]")
        sys.exit(1)

    video = sys.argv[1]
    output = sys.argv[2] if len(sys.argv) > 2 else "/app/outputs/keyframes"
    threshold = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0

    process_video(video, output, dedup_threshold=threshold)
