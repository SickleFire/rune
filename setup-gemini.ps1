<#
.SYNOPSIS
    Interactive PowerShell setup script for configuring Google Gemini API keys and models for Rune CLI.
.DESCRIPTION
    This script helps you easily set up your Gemini API key and model preference.
    It can save the settings to user-level environment variables, project-level environment variables,
    or a `rune.toml` configuration file.
.EXAMPLE
    .\setup-gemini.ps1
#>

[CmdletBinding()]
param()

# Ensure color support & clean output
$ErrorActionPreference = "Stop"

function Write-Color {
    param([string]$Text, [ConsoleColor]$Color = [ConsoleColor]::White)
    $oldColor = [Console]::ForegroundColor
    [Console]::ForegroundColor = $Color
    [Console]::WriteLine($Text)
    [Console]::ForegroundColor = $oldColor
}

Write-Color "=== Rune CLI: Gemini Setup ===" ([ConsoleColor]::Cyan)
Write-Color "This script will help you configure your Google Gemini API key and model preference for Rune." ([ConsoleColor]::Gray)
Write-Host ""

# Prompt for Gemini API Key
$apiKey = Read-Host -Prompt "Enter your Google Gemini API Key (or press Enter to skip)"
$apiKey = $apiKey.Trim()

# Prompt for Gemini Model
$defaultModel = "gemini-3.5-flash-lite"
$modelInput = Read-Host -Prompt "Enter Gemini Model name [default: $defaultModel]"
$modelInput = $modelInput.Trim()
if ([string]::IsNullOrEmpty($modelInput)) {
    $model = $defaultModel
} else {
    $model = $modelInput
}

Write-Host ""
Write-Color "Where would you like to save these settings?" ([ConsoleColor]::Yellow)
Write-Color "  1) User Environment Variables (Persistent across all projects for current user)" ([ConsoleColor]::White)
Write-Color "  2) Project 'rune.toml' configuration file (In current directory)" ([ConsoleColor]::White)
Write-Color "  3) Both (Environment Variables & rune.toml)" ([ConsoleColor]::White)
Write-Color "  4) Current PowerShell Session only" ([ConsoleColor]::White)
$choice = Read-Host -Prompt "Select option (1-4) [default: 1]"

if ([string]::IsNullOrEmpty($choice)) {
    $choice = "1"
}

