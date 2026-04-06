import os
import sys
import json
import torch
import whisper
import cv2
import ffmpeg
from moviepy.editor import VideoFileClip, AudioFileClip, ImageClip, concatenate_videoclips
from deep_translator import GoogleTranslator
from tqdm import tqdm
import requests
import tempfile
import time

# Host address of the voice-cloner service (GPT-SoVITS)
VOICE_CLONER_URL = os.getenv("VOICE_CLONER_URL", "http://voice-cloner:9880")
DIFFUSION_ENGINE_URL = os.getenv("DIFFUSION_ENGINE_URL", "http://diffusion-engine:8000")
REF_AUDIO_PATH = "/app/reference/Mon_enregistrement_1.wav"

def clean_watermark_local(image_path):
    """Call Diffusion Engine to remove watermark from a local image file."""
    print(f"Removing watermark from {image_path}...")
    try:
        url = f"{DIFFUSION_ENGINE_URL}/clean_watermark"
        # We pass the local path because both containers share the /app/outputs volume
        payload = {
            "image_url": image_path,
            "target_path": image_path # Overwrite original
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


def extract_audio(video_path, audio_path):
    print(f"Extractive audio from {video_path}...")
    video = VideoFileClip(video_path)
    video.audio.write_audiofile(audio_path, logger=None)
    return video.duration

def transcribe_audio(audio_path, language=None):
    print(f"Transcribing {audio_path}...")
    model = whisper.load_model("base", device="cuda" if torch.cuda.is_available() else "cpu")
    result = model.transcribe(audio_path, language=language, verbose=False)
    return result

def translate_transcript(transcript, dest_lang='fr'):
    print(f"Translating to {dest_lang}...")
    translator = GoogleTranslator(source='auto', target=dest_lang)
    translated_segments = []

    for segment in tqdm(transcript['segments']):
        original_text = segment['text']
        # GoogleTranslator can sometimes fail on very long strings or rate limit
        # but for this script it should be fine.
        try:
            translated_text = translator.translate(original_text)
        except:
            translated_text = original_text # fallback

        seg_copy = segment.copy()
        seg_copy["original_text"] = original_text
        seg_copy["translated_text"] = translated_text
        translated_segments.append(seg_copy)
    return translated_segments

def extract_keyframes(video_path, output_folder, segments):
    print(f"Extracting keyframes to {output_folder}...")
    os.makedirs(output_folder, exist_ok=True)
    video = cv2.VideoCapture(video_path)
    fps = video.get(cv2.CAP_PROP_FPS)

    for i, segment in enumerate(segments):
        # Extract frame at the middle of the segment
        target_time = (segment['start'] + segment['end']) / 2
        frame_idx = int(target_time * fps)
        video.set(cv2.CAP_PROP_POS_FRAMES, frame_idx)
        ret, frame = video.read()
        if ret:
            filename = f"key_{i:04d}.jpg"
            img_path = os.path.join(output_folder, filename)
            cv2.imwrite(img_path, frame)

            # Clean watermark only if it's not the first or last image
            if i > 0 and i < len(segments) - 1:
                clean_watermark_local(img_path)

            segment['keyframe'] = filename
    video.release()

def generate_tts(text, lang, output_path):
    """Call GPT-SoVITS API."""
    # Standard GPT-SoVITS FAST-API parameters
    # Note: lang should be 'zh', 'en', 'ja', 'fr', 'ko' etc.
    params = {
        "text": text,
        "text_lang": lang,
        "ref_audio_path": REF_AUDIO_PATH,
        "prompt_text": "Aujourd'hui, j'explore de nouveaux horizons avec l'intelligence artificielle. Est-ce que tu te rends compte de la précision nécessaire ? Chaque mot compte, chaque silence apporte du relief. J'articule avec soi pour que ma signature vocale soit parfaitement capturée. C'est un exercice fascinant ? N'est-ce pas ?",
        "prompt_lang": "fr"
    }

    # Retry logic for service availability
    max_retries = 10
    retry_delay = 10
    
    for attempt in range(max_retries):
        try:
            # Assuming the voice-cloner has a /tts or similar endpoint
            response = requests.get(VOICE_CLONER_URL, params=params, timeout=300)
            if response.status_code == 200:
                with open(output_path, "wb") as f:
                    f.write(response.content)
                return True
            else:
                print(f"TTS Error {response.status_code} (Attempt {attempt+1}/{max_retries}): {response.text}")
        except Exception as e:
            print(f"TTS Connection Attempt {attempt+1}/{max_retries} failed: {str(e)}")
        
        if attempt < max_retries - 1:
            time.sleep(retry_delay)
    
    return False

def assemble_video(segments, output_folder, tts_lang, final_video_path):
    print(f"Assembling video version: {tts_lang} -> {final_video_path}...")
    clips = []

    with tempfile.TemporaryDirectory() as tmp_dir:
        for i, segment in enumerate(tqdm(segments)):
            text = segment['translated_text'] if tts_lang == 'fr' else segment['original_text']
            audio_file = os.path.join(tmp_dir, f"audio_{i}.wav")

            # Generate TTS for segment
            success = generate_tts(text, tts_lang, audio_file)
            if not success:
                continue

            # Load audio to get duration
            audio_clip = AudioFileClip(audio_file)

            # Create image clip from keyframe
            img_path = os.path.join(output_folder, segment['keyframe'])
            img_clip = ImageClip(img_path).set_duration(audio_clip.duration).set_audio(audio_clip)
            clips.append(img_clip)

        if clips:
            final_clip = concatenate_videoclips(clips, method="compose")
            final_clip.write_videofile(final_video_path, codec="libx264", audio_codec="aac", fps=24, logger=None)
            print(f"Success! {final_video_path}")
        else:
            print("No clips generated.")

def main():
    video_path = "/app/host/Industrializing_AI.mp4"
    audio_path = "/app/outputs/audio/industrializing.wav"
    
    # We will use the deduplicated folder
    kf_folder_ai = "/app/outputs/keyframes_ai"
    manifest_ai = os.path.join(kf_folder_ai, "manifest.json")

    os.makedirs(os.path.dirname(audio_path), exist_ok=True)
    os.makedirs("/app/outputs/transcripts", exist_ok=True)
    os.makedirs("/app/outputs/revoiced", exist_ok=True)

    if os.path.exists(manifest_ai):
        print(f"Found deduplicated manifest at {manifest_ai}. Short-circuiting extraction!")
        with open(manifest_ai, 'r') as f:
            manifest = json.load(f)
        
        # Pull segments from manifest
        transcript = manifest["transcription"]
        
        # Step 3: Translate
        segments = translate_transcript(transcript, dest_lang='fr')
        
        # Map related_keyframes to the 'keyframe' attribute required by assemble_video
        for seg in segments:
            if not seg.get("related_keyframes"): continue
            kf_id = seg["related_keyframes"][0]
            kf_info = next((k for k in manifest["keyframes"] if k["id"] == kf_id), None)
            if kf_info:
                seg["keyframe"] = kf_info["filename"]
                
        # Clean watermarks on the deduplicated keyframes
        import glob
        files = sorted([f for f in os.listdir(kf_folder_ai) if f.endswith('.jpg') or f.endswith('.png')])
        for i, filename in enumerate(files):
            img_path = os.path.join(kf_folder_ai, filename)
            # Skip first image, systematically delete last image
            if i > 0 and i < len(files) - 1:
                clean_watermark_local(img_path)
            elif i == len(files) - 1:
                if os.path.exists(img_path):
                    os.remove(img_path)

        # Remove segments mapped to deleted files
        segments = [seg for seg in segments if 'keyframe' in seg and os.path.exists(os.path.join(kf_folder_ai, seg["keyframe"]))]

        # Now assemble video directly using the deduplicated keyframes
        assemble_video(segments, kf_folder_ai, 'fr', "/app/outputs/revoiced/industrializing_fr.mp4")
        return

    # Original Fallback behavior
    kf_folder = "/app/outputs/keyframes/industrializing/"
    os.makedirs(kf_folder, exist_ok=True)
    duration = extract_audio(video_path, audio_path)
    transcript = transcribe_audio(audio_path, language="en")
    segments = translate_transcript(transcript, dest_lang='fr')
    extract_keyframes(video_path, kf_folder, segments)
    
    with open("/app/outputs/transcripts/manifest.json", "w") as f:
        json.dump(segments, f, indent=2)

    assemble_video(segments, kf_folder, 'fr', "/app/outputs/revoiced/industrializing_fr.mp4")

if __name__ == "__main__":
    main()
