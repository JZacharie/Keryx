import boto3
from botocore.client import Config
import time

s3 = boto3.resource('s3',
                    endpoint_url='http://localhost:9000',
                    aws_access_key_id='keryx2',
                    aws_secret_access_key='4DoK6ArcksSBZJIznybMEleQ9cWMsCOTf02IdLII', # pragma: allowlist secret
                    config=Config(signature_version='s3v4'),
                    region_name='us-east-1')

def setup_minio():
    bucket_name = 'keryx'
    # Create bucket if not exists
    if s3.Bucket(bucket_name).creation_date is None:
        print(f"Creating bucket '{bucket_name}'...")
        s3.create_bucket(Bucket=bucket_name)

def upload_test_video(path: str):
    # Uploading a video to be processed
    print(f"Uploading {path} to keryx bucket...")
    s3.Bucket('keryx').upload_file(path, 'tests/input_video.mp4')
    return "http://minio:9000/keryx/tests/input_video.mp4"

if __name__ == "__main__":
    setup_minio()
    # Note: caller should provide the video path
