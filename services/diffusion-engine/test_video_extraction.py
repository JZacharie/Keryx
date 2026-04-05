import cv2
import os
import subprocess
import tempfile
import sys

def test_extraction():
    print("Pre-test: creating dummy video with ffmpeg...")
    # Get a dummy frame from the repo (e.g. frame_0004.jpg in root)
    dummy_img = "../../frame_0004.jpg"
    if not os.path.exists(dummy_img):
        # try another path if needed
        dummy_img = "frame_0007_canny.png"

    if not os.path.exists(dummy_img):
        print(f"Error: Could not find any dummy image to test with.")
        return

    # Create a 2 second video at 24 fps using this image
    with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as tmp:
        video_path = tmp.name

    try:
        # Generate video: 24 frames of the dummy image
        cmd = [
            "ffmpeg", "-y", "-loop", "1", "-i", dummy_img,
            "-c:v", "libx264", "-t", "1", "-pix_fmt", "yuv420p", "-vf", "fps=24",
            video_path
        ]
        subprocess.run(cmd, check=True, capture_output=True)
        print(f"Dummy video created at {video_path}")

        # Now test OpenCV extraction
        print(f"Testing OpenCV extraction for {video_path}...")
        cap = cv2.VideoCapture(video_path)
        if not cap.isOpened():
            print("Error: OpenCV VideoCapture could not open the video file.")
            sys.exit(1)

        fps = cap.get(cv2.CAP_PROP_FPS)
        total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
        print(f"Propertes: FPS={fps}, Total Frames={total_frames}")

        frame_count = 0
        while cap.isOpened():
            ret, frame = cap.read()
            if not ret:
                break
            frame_count += 1
            if frame_count % 10 == 0:
                print(f"Extracted frame {frame_count}...")

        cap.release()
        print(f"Test complete. Successfully extracted {frame_count} frames.")

        if frame_count == 0:
            print("Error: 0 frames extracted.")
            sys.exit(1)

    finally:
        if os.path.exists(video_path):
            os.remove(video_path)

if __name__ == "__main__":
    test_extraction()
