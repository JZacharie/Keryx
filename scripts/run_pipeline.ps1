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

# ── 5. Create job (suit le pipeline en direct) ──────────────
Write-Host ""
Write-Host "── Step 4/5: Running pipeline ──" -ForegroundColor Yellow
Write-Host "  Le pipeline est synchrone : la requete POST reste ouverte" -ForegroundColor Gray
Write-Host "  jusqu'a la fin du traitement (plusieurs minutes)." -ForegroundColor Gray
Write-Host "  Les fichiers intermediaires apparaissent dans shared_data/ au fur et a mesure." -ForegroundColor Gray
Write-Host ""

$body = @{ video_url = $VideoUrl; target_langs = @($TargetLang) } | ConvertTo-Json

# Create the job in background so we can poll outputs in parallel
$jobId = $null
$pipelineError = $null
$watchTimer = [System.Diagnostics.Stopwatch]::StartNew()

# Launch the REST call in a background job
$restJob = Start-Job -ScriptBlock {
    param($url, $body, $apiKey)
    return Invoke-RestMethod -Uri $url -Method Post -Body $body `
        -ContentType "application/json" -Headers @{ Authorization = "Bearer $apiKey" } `
        -TimeoutSec 3600
} -ArgumentList "$ApiUrl/api/jobs", $body, $ApiKey

# Poll for outputs while waiting
while ($restJob.State -eq "Running") {
    Start-Sleep -Seconds 5

    # Show new files in shared_data
    $jobDirs = Get-ChildItem $SharedDir -Directory -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
    foreach ($jd in $jobDirs) {
        $subdirs = Get-ChildItem $jd.FullName -Directory -ErrorAction SilentlyContinue
        foreach ($sd in $subdirs) {
            $files = @(Get-ChildItem $sd.FullName -File -ErrorAction SilentlyContinue)
            if ($files.Count -gt 0) {
                Write-Host "  [$($jd.Name)/$($sd.Name)/] $($files.Count) fichiers" -ForegroundColor DarkGray
            }
        }
    }
}

$watchTimer.Stop()

# Get the result
try {
    $r = Receive-Job -Job $restJob -ErrorAction Stop
    $jobId = $r.job_id
    Write-Host ""
    Write-Host "  Pipeline termine en $([math]::Round($watchTimer.Elapsed.TotalSeconds))s" -ForegroundColor Green
} catch {
    $pipelineError = $_
}

Remove-Job -Job $restJob -ErrorAction SilentlyContinue

# ── 6. Show results ─────────────────────────────────────────
if ($jobId) {
    Write-Host ""
    Write-Host "══════════════════════════════════════════════════" -ForegroundColor Green
    Write-Host "  JOB TERMINE : $jobId" -ForegroundColor Green
    Write-Host "══════════════════════════════════════════════════" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Outputs :" -ForegroundColor Cyan
    $jobDir = Join-Path $SharedDir $jobId
    if (Test-Path $jobDir) {
        Get-ChildItem $jobDir -Directory | ForEach-Object {
            Write-Host "    $($_.Name)/" -ForegroundColor Yellow
            Get-ChildItem $_.FullName -File | ForEach-Object {
                Write-Host "      $($_.Name) ($([math]::Round($_.Length/1KB)) KB)" -ForegroundColor Gray
            }
        }
    }
} else {
    Write-Host ""
    Write-Host "  PIPELINE ECHOUE" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Verifie les logs de l'orchestrator :" -ForegroundColor Yellow
    Write-Host "    & $DockerCmd -f $ComposePath logs orchestrator --tail 100" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  Causes possibles :" -ForegroundColor Yellow
    Write-Host "  1. MinIO inaccessible (docker compose ps minio)" -ForegroundColor Gray
    Write-Host "  2. Docker socket non monte" -ForegroundColor Gray
    Write-Host "  3. Modele Whisper/XTTS en cours de telechargement (premiere fois ~5-10min)" -ForegroundColor Gray
    Write-Host "  4. GPU indisponible" -ForegroundColor Gray
    Write-Host ""
    if ($pipelineError) {
        Write-Host "  Erreur: $pipelineError" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "Logs:   & $DockerCmd -f $ComposePath logs orchestrator -f" -ForegroundColor Gray
Write-Host "Stop:   & $DockerCmd -f $ComposePath down" -ForegroundColor Gray
