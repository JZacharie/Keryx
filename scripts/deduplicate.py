import os
import sys
import numpy as np
from PIL import Image

def calculate_diff(img1_path, img2_path):
    """Calculates the Mean Absolute Error between two images after resizing."""
    try:
        with Image.open(img1_path) as i1, Image.open(img2_path) as i2:
            arr1 = np.array(i1.convert('L').resize((64, 64))).astype(float)
            arr2 = np.array(i2.convert('L').resize((64, 64))).astype(float)
            return np.mean(np.abs(arr1 - arr2))
    except Exception as e:
        print(f"Error comparing {img1_path} and {img2_path}: {e}")
        return float('inf')

def deduplicate(folder, threshold=10.0):
    files = sorted([f for f in os.listdir(folder) if f.lower().endswith(('.jpg', '.jpeg', '.png'))])
    if not files:
        print("No image files found.")
        return

    print(f"Processing {len(files)} files in {folder}...")

    kept_count = 1
    removed_count = 0
    last_kept_file = files[0]

    for i in range(1, len(files)):
        current_file = files[i]
        path1 = os.path.join(folder, last_kept_file)
        path2 = os.path.join(folder, current_file)

        diff = calculate_diff(path1, path2)

        if diff >= threshold:
            # Keep this file
            last_kept_file = current_file
            kept_count += 1
        else:
            # Mark as duplicate and remove
            os.remove(path2)
            removed_count += 1

    print(f"Deduplication complete.")
    print(f"Kept: {kept_count}")
    print(f"Removed: {removed_count}")

if __name__ == "__main__":
    target_folder = sys.argv[1] if len(sys.argv) > 1 else "/app/outputs/keyframes"
    threshold = float(sys.argv[2]) if len(sys.argv) > 2 else 5.0
    deduplicate(target_folder, threshold)
