import pytest
from fastapi.testclient import TestClient
from api import app
import os
from unittest.mock import patch, MagicMock

client = TestClient(app)

def test_health_check():
    """Simple verification that the app starts."""
    # Since loading the model is done at module level, 
    # we might want to mock TTS if we want to run this without GPU/Models.
    pass

@patch("api.TTS")
def test_generate_tts_mocked(mock_tts_class):
    """Test the generation endpoint with mocked TTS to verify logic."""
    mock_tts_instance = MagicMock()
    mock_tts_class.return_value.to.return_value = mock_tts_instance
    
    # We need a dummy wav file for the test
    with open("dummy_speaker.wav", "wb") as f:
        f.write(b"dummy wav content")

    try:
        response = client.get("/", params={
            "text": "Hello world",
            "language": "en",
            "speaker_wav": "dummy_speaker.wav"
        })
        
        # We expect a success if everything is mocked correctly
        # Note: api.py currently doesn't actually produce a wav in the mock
        # because tts.tts_to_file is called. We should mock that it creates a file.
        
        def side_effect(text, speaker_wav, language, file_path):
            with open(file_path, "wb") as f:
                f.write(b"fake generated wav")
                
        mock_tts_instance.tts_to_file.side_effect = side_effect
        
        response = client.get("/", params={
            "text": "Hello world",
            "language": "en",
            "speaker_wav": "dummy_speaker.wav"
        })
        
        assert response.status_code == 200
        assert response.content == b"fake generated wav"
        assert response.headers["content-type"] == "audio/wav"
        
    finally:
        if os.path.exists("dummy_speaker.wav"):
            os.remove("dummy_speaker.wav")

def test_missing_speaker_wav():
    """Test error handling when speaker wav is missing."""
    response = client.get("/", params={
        "text": "Hello",
        "language": "en",
        "speaker_wav": "non_existent.wav"
    })
    assert response.status_code == 404
    assert "not found" in response.json()["detail"]
