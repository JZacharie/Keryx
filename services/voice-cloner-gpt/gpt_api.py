import subprocess
import os

# Paths within the container
GSV_PATH = "/app"
SOVITS_WEIGHTS = "/app/pretrained_models/gsv-v2final-pretrained/s2G488k.pth"
GPT_WEIGHTS = "/app/pretrained_models/gsv-v2final-pretrained/s1v2.ckpt"
REF_WAV = "/app/reference/Joseph.wav"
REF_TEXT = "Aujourd'hui, j'explore de nouveaux horizons avec l'intelligence artificielle."
REF_LANG = "fr"

def run_api():
    cmd = [
        "python", "api.py",
        "-s", SOVITS_WEIGHTS,
        "-g", GPT_WEIGHTS,
        "-dr", REF_WAV,
        "-dt", REF_TEXT,
        "-dl", REF_LANG,
        "-v", "v2",
        "-a", "0.0.0.0",
        "-p", "9880"
    ]
    print(f"Launching GPT-SoVITS API v2 with weights: {SOVITS_WEIGHTS}")
    subprocess.run(cmd, check=True)

if __name__ == "__main__":
    run_api()
