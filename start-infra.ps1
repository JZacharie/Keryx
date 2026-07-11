# Stop any existing container stacks to release ports/resources
Write-Host "Stopping existing Keryx containers..." -ForegroundColor Yellow
podman compose -f docker-compose.windows.yaml down

# Start only the core infrastructure services
Write-Host "Starting Keryx core infrastructure (Orchestrator, Frontend, Redis, MinIO)..." -ForegroundColor Green
podman compose -f docker-compose.windows.yaml up orchestrator frontend redis minio create-buckets 2>&1 | Tee-Object -FilePath "keryx.log"
