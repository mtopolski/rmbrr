#!/usr/bin/env pwsh

# Benchmark script for rmbrr vs common Windows deletion methods
# Usage: ./benchmark.ps1

$ErrorActionPreference = "Stop"

# Configuration
$BenchDir = $PSScriptRoot
$TestRoot = Join-Path $BenchDir "test_dirs"
$ResultsFile = Join-Path $BenchDir "benchmark_results.txt"
$Rmbrr = Join-Path (Split-Path $BenchDir) "target\release\rmbrr.exe"

# Ensure the release binary exists
if (-not (Test-Path $Rmbrr)) {
    Write-Host "Building rmbrr in release mode..." -ForegroundColor Yellow
    Push-Location (Split-Path $BenchDir)
    cargo build --release
    Pop-Location
}

# Create test root directory
if (Test-Path $TestRoot) {
    Write-Host "Cleaning up old test directories..." -ForegroundColor Yellow
    Remove-Item $TestRoot -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $TestRoot -Force | Out-Null

Write-Host "=== rmbrr Benchmark Suite ===" -ForegroundColor Cyan
Write-Host ""

# Step 1: Install the nightmare node_modules
Write-Host "[1/2] Creating nightmare node_modules..." -ForegroundColor Green
$SourceDir = Join-Path $BenchDir "node_modules"

if (-not (Test-Path $SourceDir)) {
    Write-Host "  Installing dependencies (this will take a while)..." -ForegroundColor Yellow
    Push-Location $BenchDir
    npm install --loglevel=error
    Pop-Location
}

# Count files and directories
Write-Host "  Analyzing node_modules structure..." -ForegroundColor Yellow
$FileCount = (Get-ChildItem $SourceDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
$DirCount = (Get-ChildItem $SourceDir -Recurse -Directory -ErrorAction SilentlyContinue | Measure-Object).Count
$TotalSize = [math]::Round((Get-ChildItem $SourceDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1MB, 2)

Write-Host ""
Write-Host "Test dataset:" -ForegroundColor Cyan
Write-Host "  Files:       $FileCount" -ForegroundColor White
Write-Host "  Directories: $DirCount" -ForegroundColor White
Write-Host "  Total size:  ${TotalSize} MB" -ForegroundColor White
Write-Host ""

# Initialize results
$Results = @()
$Results += "=== rmbrr Benchmark Results ==="
$Results += ""
$Results += "Test dataset:"
$Results += "  Files:       $FileCount"
$Results += "  Directories: $DirCount"
$Results += "  Total size:  ${TotalSize} MB"
$Results += ""
$Results += "Results:"
$Results += ""

# Test methods with thread configurations
$ThreadCounts = @(1, 2, 4, 8, 16, 32)

$Methods = @()

# rmbrr with different thread counts
foreach ($Threads in $ThreadCounts) {
    $Methods += @{
        Name = "rmbrr (${Threads} threads)"
        Command = {
            param($Path, $ThreadCount)
            & $Rmbrr $Path --silent --threads $ThreadCount
        }.GetNewClosure()
        Threads = $Threads
    }
}

# Other tools (don't support thread configuration)
$Methods += @(
    @{
        Name = "rimraf (Node.js)"
        Command = {
            param($Path, $ThreadCount)
            & npx rimraf $Path
        }
        Threads = $null
    },
    @{
        Name = "PowerShell Remove-Item"
        Command = {
            param($Path, $ThreadCount)
            Remove-Item $Path -Recurse -Force -ErrorAction SilentlyContinue
        }
        Threads = $null
    },
    @{
        Name = "cmd rmdir"
        Command = {
            param($Path, $ThreadCount)
            cmd /c "rmdir /s /q `"$Path`" 2>nul"
        }
        Threads = $null
    },
    @{
        Name = "cmd del + rmdir"
        Command = {
            param($Path, $ThreadCount)
            cmd /c "del /f /s /q `"$Path`" >nul 2>&1 && rmdir /s /q `"$Path`" 2>nul"
        }
        Threads = $null
    },
    @{
        Name = "robocopy /MIR"
        Command = {
            param($Path, $ThreadCount)
            $EmptyDir = Join-Path $TestRoot "_empty_temp"
            New-Item -ItemType Directory -Path $EmptyDir -Force | Out-Null
            robocopy $EmptyDir $Path /MIR /NFL /NDL /NJH /NJS /NC /NS /NP | Out-Null
            Remove-Item $EmptyDir -Force
            Remove-Item $Path -Force -ErrorAction SilentlyContinue
        }
        Threads = $null
    }
)

Write-Host "[2/2] Running benchmarks..." -ForegroundColor Green
Write-Host ""

# Run each test
foreach ($Method in $Methods) {
    $TestDir = Join-Path $TestRoot ("test_" + ($Method.Name -replace '[^a-zA-Z0-9]', '_'))

    Write-Host "Testing: $($Method.Name)" -ForegroundColor Yellow
    Write-Host "  Copying test data..." -ForegroundColor Gray

    # Copy node_modules for this test
    Copy-Item $SourceDir $TestDir -Recurse -Force

    # Verify copy
    if (-not (Test-Path $TestDir)) {
        Write-Host "  ERROR: Failed to copy test directory" -ForegroundColor Red
        continue
    }

    Write-Host "  Running deletion..." -ForegroundColor Gray

    # Measure deletion time
    $Stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

    try {
        & $Method.Command $TestDir $Method.Threads
        $Stopwatch.Stop()

        # Verify deletion
        $Success = -not (Test-Path $TestDir)

        if ($Success) {
            $TimeMs = $Stopwatch.ElapsedMilliseconds
            $TimeSec = [math]::Round($TimeMs / 1000, 3)
            Write-Host "  ✓ Completed in ${TimeSec}s (${TimeMs}ms)" -ForegroundColor Green
            $Results += "$($Method.Name): ${TimeMs}ms (${TimeSec}s)"
        } else {
            Write-Host "  ✗ Failed (directory still exists)" -ForegroundColor Red
            $Results += "$($Method.Name): FAILED (incomplete deletion)"
            # Clean up failed attempt
            Remove-Item $TestDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    } catch {
        $Stopwatch.Stop()
        Write-Host "  ✗ Failed with error: $($_.Exception.Message)" -ForegroundColor Red
        $Results += "$($Method.Name): FAILED (error: $($_.Exception.Message))"
        # Clean up failed attempt
        Remove-Item $TestDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host ""

    # Small delay between tests
    Start-Sleep -Milliseconds 500
}

# Calculate relative performance
Write-Host "=== Results Summary ===" -ForegroundColor Cyan
Write-Host ""

# Find best rmbrr time (for comparison baseline)
$RmbrrTime = $null
foreach ($Line in $Results) {
    if ($Line -match "^rmbrr \(.+ threads\): (\d+)ms") {
        $CurrentTime = [int]$Matches[1]
        if ($RmbrrTime -eq $null -or $CurrentTime -lt $RmbrrTime) {
            $RmbrrTime = $CurrentTime
        }
    }
}

# Display results with relative performance
foreach ($Line in $Results) {
    if ($Line -match "^(.+): (\d+)ms") {
        $Name = $Matches[1]
        $TimeMs = [int]$Matches[2]
        $TimeSec = [math]::Round($TimeMs / 1000, 3)

        # Compare to best rmbrr time if it's not a rmbrr variant
        if ($RmbrrTime -and $Name -notmatch "^rmbrr \(") {
            $Ratio = [math]::Round($TimeMs / $RmbrrTime, 2)
            Write-Host "$Name`: $TimeSec`s ($Ratio`x slower)" -ForegroundColor White
            $Results += "$Name`: $TimeSec`s ($Ratio`x slower than best rmbrr)"
        } else {
            Write-Host "$Name`: $TimeSec`s" -ForegroundColor White
        }
    } elseif ($Line -match "^(.+): FAILED") {
        Write-Host $Line -ForegroundColor Red
    } elseif ($Line -ne "") {
        Write-Host $Line -ForegroundColor Gray
    }
}

Write-Host ""

# Save results to file
$Results | Out-File $ResultsFile -Encoding UTF8
Write-Host "Results saved to: $ResultsFile" -ForegroundColor Green

# Cleanup
Write-Host ""
Write-Host "Cleaning up test directories..." -ForegroundColor Yellow
Remove-Item $TestRoot -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Benchmark complete!" -ForegroundColor Green
