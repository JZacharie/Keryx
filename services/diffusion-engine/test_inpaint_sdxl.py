import torch
from PIL import Image
import numpy as np
import cv2
from diffusers import ControlNetModel, StableDiffusionXLControlNetInpaintPipeline
from diffusers.utils import load_image
import os

# 1. Configuration du modèle ControlNet (pour la structure)
def get_pipe():
    # On utilise float16 pour économiser de la VRAM (nécessite un GPU NVIDIA récent)
    print("Chargement du modèle ControlNet...")
    controlnet = ControlNetModel.from_pretrained(
        "diffusers/controlnet-canny-sdxl-1.0",
        torch_dtype=torch.float16
    )

    # 2. Chargement de la Pipeline Inpaint + ControlNet
    print("Chargement de la pipeline SDXL Inpaint + ControlNet...")
    pipe = StableDiffusionXLControlNetInpaintPipeline.from_pretrained(
        "stabilityai/stable-diffusion-xl-base-1.0",
        controlnet=controlnet,
        torch_dtype=torch.float16
    )

    # Préférer CUDA si disponible
    if torch.cuda.is_available():
        pipe = pipe.to("cuda")
        print("Pipeline chargée sur CUDA")
    else:
        print("CUDA non disponible, utilisation du CPU (attention: très lent)")
        pipe = pipe.to("cpu")

    return pipe

def get_canny_filter(image):
    # Convert PIL Image to numpy array
    image_array = np.array(image)

    # Canny edge detection
    low_threshold = 100
    high_threshold = 200
    edges = cv2.Canny(image_array, low_threshold, high_threshold)

    # Convert grayscale edges back to RGB PIL Image
    # ControlNet expects the same shape/channels as the input
    edges = edges[:, :, None]
    edges = np.concatenate([edges, edges, edges], axis=2)
    return Image.fromarray(edges)

def process_restaurant_image(pipe, image_path, mask_path, prompt):
    # Chargement des images (Depuis ton storage ou local)
    print(f"Chargement de l'image: {image_path}")
    init_image = load_image(image_path).convert("RGB")

    print(f"Chargement du masque: {mask_path}")
    mask_image = load_image(mask_path).convert("RGB")

    # Redimensionnement pour plus de rapidité/mémoire (SDXL préfère 1024x1024)
    init_image = init_image.resize((1024, 1024))
    mask_image = mask_image.resize((1024, 1024))

    # Création de la carte de structure (Canny)
    print("Génération de la carte Canny...")
    control_image = get_canny_filter(init_image)

    # 3. Génération
    print(f"Lancement de la génération avec le prompt: '{prompt}'")
    # Utilisation d'un générateur pour la reproductibilité (optionnel)
    generator = torch.Generator(device=pipe.device).manual_seed(42)

    image = pipe(
        prompt=prompt,
        negative_prompt="low quality, blurry, distorted text, ugly, messy",
        image=init_image,
        mask_image=mask_image,
        control_image=control_image,
        strength=0.9, # Importance du changement dans le masque
        controlnet_conditioning_scale=0.5, # Rigidité de la structure
        generator=generator,
        num_inference_steps=30, # Ajustable selon les besoins
    ).images[0]

    return image, control_image

if __name__ == "__main__":
    # Paramètres de test
    # On peut utiliser des URLs d'exemple si les fichiers n'existent pas encore localement
    IMAGE_URL = "https://huggingface.co/datasets/huggingface/documentation-images/resolve/main/diffusers/controlnet_canny_source.png"
    # Créons un masque bidon (ex: un carré noir au milieu sur fond blanc)
    # Dans un vrai usage, le masque délimite la zone de changement

    # Chemin local si possible
    TEST_IMAGE = "../../frame_0004.jpg" if os.path.exists("../../frame_0004.jpg") else IMAGE_URL

    # Création d'un masque de test simple (ex: une zone rectangulaire)
    print("Création d'un masque de test...")
    dummy_mask = Image.new("RGB", (1024, 1024), (0, 0, 0)) # Noir = Pas de changement ?
    # En Inpainting diffusers: blanc (255, 255, 255) est la zone à repeindre
    from PIL import ImageDraw
    draw = ImageDraw.Draw(dummy_mask)
    draw.rectangle([300, 300, 700, 700], fill=(255, 255, 255))
    dummy_mask.save("test_mask.png")

    try:
        pipeline = get_pipe()
        result_image, canny_map = process_restaurant_image(
            pipeline,
            TEST_IMAGE,
            "test_mask.png",
            "A modern futuristic restaurant interior, cinematic lighting, 8k"
        )

        # Sauvegarde des résultats
        result_image.save("result_inpaint.png")
        canny_map.save("canny_structure.png")
        print("Succès ! Images sauvegardées: result_inpaint.png, canny_structure.png")

    except Exception as e:
        print(f"Erreur lors du test: {e}")
