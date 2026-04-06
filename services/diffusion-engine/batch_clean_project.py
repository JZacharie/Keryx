import os
import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
import glob

# Configuration de la zone du watermark (Bas à droite)
WATERMARK_CORNER = {
    "x_start_ratio": 0.82,
    "y_start_ratio": 0.90,
    "x_end_ratio": 1.0,
    "y_end_ratio": 1.0
}

def clean_image_opencv(image_path, output_path):
    """Effectue un nettoyage localisé (inpaint) sans GPU via OpenCV."""
    print(f"Nettoyage Local (OpenCV) : {image_path}")
    img = cv2.imread(image_path)
    if img is None:
        print(f"Erreur : Impossible de lire {image_path}")
        return False

    h, w = img.shape[:2]

    # Création du masque binaire
    mask = np.zeros((h, w), dtype=np.uint8)
    x1, y1 = int(w * WATERMARK_CORNER["x_start_ratio"]), int(h * WATERMARK_CORNER["y_start_ratio"])
    x2, y2 = w, h
    cv2.rectangle(mask, (x1, y1), (x2, y2), 255, -1)

    # Inpainting (Algorithme Navier-Stokes)
    # 3 px de rayon pour le blend
    dst = cv2.inpaint(img, mask, 3, cv2.INPAINT_NS)

    cv2.imwrite(output_path, dst)
    return True

def run_batch():
    # 1. Création du dossier output
    output_dir = "../../output"
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
        print(f"Dossier {output_dir} créé.")

    # 2. Recherche des images dans la racine du projet
    image_patterns = ["../../frame_*.jpg", "../../frame_*.png"]
    images_found = []
    for pattern in image_patterns:
        images_found.extend(glob.glob(pattern))

    if not images_found:
        print("Aucune image frame_00xx trouvée à la racine.")
        return

    print(f"Nettoyage de {len(images_found)} image(s)...")

    # 3. Traitement de chaque image
    for img_path in images_found:
        filename = os.path.basename(img_path)
        name, ext = os.path.splitext(filename)
        output_name = f"{name}_PROPRE{ext}"
        output_path = os.path.join(output_dir, output_name)

        success = clean_image_opencv(img_path, output_path)
        if success:
            print(f"  -> Sauvegardé dans: {output_path}")

if __name__ == "__main__":
    run_batch()
