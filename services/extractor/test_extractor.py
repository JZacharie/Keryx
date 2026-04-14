import httpx
import asyncio
import sys
import os

# Configuration for testing
EXTRACTOR_URL = os.getenv("EXTRACTOR_URL", "http://localhost:8000")
TEST_VIDEO_URL = "https://www.youtube.com/watch?v=aqz-KE-bpKQ" # Big Buck Bunny 60fps 4K - Short Version
TEST_JOB_ID = "test-job-001"

async def test_health():
    print(f"--- Checking Health at {EXTRACTOR_URL}/health ---")
    async with httpx.AsyncClient() as client:
        try:
            resp = await client.get(f"{EXTRACTOR_URL}/health")
            print(f"Status: {resp.status_code}")
            print(f"Body: {resp.json()}")
            assert resp.status_code == 200
            assert resp.json()["status"] == "ok"
            print("✅ Health Check Passed")
        except Exception as e:
            print(f"❌ Health Check Failed: {e}")
            return False
    return True

async def test_extract_robustness():
    print(f"\n--- Testing Extraction Robustness for {TEST_VIDEO_URL} ---")
    payload = {
        "url": TEST_VIDEO_URL,
        "job_id": TEST_JOB_ID,
        "audio_format": "wav"
    }
    
    async with httpx.AsyncClient(timeout=300.0) as client:
        try:
            print(f"Sending POST to /extract (timeout=300s)...")
            resp = await client.post(f"{EXTRACTOR_URL}/extract", json=payload)
            print(f"Status: {resp.status_code}")
            
            if resp.status_code != 200:
                print(f"❌ Extraction Failed: {resp.text}")
                return False
                
            data = resp.json()
            print(f"Response: {data}")
            
            # Validation logic
            assert data["status"] == "success"
            assert "video_url" in data and data["video_url"].endswith(".mp4")
            assert "audio_url" in data and data["audio_url"].endswith(".wav")
            assert data["duration"] > 0
            assert "title" in data
            print("✅ Extraction Test Passed")
            
        except Exception as e:
            print(f"❌ Extraction Test Failed: {e}")
            return False
    return True

async def main():
    health_ok = await test_health()
    if not health_ok:
        print("Stopping further tests due to unhealthy service.")
        sys.exit(1)
        
    extract_ok = await test_extract_robustness()
    if not extract_ok:
        sys.exit(1)
        
    print("\n🎉 ALL TESTS PASSED SUCCESSFULLY 🎉")

if __name__ == "__main__":
    asyncio.run(main())