switch ($choice) {
    "1" {
        if (-not [string]::IsNullOrEmpty($apiKey)) {
            [Environment]::SetEnvironmentVariable("GEMINI_API_KEY", $apiKey, [EnvironmentVariableTarget]::User)
            Write-Color "[OK] Set User Environment Variable: GEMINI_API_KEY" ([ConsoleColor]::Green)
        }
        [Environment]::SetEnvironmentVariable("GEMINI_MODEL", $model, [EnvironmentVariableTarget]::User)
        Write-Color "[OK] Set User Environment Variable: GEMINI_MODEL = $model" ([ConsoleColor]::Green)
        
        # Also set for current session
        if (-not [string]::IsNullOrEmpty($apiKey)) {
            $env:GEMINI_API_KEY = $apiKey
        }
        $env:GEMINI_MODEL = $model
        
        Write-Color "`nSuccess! Gemini API Key and Model configured in User Environment Variables." ([ConsoleColor]::Green)
        Write-Color "You can now run 'rune' from any terminal." ([ConsoleColor]::Cyan)
    }
    "2" {
        $tomlPath = "./rune.toml"
        $tomlContent = ""
        
        if (Test-Path $tomlPath) {
            $tomlContent = Get-Content $tomlPath -Raw
        } else {
            if (Test-Path "./rune.toml.example") {
                $tomlContent = Get-Content "./rune.toml.example" -Raw
                Write-Color "[INFO] Copied defaults from rune.toml.example" ([ConsoleColor]::Gray)
            } else {
                $tomlContent = @"
# Rune Configuration (`rune.toml`)
provider = "gemini"
gemini_api_key = ""
gemini_model = "gemini-3.5-flash-lite"
openai_api_key = ""
openai_model = "gpt-5.6-luna"
anthropic_api_key = ""
anthropic_model = "claude-3-5-sonnet"
ollama_base_url = "http://localhost:11434"
ollama_model = "llama3"
lm_studio_base_url = "http://localhost:1234/v1"
lm_studio_model = "local-model"
architect_model = "gemini-3.5-flash-lite"
coder_model = "gpt-5.6-luna"
"@
            }
        }

        # Update or add provider = "gemini"
        if ($tomlContent -match '(?m)^\s*provider\s*=') {
            $tomlContent = [System.Text.RegularExpressions.Regex]::Replace($tomlContent, '(?m)^\s*provider\s*=.*$', 'provider = "gemini"')
        } else {
            $tomlContent = "provider = ""`n" + $tomlContent
        }

        # Update or add gemini_api_key
        if (-not [string]::IsNullOrEmpty($apiKey)) {
            if ($tomlContent -match '(?m)^\s*gemini_api_key\s*=') {
                $tomlContent = [System.Text.RegularExpressions.Regex]::Replace($tomlContent, '(?m)^\s*gemini_api_key\s*=.*$', "gemini_api_key = `"$apiKey`"")
            } else {
                $tomlContent += "`ngemini_api_key = `"$apiKey`"`n"
            }
        }

        # Update or add gemini_model
        if ($tomlContent -match '(?m)^\s*gemini_model\s*=') {
            $tomlContent = [System.Text.RegularExpressions.Regex]::Replace($tomlContent, '(?m)^\s*gemini_model\s*=.*$', "gemini_model = `"$model`"")
        } else {
            $tomlContent += "`ngemini_model = `"$model`"`n"
        }

        Set-Content -Path $tomlPath -Value $tomlContent -Encoding UTF8
        Write-Color "[OK] Updated $tomlPath with Gemini settings." ([ConsoleColor]::Green)
        Write-Color "`nSuccess! rune.toml configured successfully in the current directory." ([ConsoleColor]::Green)
    }
    "3" {
        if (-not [string]::IsNullOrEmpty($apiKey)) {
            [Environment]::SetEnvironmentVariable("GEMINI_API_KEY", $apiKey, [EnvironmentVariableTarget]::User)
            $env:GEMINI_API_KEY = $apiKey
            Write-Color "[OK] Set User Environment Variable: GEMINI_API_KEY" ([ConsoleColor]::Green)
        }
        [Environment]::SetEnvironmentVariable("GEMINI_MODEL", $model, [EnvironmentVariableTarget]::User)
        $env:GEMINI_MODEL = $model
        Write-Color "[OK] Set User Environment Variable: GEMINI_MODEL = $model" ([ConsoleColor]::Green)

        $tomlPath = "./rune.toml"
        $tomlContent = ""
        if (Test-Path $tomlPath) {
            $tomlContent = Get-Content $tomlPath -Raw
        } else {
            if (Test-Path "./rune.toml.example") {
                $tomlContent = Get-Content "./rune.toml.example" -Raw
            } else {
                $tomlContent = @"
provider = "gemini"
gemini_api_key = ""
gemini_model = "gemini-3.5-flash-lite"
"@
            }
        }

        if ($tomlContent -match '(?m)^\s*provider\s*=') {
            $tomlContent = [System.Text.RegularExpressions.Regex]::Replace($tomlContent, '(?m)^\s*provider\s*=.*$', 'provider = "gemini"')
        }
        if (-not [string]::IsNullOrEmpty($apiKey)) {
            if ($tomlContent -match '(?m)^\s*gemini_api_key\s*=') {
                $tomlContent = [System.Text.RegularExpressions.Regex]::Replace($tomlContent, '(?m)^\s*gemini_api_key\s*=.*$', "gemini_api_key = `"$apiKey`"")
            } else {
                $tomlContent += "`ngemini_api_key = `"$apiKey`"`n"
            }
        }
        if ($tomlContent -match '(?m)^\s*gemini_model\s*=') {
            $tomlContent = [System.Text.RegularExpressions.Regex]::Replace($tomlContent, '(?m)^\s*gemini_model\s*=.*$', "gemini_model = `"$model`"")
        } else {
            $tomlContent += "`ngemini_model = `"$model`"`n"
        }

        Set-Content -Path $tomlPath -Value $tomlContent -Encoding UTF8
        Write-Color "[OK] Updated $tomlPath with Gemini settings." ([ConsoleColor]::Green)
        Write-Color "`nSuccess! Configured both User Environment Variables and rune.toml." ([ConsoleColor]::Green)
    }
    "4" {
        if (-not [string]::IsNullOrEmpty($apiKey)) {
            $env:GEMINI_API_KEY = $apiKey
            Write-Color "[OK] Session Environment Variable GEMINI_API_KEY set." ([ConsoleColor]::Green)
        }
        $env:GEMINI_MODEL = $model
        Write-Color "[OK] Session Environment Variable GEMINI_MODEL set to $model." ([ConsoleColor]::Green)
        Write-Color "`nSuccess! Configured for current PowerShell session." ([ConsoleColor]::Green)
    }
    default {
        Write-Color "[ERROR] Invalid selection." ([ConsoleColor]::Red)
        exit 1
    }
}

Write-Host ""
Write-Color "You are all set! Test your configuration by running:" ([ConsoleColor]::Cyan)
Write-Color "  rune" ([ConsoleColor]::White)
