<#
.SYNOPSIS
    Automated installer script for Rune CLI on Windows, macOS, and Linux.
.DESCRIPTION
    This PowerShell script downloads the latest release of Rune CLI from GitHub,
    extracts it, installs the `rune` binary into a directory in your system's PATH
    (defaulting to `C:\Program Files\Rune` or user profile `~\.local\bin`),
    and optionally invokes the interactive Gemini setup script.
.EXAMPLE
    iex (iwr -useb https://raw.githubusercontent.com/SickleFire/rune/main/install.ps1)
#>

[CmdletBinding()]
param(
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

function Write-Color {
    param([string]$Text, [ConsoleColor]$Color = [ConsoleColor]::White)
    $oldColor = [Console]::ForegroundColor
    [Console]::ForegroundColor = $Color
    [Console]::WriteLine($Text)
    [Console]::ForegroundColor = $oldColor
}

Write-Color "=== Rune CLI Installer ===" ([ConsoleColor]::Cyan)

$Repo = "SickleFire/rune"
$BinaryName = "rune.exe"

# Determine OS and architecture
$isWindows = $true # Running in PowerShell
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }

if ($arch -ne "x86_64") {
    Write-Color "[ERROR] Unsupported architecture: $arch. 64-bit Windows is required." ([ConsoleColor]::Red)
    exit 1
}

$Target = "x86_64-pc-windows-msvc"
$Archive = "rune-$Target.zip"
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/$Archive"

# Default install directory
if ([string]::IsNullOrEmpty($InstallDir)) {
    # Check if running as Administrator
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if ($isAdmin) {
        $InstallDir = "C:\Program Files\Rune"
    } else {
        $InstallDir = "$HOME\.local\bin"
    }
}

Write-Color "Downloading $Archive..." ([ConsoleColor]::Blue)
$tmpDir = Join-Path $env:TEMP ("rune-install-" + [Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    $zipPath = Join-Path $tmpDir $Archive
    
    # Download with TLS 1.2+
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12, [Net.SecurityProtocolType]::Tls13
    
    try {
        $headers = @{ "User-Agent" = "Rune-Installer" }
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $zipPath -UseBasicParsing -Headers $headers
    } catch {
        Write-Color "[ERROR] Failed to download from $DownloadUrl" ([ConsoleColor]::Red)
        Write-Color $_.Exception.Message ([ConsoleColor]::Red)
        exit 1
    }

    Write-Color "Extracting archive..." ([ConsoleColor]::Blue)
    Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

    Write-Color "Installing rune to $InstallDir..." ([ConsoleColor]::Blue)
    $extractedExe = Join-Path $tmpDir "rune.exe"
    if (-not (Test-Path $extractedExe)) {
        # Check subfolder if any
        $found = Get-ChildItem -Path $tmpDir -Filter "rune.exe" -Recurse | Select-Object -First 1
        if ($found) {
            $extractedExe = $found.FullName
        } else {
            Write-Color "[ERROR] rune.exe not found in extracted archive." ([ConsoleColor]::Red)
            exit 1
        }
    }

    Copy-Item -Path $extractedExe -Destination (Join-Path $InstallDir "rune.exe") -Force

    # Add to User PATH if not already present
    $userPath = [Environment]::GetEnvironmentVariable("PATH", [EnvironmentVariableTarget]::User)
    if ($userPath -notlike "*$InstallDir*") {
        Write-Color "Adding $InstallDir to user PATH..." ([ConsoleColor]::Yellow)
        $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
        [Environment]::SetEnvironmentVariable("PATH", $newPath, [EnvironmentVariableTarget]::User)
        # Also update current session PATH
        $env:PATH = "$env:PATH;$InstallDir"
        Write-Color "[OK] Added to PATH." ([ConsoleColor]::Green)
    }

    Write-Color "`n=== Successfully installed Rune CLI! ===" ([ConsoleColor]::Green)
    Write-Color "Installation Path: $InstallDir\rune.exe" ([ConsoleColor]::White)

    # Prompt to run Gemini setup
    Write-Host ""
    $setupGemini = Read-Host -Prompt "Would you like to configure your Google Gemini API key now? (Y/n)"
    if ([string]::IsNullOrEmpty($setupGemini) -or $setupGemini.StartsWith("y", [StringComparison]::OrdinalIgnoreCase)) {
        $setupScriptUrl = "https://raw.githubusercontent.com/$Repo/main/setup-gemini.ps1"
        Write-Color "Running Gemini setup..." ([ConsoleColor]::Cyan)
        try {
            $scriptContent = (New-Object Net.WebClient).DownloadString($setupScriptUrl)
            Invoke-Command -ScriptBlock ([scriptblock]::Create($scriptContent))
        } catch {
            Write-Color "[INFO] Could not fetch setup-gemini.ps1 remotely. You can run setup-gemini.ps1 locally anytime." ([ConsoleColor]::Yellow)
        }
    }

} finally {
    if (Test-Path $tmpDir) {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
