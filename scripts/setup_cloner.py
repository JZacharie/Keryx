import os
import subprocess

def download_models():
    """Helper to download GPT-SoVITS base models if they don't exist."""
    models_dir = "models"
    cache_dir = os.path.join(models_dir, "gpt-sovits")
    os.makedirs(cache_dir, exist_ok=True)

    # Official base models (V2)
    urls = {
        "s1v2.ckpt": "https://huggingface.co/lj1995/GPT-SoVITS/resolve/main/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch%3D12-step%3D369668.ckpt",
        "s2G488k.pth": "https://huggingface.co/lj1995/GPT-SoVITS/resolve/main/gsv-v2final-pretrained/s2G2333k.pth",
        "s2D488k.pth": "https://huggingface.co/lj1995/GPT-SoVITS/resolve/main/gsv-v2final-pretrained/s2D2333k.pth",
        "pytorch_model.bin": "https://huggingface.co/lj1995/GPT-SoVITS/resolve/main/chinese-roberta-wwm-ext-large/pytorch_model.bin",
        "config.json": "https://huggingface.co/lj1995/GPT-SoVITS/resolve/main/chinese-roberta-wwm-ext-large/config.json"

    }

    print("--- Setting up GPT-SoVITS models into .cache/gpt-sovits ---")
    for filename, url in urls.items():
        target = os.path.join(cache_dir, filename)
        if not os.path.exists(target):
            print(f"Downloading {filename}...")
            subprocess.run(["wget", "-O", target, url], check=True)
        else:
            print(f"{filename} already exists.")


if __name__ == "__main__":
    download_models()
