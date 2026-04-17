# rlwy installer for Windows (PowerShell).
#
# Usage:
#   irm https://raw.githubusercontent.com/rlwy-dev/rlwy/main/install.ps1 | iex
#   $env:RLWY_VERSION = 'v0.1.0'; irm https://raw.githubusercontent.com/rlwy-dev/rlwy/main/install.ps1 | iex
#
# Env:
#   RLWY_VERSION      tag to install (default: latest)
#   RLWY_INSTALL_DIR  install directory (default: $env:LOCALAPPDATA\Programs\rlwy)
#   RLWY_REPO         override repo (default: rlwy-dev/rlwy)

$ErrorActionPreference = 'Stop'

$Repo       = if ($env:RLWY_REPO)        { $env:RLWY_REPO }        else { 'rlwy-dev/rlwy' }
$Version    = if ($env:RLWY_VERSION)     { $env:RLWY_VERSION }     else { 'latest' }
$InstallDir = if ($env:RLWY_INSTALL_DIR) { $env:RLWY_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\rlwy' }

function Msg ($m)  { Write-Host "==> $m" -ForegroundColor Green }
function Warn ($m) { Write-Host "==> $m" -ForegroundColor Yellow }
function Die ($m)  { Write-Host "error: $m" -ForegroundColor Red; exit 1 }

# Arch detect (Windows x64 only for now; arm64 support can be added later)
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($arch) {
  'X64'   { $triple = 'x86_64-pc-windows-msvc' }
  default { Die "unsupported Windows arch: $arch (only x64 is currently released)" }
}

if ($Version -eq 'latest') {
  try {
    $resp = Invoke-WebRequest -Uri "https://github.com/$Repo/releases/latest" -MaximumRedirection 0 -ErrorAction SilentlyContinue
  } catch {
    $resp = $_.Exception.Response
  }
  $loc = $null
  if ($resp -and $resp.Headers -and $resp.Headers.Location) {
    $loc = $resp.Headers.Location.ToString()
  } elseif ($resp -and $resp.Headers['Location']) {
    $loc = $resp.Headers['Location']
  }
  if (-not $loc) {
    # Fallback to GitHub API
    $api = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ 'User-Agent' = 'rlwy-installer' }
    $Version = $api.tag_name
  } else {
    $Version = $loc.Split('/')[-1]
  }
  if (-not $Version) { Die "could not resolve latest release for $Repo" }
}

$ver   = $Version.TrimStart('v')
$asset = "rlwy-v$ver-$triple.exe"
$url   = "https://github.com/$Repo/releases/download/$Version/$asset"

Msg "installing rlwy $Version ($triple)"
Msg "↓ $url"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$dest = Join-Path $InstallDir 'rlwy.exe'
$tmp  = "$dest.download"

try {
  Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing
} catch {
  Die "download failed: $url`n$($_.Exception.Message)"
}

if ((Get-Item $tmp).Length -eq 0) { Die "downloaded file is empty" }

Move-Item -Force $tmp $dest
Msg "installed → $dest"

# Add to user PATH if missing
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not ($userPath -split ';' | Where-Object { $_ -eq $InstallDir })) {
  [Environment]::SetEnvironmentVariable('Path', "$userPath;$InstallDir", 'User')
  Warn "added $InstallDir to user PATH. Restart your shell to pick it up."
}

try { & $dest --version } catch { }
Msg "done. try: rlwy --help"
