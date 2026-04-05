import numpy as np
import cv2
from PIL import Image
import os

def get_canny_image(image: Image.Image) -> Image.Image:
    image_np = np.array(image)
    # Applying Canny as specified in the script
    image_np = cv2.Canny(image_np, 100, 200)
    image_np = image_np[:, :, None]
    image_np = np.concatenate([image_np, image_np, image_np], axis=2)
    return Image.fromarray(image_np)

if __name__ == "__main__":
    img_path = "../../frame_0004.jpg"
    if os.path.exists(img_path):
        img = Image.open(img_path).convert("RGB")
        canny_img = get_canny_image(img)
        canny_img.save("test_canny_result.png")
        print("Success! Canny version saved as test_canny_result.png")
    else:
        print(f"Error: {img_path} not found.")
