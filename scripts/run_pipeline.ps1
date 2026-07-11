param(
    [string]$VideoUrl = "https://youtube.com/watch?v=s_cfKmu34Es",
    [string]$TargetLang = "en",
    [string]$ApiKey = "",
    [string]$ComposeFile = "docker-compose.windows.yaml"
)

$ErrorActionPreference = "Stop"

# ── Find Docker Compose ────────────────────────────────────
$DockerCmd = $null
if (Get-Command "docker-compose" -ErrorAction SilentlyContinue) {
    $DockerCmd = "docker-compose"
} elseif (Get-Command "docker" -ErrorAction SilentlyContinue) {
    & docker compose version 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) {
        $DockerCmd = "docker compose"
    }
}
if (-not $DockerCmd) {
    Write-Host "ERREUR : Docker Compose introuvable." -ForegroundColor Red
    Write-Host "  Installe Docker Desktop depuis https://www.docker.com/products/docker-desktop/" -ForegroundColor Yellow
    exit 1
}
Write-Host "  Docker: $DockerCmd" -ForegroundColor Gray

# ── Config ──────────────────────────────────────────────────
$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$SharedDir = Join-Path $ProjectRoot "shared_data"
$ComposePath = Join-Path $ProjectRoot $ComposeFile
$ApiUrl = "http://localhost:3000"

Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║         Keryx Pipeline Launcher                  ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "Project  : $ProjectRoot" -ForegroundColor Gray
Write-Host "Outputs  : $SharedDir" -ForegroundColor Gray
Write-Host "Video    : $VideoUrl" -ForegroundColor Gray
Write-Host "API      : $ApiUrl" -ForegroundColor Gray

# Ensure shared_data exists
if (-not (Test-Path $SharedDir)) {
    New-Item -ItemType Directory -Path $SharedDir -Force | Out-Null
}

# ── 1. Build worker images ──────────────────────────────────
Write-Host ""
Write-Host "── Step 1/5: Building worker images ──" -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    & $DockerCmd -f $ComposePath --profile manual build --parallel
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "Build failed" }
    Write-Host "  OK" -ForegroundColor Green
} finally { Pop-Location }

# ── 2. Start infrastructure ─────────────────────────────────
Write-Host ""
Write-Host "── Step 2/5: Starting infrastructure ──" -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    & $DockerCmd -f $ComposePath up -d redis minio create-buckets orchestrator
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "Infrastructure startup failed" }
    Write-Host "  OK (redis, minio, orchestrator)" -ForegroundColor Green
} finally { Pop-Location }

# ── 3. Wait for orchestrator ─────────────────────────────────
Write-Host ""
Write-Host "── Step 3/5: Waiting for orchestrator ──" -ForegroundColor Yellow
$ready = $false
for ($i = 0; $i -lt 60; $i++) {
    try {
        $resp = Invoke-RestMethod -Uri "$ApiUrl/health" -TimeoutSec 5
        if ($resp.status -eq "ok") { $ready = $true; break }
    } catch {}
    Write-Host "  waiting $($i*2)s..." -ForegroundColor Gray
    Start-Sleep -Seconds 2
}
if (-not $ready) { throw "Orchestrator not ready" }
Write-Host "  OK" -ForegroundColor Green

# ── 4. API key ──────────────────────────────────────────────
if (-not $ApiKey) {
    $ApiKey = [Environment]::GetEnvironmentVariable("API_KEY")
}
if (-not $ApiKey) { $ApiKey = "0123456789" }
Write-Host "  Key: $($ApiKey.Substring(0,4))..." -ForegroundColor Gray

# ── 5. Create job ───────────────────────────────────────────
Write-Host ""
Write-Host "── Step 4/5: Submitting job ──" -ForegroundColor Yellow
$body = @{ video_url = $VideoUrl; target_langs = @($TargetLang) } | ConvertTo-Json
try {
    $r = Invoke-RestMethod -Uri "$ApiUrl/api/jobs" -Method Post -Body $body `
        -ContentType "application/json" -Headers @{ Authorization = "Bearer $ApiKey" }
    $jobId = $r.job_id
    Write-Host "  Job: $jobId" -ForegroundColor Green
} catch {
    Write-Host "  FAILED: $_" -ForegroundColor Red
    exit 1
}

# ── 6. Monitor ──────────────────────────────────────────────
Write-Host ""
Write-Host "── Step 5/5: Monitoring (outputs → shared_data/) ──" -ForegroundColor Yellow
$done = $false
while (-not $done) {
    Start-Sleep -Seconds 5
    try {
        $job = Invoke-RestMethod -Uri "$ApiUrl/api/jobs/$jobId" -TimeoutSec 10
        $label = if ($job.status -is [string]) { $job.status }
        elseif ($job.status.Processing) { "Processing: $($job.status.Processing)" }
        elseif ($job.status.Failed) { "Failed: $($job.status.Failed)" }
        else { "unknown" }
        Write-Host "  [$label]" -ForegroundColor Magenta

        # Show outputs per microservice
        $jobDir = Join-Path $SharedDir $jobId
        if (Test-Path $jobDir) {
            Get-ChildItem $jobDir -Directory | ForEach-Object {
                $files = @(Get-ChildItem $_.FullName -File)
                if ($files.Count -gt 0) {
                    Write-Host "    $($_.Name)/  ($($files.Count) fichiers)" -ForegroundColor DarkGray
                }
            }
        }

        if ($job.status -eq "Completed") {
            Write-Host ""
            Write-Host "═══════════════════════════════════════════" -ForegroundColor Green
            Write-Host "  JOB TERMINE" -ForegroundColor Green
            Write-Host "═══════════════════════════════════════════" -ForegroundColor Green
            Write-Host ""
            Write-Host "  Outputs : $jobDir" -ForegroundColor Cyan
            Get-ChildItem $jobDir -Directory | ForEach-Object {
                Write-Host "    $($_.Name)/" -ForegroundColor Yellow
                Get-ChildItem $_.FullName -File | ForEach-Object {
                    Write-Host "      $($_.Name) ($([math]::Round($_.Length/1KB)) KB)" -ForegroundColor Gray
                }
            }
            $done = $true
        } elseif ($job.status.Failed) {
            Write-Host "  ECHEC: $($job.status.Failed)" -ForegroundColor Red
            $done = $true
        }
    } catch {
        Write-Host "  Erreur statut: $_" -ForegroundColor DarkYellow
    }
}

Write-Host ""
Write-Host "Logs: & $DockerCmd -f $ComposePath logs orchestrator -f" -ForegroundColor Gray
