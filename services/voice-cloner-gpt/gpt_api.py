import subprocess
import os

# Paths within the container
GSV_PATH = "/app"
SOVITS_WEIGHTS = "/app/GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth"
GPT_WEIGHTS = "/app/GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt"
REF_WAV = "/app/reference/Mon_enregistrement_1.wav"
REF_TEXT = "Aujourd'hui, j'explore de nouveaux horizons avec l'intelligence artificielle. Est-ce que tu te rends compte de la précision nécessaire ? Chaque mot compte, chaque silence apporte du relief. J'articule avec soi pour que ma signature vocale soit parfaitement capturée. C'est un exercice fascinant ? N'est-ce pas ?"
REF_LANG = "fr"

def run_api():
    port = os.getenv("PORT", "9880")
    cmd = [
        "python", "api.py",
        "-s", SOVITS_WEIGHTS,
        "-g", GPT_WEIGHTS,
        "-dr", REF_WAV,
        "-dt", REF_TEXT,
        "-dl", REF_LANG,
        "-a", "0.0.0.0",
        "-p", port
    ]
    print(f"Launching GPT-SoVITS API v2 with weights: {SOVITS_WEIGHTS} on port {port}")
    subprocess.run(cmd, check=True)

if __name__ == "__main__":
    run_api()
