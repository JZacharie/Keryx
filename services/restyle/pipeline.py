import os
import re
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import numpy as np
import easyocr

_reader = None

def get_reader():
    global _reader
    if _reader is None:
        _reader = easyocr.Reader(['fr', 'en'], gpu=False)
    return _reader

def extract_content(image: Image.Image) -> dict:
    reader = get_reader()
    img_array = np.array(image.convert("RGB"))
    results = reader.readtext(img_array)

    texts = []
    boxes = []
    full_text_parts = []

    for bbox, text, confidence in results:
        if confidence < 0.3:
            continue
        texts.append(text)
        boxes.append(bbox)
        full_text_parts.append(text)

    return {
        "texts": texts,
        "boxes": boxes,
        "full_text": "\n".join(full_text_parts),
    }


def rewrite_text(text: str, style_prompt: str = "", api_key: str = "") -> str:
    import google.generativeai as genai

    genai.configure(api_key=api_key or os.getenv("GEMINI_API_KEY"))
    model = genai.GenerativeModel("gemini-2.0-flash")

    prompt = (
        f"Reformule le texte suivant d'une slide de présentation "
        f"pour le rendre plus personnel, dynamique et percutant.\n"
        f"Consigne de style : {style_prompt or 'Rends-le plus engageant'}\n\n"
        f"---\n{text}\n---\n"
        f"Retourne UNIQUEMENT le texte reformulé, sans introduction ni conclusion."
    )

    resp = model.generate_content(prompt)
    return resp.text.strip()


def _wrap_text(text: str, font, max_width: int) -> list[str]:
    words = text.split()
    lines = []
    current = ""
    for word in words:
        test = f"{current} {word}".strip()
        bbox = font.getbbox(test)
        w = bbox[2] - bbox[0] if bbox else 0
        if w <= max_width:
            current = test
        else:
            lines.append(current)
            current = word
    if current:
        lines.append(current)
    return lines


def generate_new_slide(
    original: Image.Image,
    new_text: str,
    logo_bbox=None,
    font_path: str = "",
    background_template: Image.Image = None,
) -> Image.Image:
    if background_template:
        canvas = background_template.copy().resize(original.size, Image.LANCZOS)
    else:
        canvas = Image.new("RGB", original.size, (255, 255, 255))

    draw = ImageDraw.Draw(canvas)

    if logo_bbox:
        x1, y1 = int(logo_bbox[0][0]), int(logo_bbox[0][1])
        x2, y2 = int(logo_bbox[2][0]), int(logo_bbox[2][1])
        logo = original.crop((x1, y1, x2, y2))
        canvas.paste(logo, (x1, y1))

    font_size = 36
    try:
        font = ImageFont.truetype(font_path or "arial.ttf", font_size)
    except Exception:
        font = ImageFont.load_default()

    margin = 60
    y_position = original.height // 3
    for line in new_text.split("\n"):
        for wrapped in _wrap_text(line, font, original.width - 2 * margin):
            draw.text((margin, y_position), wrapped, fill=(30, 30, 30), font=font)
            y_position += font_size + 10

    return canvas


def execution_complete(input_img, style_prompt="", gemini_api_key=""):
    content = extract_content(input_img)
    original_text = content["full_text"]

    if not original_text.strip():
        return input_img, "(aucun texte détecté)"

    new_text = rewrite_text(original_text, style_prompt, gemini_api_key)

    logo_box = None
    if content["boxes"]:
        boxes_sorted = sorted(content["boxes"], key=lambda b: b[0][0] + b[0][1])
        logo_box = boxes_sorted[0]

    templates_dir = os.path.join(os.path.dirname(__file__), "templates")
    default_template = os.path.join(templates_dir, "default.png")
    bg = None
    if os.path.exists(default_template):
        bg = Image.open(default_template)

    output = generate_new_slide(input_img, new_text, logo_bbox=logo_box, background_template=bg)
    return output, new_text
