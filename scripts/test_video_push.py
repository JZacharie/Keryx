import requests
import json
import os

# Hostname used in docker-compose for the diffusion engine
URL = "http://localhost:8000/clean_video_watermark"
VIDEO_PATH = "/app/host/Industrializing_AI.mp4"
TARGET_PATH = "/app/outputs/revoiced/industrializing_cleaned_manual.mp4"

def test_video_push():
    print(f"--- Pushing video for watermark removal ---")
    print(f"Target: {URL}")
    print(f"Video: {VIDEO_PATH}")
    
    payload = {
        "video_url": VIDEO_PATH,
        "target_path": TARGET_PATH
    }
    
    try:
        response = requests.post(URL, json=payload, timeout=3600) # Long timeout for video processing
        if response.status_code == 200:
            print("Successfully pushed!")
            print(json.dumps(response.json(), indent=2))
        else:
            print(f"Error {response.status_code}: {response.text}")
    except Exception as e:
        print(f"Exception: {str(e)}")

if __name__ == "__main__":
    test_video_push()
