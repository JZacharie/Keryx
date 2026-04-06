import torch
from PIL import Image
import numpy as np
import cv2
from diffusers import ControlNetModel, StableDiffusionXLControlNetInpaintPipeline
from diffusers.utils import load_image
import os

# CONFIGURATION POUR UN RENDU RÉALISTE
PROMPT_REALISTE = "Photorealistic high-tech autonomous factory floor, cinematic lighting, modern industrial architecture, highly detailed, 8k, bokeh, professional photography, metallic textures, electronic components."
NEGATIVE_PROMPT_REALISTE = "cartoon, drawing, illustration, icon, vector art, text, watermark, blurry, sketch, flat colors, low resolution."

def get_pipe():
    print("Chargement des modèles avec support de rendu réaliste...")
    controlnet = ControlNetModel.from_pretrained(
        "diffusers/controlnet-canny-sdxl-1.0",
        torch_dtype=torch.float16
    )
    pipe = StableDiffusionXLControlNetInpaintPipeline.from_pretrained(
        "stabilityai/stable-diffusion-xl-base-1.0",
        controlnet=controlnet,
        torch_dtype=torch.float16
    )
    if torch.cuda.is_available():
        pipe = pipe.to("cuda")
    else:
        pipe = pipe.to("cpu")
    return pipe

def get_canny_filter(image):
    image_array = np.array(image)
    # On ajuste les seuils pour capturer plus ou moins de détails de la structure
    edges = cv2.Canny(image_array, 100, 200)
    edges = edges[:, :, None]
    edges = np.concatenate([edges, edges, edges], axis=2)
    return Image.fromarray(edges)

def process_realistic_transform(pipe, image_path, mask_path, prompt):
    print(f"Transformation réaliste de: {image_path}")
    init_image = load_image(image_path).convert("RGB").resize((1024, 1024))
    mask_image = load_image(mask_path).convert("RGB").resize((1024, 1024))

    # Structure Canny (ControlNet)
    control_image = get_canny_filter(init_image)

    # Génération
    # Pour un rendu réaliste à partir d'un schéma :
    # 1. On augmente num_inference_steps pour plus de détails.
    # 2. On baisse légèrement controlnet_conditioning_scale pour que l'IA puisse 'interpréter' les schémas comme des objets réels.
    # 3. On utilise un guidage fort (strength) pour remplacer le style 'dessin' par du 'réel'.

    image = pipe(
        prompt=prompt,
        negative_prompt=NEGATIVE_PROMPT_REALISTE,
        image=init_image,
        mask_image=mask_image,
        control_image=control_image,
        strength=0.95, # Remplacement quasi total dans le masque
        controlnet_conditioning_scale=0.6, # Flexibilité structurelle
        num_inference_steps=50,
    ).images[0]

    return image, control_image

if __name__ == "__main__":
    # Test avec frame_0007.jpg (Diagramme de production)
    IMAGE_REF = "../../frame_0007.jpg"

    # Création d'un masque de test (ex: toute l'image pour tout transformer, ou une zone)
    # Pour transformer tout le schéma en usine réaliste, on utilise un masque blanc
    mask = Image.new("RGB", (1024, 1024), (255, 255, 255))
    mask.save("transformation_mask.png")

    print("Paramètres de réalisme configurés.")
    # (Note: Le chargement réel du modèle nécessite un GPU)
    # L'utilisateur lancera ce script sur son serveur keryx.
