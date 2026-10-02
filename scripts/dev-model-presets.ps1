#Requires -Version 5.1
<#
.SYNOPSIS
Build and launch the model-presets TUI experiment on Windows x64.
.EXAMPLE
powershell -File .\scripts\dev-model-presets.ps1
.EXAMPLE
pwsh -File .\scripts\dev-model-presets.ps1 -NoBuild
#>
[CmdletBinding()]
param([switch]$NoBuild)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

$BUILD_TARGET = 'x86_64-pc-windows-msvc'
$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoWorkspace = Join-Path $repoRoot 'codex-rs'
$targetDirectory = Join-Path $cargoWorkspace 'target'
$codexBinary = Join-Path $targetDirectory "$BUILD_TARGET\debug\codex.exe"
$originalEnvironment = @{}
foreach ($entry in Get-ChildItem Env:) {
    $originalEnvironment[$entry.Name] = $entry.Value
}

Push-Location $cargoWorkspace
try {
    if (-not $NoBuild) {
        # Even toolchain-list queries can otherwise install the directory's pinned toolchain.
        $env:RUSTUP_AUTO_INSTALL = '0'
        $rustup = Get-Command rustup.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $rustup) {
            throw 'rustup was not found. Install Rust with rustup and open a new PowerShell window.'
        }

        $toolchainConfig = Get-Content -Encoding UTF8 -Raw 'rust-toolchain.toml'
        $channelMatch = [regex]::Match($toolchainConfig, '(?m)^\s*channel\s*=\s*"([^"]+)"')
        if (-not $channelMatch.Success) {
            throw 'Cannot read the channel from codex-rs/rust-toolchain.toml.'
        }
        $channel = $channelMatch.Groups[1].Value
        $toolchain = "$channel-$BUILD_TARGET"
        $installedToolchains = @(& $rustup.Source toolchain list)
        if ($LASTEXITCODE -ne 0) {
            throw 'Cannot list installed Rust toolchains.'
        }
        if ($toolchain -notin @($installedToolchains | ForEach-Object { ($_ -split '\s+')[0] })) {
            throw "Required Rust toolchain is missing. Run: rustup toolchain install $toolchain --profile minimal --component rustfmt --component clippy --component rust-src"
        }

        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
            throw 'Visual Studio Installer was not found. Install the Desktop development with C++ workload.'
        }
        $visualStudioPath = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -ne 0 -or -not $visualStudioPath) {
            throw 'MSVC x64/x86 tools were not found. Add Desktop development with C++ in Visual Studio Installer.'
        }
        $developerCommand = Join-Path $visualStudioPath 'Common7\Tools\VsDevCmd.bat'
        $developerEnvironment = & $env:ComSpec /d /s /c "`"$developerCommand`" -no_logo -arch=x64 -host_arch=x64 >nul && set"
        if ($LASTEXITCODE -ne 0) {
            throw 'Failed to initialize the Visual Studio x64 developer environment.'
        }
        foreach ($line in $developerEnvironment) {
            if ($line -match '^([^=]+)=(.*)$') {
                if (-not $originalEnvironment.ContainsKey($Matches[1]) -or $originalEnvironment[$Matches[1]] -cne $Matches[2]) {
                    [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
                }
            }
        }

        Write-Host "Building Codex with Rust $channel ($BUILD_TARGET)..."
        & $rustup.Source run $toolchain cargo build --locked -p codex-cli --bin codex --target $BUILD_TARGET --target-dir $targetDirectory
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed (exit code $LASTEXITCODE). Codex was not started."
        }
    }

    if (-not (Test-Path -LiteralPath $codexBinary -PathType Leaf)) {
        throw "Codex binary was not found at $codexBinary. Run this script without -NoBuild first."
    }

    $devRoot = Join-Path $env:LOCALAPPDATA 'CodexModelPresetsDev'
    $devCodexHome = Join-Path $devRoot 'home'
    $devWorkspace = Join-Path $devRoot 'workspace'
    New-Item -ItemType Directory -Force -Path $devCodexHome, $devWorkspace | Out-Null
    $env:CODEX_HOME = $devCodexHome
    $env:CODEX_SQLITE_HOME = $devCodexHome
    Set-Location $devWorkspace

    Write-Host "Starting $codexBinary"
    Write-Host "Development state: $devCodexHome"
    # Source builds run the embedded server without a packaged daemon installation.
    & $codexBinary --no-daemon
    if ($LASTEXITCODE -ne 0) {
        throw "Codex exited with code $LASTEXITCODE."
    }
} finally {
    # VsDevCmd changes PATH and SDK variables as well as the Codex-specific settings.
    foreach ($entry in @(Get-ChildItem Env:)) {
        if (-not $originalEnvironment.ContainsKey($entry.Name)) {
            Remove-Item -LiteralPath "Env:$($entry.Name)"
        }
    }
    foreach ($name in $originalEnvironment.Keys) {
        $currentEntry = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        if (-not $currentEntry -or $currentEntry.Value -cne $originalEnvironment[$name]) {
            [Environment]::SetEnvironmentVariable($name, $originalEnvironment[$name], 'Process')
        }
    }
    Pop-Location
}
