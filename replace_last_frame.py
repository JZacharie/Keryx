import glob
import os
from PIL import Image

folder = "/app/outputs/keyframes_ai"
files = sorted([f for f in glob.glob(f"{folder}/key_*.jpg")])
if not files:
    print("No keyframes found!")
    exit(1)

last_file = files[-1]
src_path = "/app/host/starops.png"
target_w, target_h = 1253, 720

print(f"Loading {src_path} to replace {last_file}...")
img = Image.open(src_path)
w, h = img.size

aspect_img = w / h
aspect_target = target_w / target_h

if aspect_img > aspect_target:
    new_h = target_h
    new_w = int(w * (target_h / h))
else:
    new_w = target_w
    new_h = int(h * (target_w / w))

img = img.resize((new_w, new_h), Image.LANCZOS)

left = int((new_w - target_w) / 2)
top = int((new_h - target_h) / 2)
right = left + target_w
bottom = top + target_h

img = img.crop((left, top, right, bottom))
img = img.convert('RGB')
img.save(last_file)
print(f"Replaced exactly {last_file}. New size: {img.size}")
