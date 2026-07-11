param(
    [string]$VideoUrl = "https://youtube.com/watch?v=s_cfKmu34Es",
    [string]$TargetLang = "en",
    [string]$ApiKey = "",
    [string]$ComposeFile = "../docker-compose.windows.yaml"
)

$ErrorActionPreference = "Stop"

# ── Config ──────────────────────────────────────────────────
$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$SharedDir = Join-Path $ProjectRoot "shared_data"
$ComposePath = Join-Path $ProjectRoot $ComposeFile

Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║         Keryx Pipeline Launcher                  ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "Project root : $ProjectRoot" -ForegroundColor Gray
Write-Host "Shared dir   : $SharedDir" -ForegroundColor Gray
Write-Host "Video        : $VideoUrl" -ForegroundColor Gray
Write-Host "Target lang  : $TargetLang" -ForegroundColor Gray

# Ensure shared_data exists
if (-not (Test-Path $SharedDir)) {
    New-Item -ItemType Directory -Path $SharedDir -Force | Out-Null
    Write-Host "✓ Created shared_data directory" -ForegroundColor Green
}

# ── 1. Build worker images ──────────────────────────────────
Write-Host ""
Write-Host "── Step 1: Building worker images ──" -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    docker compose -f $ComposePath --profile manual build --parallel
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
    Write-Host "✓ Worker images built" -ForegroundColor Green
} finally {
    Pop-Location
}

# ── 2. Start infrastructure ─────────────────────────────────
Write-Host ""
Write-Host "── Step 2: Starting infrastructure ──" -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    docker compose -f $ComposePath up -d redis minio create-buckets orchestrator
    if ($LASTEXITCODE -ne 0) { throw "Infrastructure startup failed" }
    Write-Host "✓ Infrastructure started (redis, minio, orchestrator)" -ForegroundColor Green
} finally {
    Pop-Location
}

# ── 3. Wait for orchestrator ─────────────────────────────────
Write-Host ""
Write-Host "── Step 3: Waiting for orchestrator health ──" -ForegroundColor Yellow
$orchestratorReady = $false
for ($i = 0; $i -lt 60; $i++) {
    try {
        $resp = Invoke-RestMethod -Uri "http://localhost:3000/health" -TimeoutSec 5
        if ($resp.status -eq "ok") {
            $orchestratorReady = $true
            Write-Host "✓ Orchestrator ready" -ForegroundColor Green
            break
        }
    } catch {
        # not ready yet
    }
    Write-Host "  Waiting... ($i/60)" -ForegroundColor Gray
    Start-Sleep -Seconds 2
}
if (-not $orchestratorReady) {
    throw "Orchestrator not ready after 120s. Check: docker compose logs orchestrator"
}

# ── 4. Read API key ──────────────────────────────────────────
if ([string]::IsNullOrEmpty($ApiKey)) {
    $ApiKey = [System.Environment]::GetEnvironmentVariable("API_KEY")
}
if ([string]::IsNullOrEmpty($ApiKey)) {
    $ApiKey = "0123456789"  # default from compose file
}
Write-Host "  Using API key: $($ApiKey.Substring(0, [Math]::Min(4, $ApiKey.Length)))..." -ForegroundColor Gray

# ── 5. Create job ────────────────────────────────────────────
Write-Host ""
Write-Host "── Step 4: Submitting video job ──" -ForegroundColor Yellow
$body = @{
    video_url   = $VideoUrl
    target_langs = @($TargetLang)
} | ConvertTo-Json

try {
    $jobResult = Invoke-RestMethod -Uri "http://localhost:3000/api/jobs" -Method Post `
        -Body $body -ContentType "application/json" `
        -Headers @{ Authorization = "Bearer $ApiKey" }
    $jobId = $jobResult.job_id
    Write-Host "✓ Job created: $jobId" -ForegroundColor Green
} catch {
    Write-Host "✗ Failed to create job: $_" -ForegroundColor Red
    exit 1
}

# ── 6. Monitor job ───────────────────────────────────────────
Write-Host ""
Write-Host "── Step 5: Monitoring job ──" -ForegroundColor Yellow
Write-Host "  Outputs will appear in: $SharedDir" -ForegroundColor Cyan
Write-Host ""

$done = $false
while (-not $done) {
    try {
        $job = Invoke-RestMethod -Uri "http://localhost:3000/api/jobs/$jobId" -TimeoutSec 10
        $statusLabel = "unknown"
        if ($job.status -is [string]) {
            $statusLabel = $job.status
        } elseif ($job.status.PSObject.Properties.Name -contains "Processing") {
            $statusLabel = "Processing: $($job.status.Processing)"
        } elseif ($job.status.PSObject.Properties.Name -contains "Failed") {
            $statusLabel = "Failed: $($job.status.Failed)"
        }
        Write-Host "  Status: $statusLabel" -ForegroundColor Magenta

        # Show shared_data contents
        $outputDirs = Get-ChildItem -Path $SharedDir -Directory -ErrorAction SilentlyContinue
        foreach ($dir in $outputDirs) {
            $subdirs = Get-ChildItem -Path $dir.FullName -Directory -ErrorAction SilentlyContinue
            foreach ($sd in $subdirs) {
                $files = Get-ChildItem -Path $sd.FullName -File -ErrorAction SilentlyContinue
                if ($files.Count -gt 0) {
                    Write-Host "    📁 $($dir.Name)/$($sd.Name)/  ($($files.Count) files)" -ForegroundColor DarkGray
                }
            }
        }

        if ($job.status -eq "Completed") {
            Write-Host ""
            Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Green
            Write-Host "║                 JOB COMPLETED                    ║" -ForegroundColor Green
            Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Green
            Write-Host ""
            Write-Host "Output directory: $SharedDir\$jobId\" -ForegroundColor Cyan
            Write-Host ""
            Write-Host "Subdirectories by microservice:" -ForegroundColor White
            $serviceDirs = Get-ChildItem -Path "$SharedDir\$jobId" -Directory -ErrorAction SilentlyContinue
            foreach ($svcDir in $serviceDirs) {
                Write-Host "  📁 $($svcDir.Name)/" -ForegroundColor Yellow
                $svcFiles = Get-ChildItem -Path $svcDir.FullName -File -ErrorAction SilentlyContinue
                foreach ($f in $svcFiles) {
                    Write-Host "      📄 $($f.Name) ($( [math]::Round($f.Length/1KB) ) KB)" -ForegroundColor Gray
                }
            }
            $done = $true
        } elseif ($job.status -is [System.Management.Automation.PSCustomObject] -and $null -ne $job.status.Failed) {
            Write-Host ""
            Write-Host "✗ JOB FAILED: $($job.status.Failed)" -ForegroundColor Red
            Write-Host "Check logs: docker compose -f $ComposeFile logs orchestrator" -ForegroundColor Yellow
            $done = $true
        }

    } catch {
        Write-Host "  Error fetching job status: $_" -ForegroundColor DarkYellow
    }

    if (-not $done) {
        Start-Sleep -Seconds 5
    }
}

Write-Host ""
Write-Host "To view logs:       docker compose -f $ComposeFile logs orchestrator -f" -ForegroundColor Gray
Write-Host "To stop everything:  docker compose -f $ComposeFile down" -ForegroundColor Gray
