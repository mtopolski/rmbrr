# Install script for rmbrr (Windows)
# Usage: iwr -useb https://raw.githubusercontent.com/mtopolski/rmbrr/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

Write-Host "Installing rmbrr..." -ForegroundColor Cyan

# Get latest release version
try {
    $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/mtopolski/rmbrr/releases/latest"
    $version = $releases.tag_name
} catch {
    Write-Host "Failed to get latest version: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

Write-Host "Latest version: $version" -ForegroundColor Green

# Download binary
$downloadUrl = "https://github.com/mtopolski/rmbrr/releases/download/$version/rmbrr-windows-x86_64.exe"
$tempFile = [System.IO.Path]::GetTempFileName() + ".exe"

Write-Host "Downloading from $downloadUrl..." -ForegroundColor Yellow

try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing
} catch {
    Write-Host "Failed to download rmbrr: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Determine install location
$installDir = "$env:LOCALAPPDATA\rmbrr"
$installPath = Join-Path $installDir "rmbrr.exe"

# Create directory if needed
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Move binary
Move-Item -Path $tempFile -Destination $installPath -Force

Write-Host "Installed to: $installPath" -ForegroundColor Green

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "Adding to PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable(
        "PATH",
        "$userPath;$installDir",
        "User"
    )
    $env:PATH = "$env:PATH;$installDir"
    Write-Host "Added $installDir to PATH" -ForegroundColor Green
    Write-Host "Note: Restart your terminal for PATH changes to take effect" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Successfully installed rmbrr!" -ForegroundColor Green
Write-Host ""
Write-Host "Try it out:" -ForegroundColor Cyan
Write-Host "  rmbrr --help" -ForegroundColor White
