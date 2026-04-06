import os
import requests

DIFFUSION_ENGINE_URL = "http://diffusion-engine:8000"

def clean_watermark_local(image_path):
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

def clean_all():
    folder = "/app/outputs/keyframes_ai"
    if not os.path.exists(folder):
        print(f"Folder not found: {folder}")
        return

    files = sorted([f for f in os.listdir(folder) if f.endswith('.jpg') or f.endswith('.png')])
    if not files:
        print(f"No images found in {folder}.")
        return

    print(f"Found {len(files)} images to process in {folder}.")

    for i, filename in enumerate(files):
        # Skip first image
        if i == 0:
            print(f"Skipping first image: {filename}")
            continue

        # Systematic delete of last image
        if i == len(files) - 1:
            print(f"Systematically deleting last image: {filename}")
            file_path = os.path.join(folder, filename)
            os.remove(file_path)
            continue

        img_path = os.path.join(folder, filename)
        clean_watermark_local(img_path)

if __name__ == "__main__":
    clean_all()
