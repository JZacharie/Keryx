import os
import torch
from fastapi import FastAPI, HTTPException, Response
from pydantic import BaseModel
from TTS.api import TTS
import tempfile

app = FastAPI()

# Force agreement to Coqui non-commercial license for programmatic use
os.environ["COQUI_TOS_AGREED"] = "1"

# Initialize XTTS v2
device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"Loading XTTS v2 on {device}...")
tts = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to(device)

@app.get("/")
async def generate_tts(text: str, language: str = "en", speaker_wav: str = "/app/host/Joseph.wav"):
    if not os.path.exists(speaker_wav):
        raise HTTPException(status_code=404, detail=f"Speaker wav not found: {speaker_wav}")

    try:
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
            wav_path = tmp.name

        tts.tts_to_file(
            text=text,
            speaker_wav=speaker_wav,
            language=language,
            file_path=wav_path
        )

        with open(wav_path, "rb") as f:
            content = f.read()

        os.remove(wav_path)
        return Response(content=content, media_type="audio/wav")
    except Exception as e:
        print(f"TTS Error: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=9880)
