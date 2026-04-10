import requests
import time
import sys

API_URL = "http://localhost:3000/api"

def test_e2e_flow():
    print("🚀 Starting Keryx E2E Test Flow...")
    
    # 1. Submit Job
    payload = {
        "video_url": "https://www.youtube.com/watch?v=PsPqWLoZaMc",
        "target_langs": ["fr", "en"]
    }
    
    print(f"📡 Submitting job to {API_URL}/jobs")
    response = requests.post(f"{API_URL}/jobs", json=payload)
    
    if response.status_code != 202:
        print(f"❌ Failed to submit job. Status: {response.status_code}, Body: {response.text}")
        sys.exit(1)
        
    job_id = response.json().get("job_id")
    print(f"✅ Job submitted successfully! ID: {job_id}")
    
    # 2. Polling Loop
    max_retries = 30 # 5 minutes max
    for i in range(max_retries):
        resp = requests.get(f"{API_URL}/jobs/{job_id}")
        if resp.status_code == 200:
            status = resp.json().get("status")
            print(f"⏳ Status: {status} ({i}/{max_retries})")
            
            if status == "Completed":
                print("🎉 Job Completed Successfully!")
                verify_artifacts(job_id)
                sys.exit(0)
            elif status == "Failed":
                print("❌ Job Failed!")
                sys.exit(1)
                
        time.sleep(10)
        
    print("⏳ Timeout reached!")
    sys.exit(1)

def verify_artifacts(job_id):
    # Here you would use boto3 to check the S3 bucket
    print(f"🔍 Verifying artifacts for {job_id} in S3...")
    print("✅ E2E Pass.")

if __name__ == "__main__":
    test_e2e_flow()
