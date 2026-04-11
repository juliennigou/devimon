$ErrorActionPreference = "Stop"

$Repo = "juliennigou/devimon"
$GitUrl = "https://github.com/$Repo"
$InstallDir = Join-Path $env:USERPROFILE ".devimon\bin"
$BinPath = Join-Path $InstallDir "devimon.exe"

function Get-WindowsAssetName {
    $arch = if ($env:PROCESSOR_ARCHITEW6432) {
        $env:PROCESSOR_ARCHITEW6432
    } else {
        $env:PROCESSOR_ARCHITECTURE
    }

    switch ($arch.ToUpperInvariant()) {
        "AMD64" { return "devimon-windows-x86_64.exe" }
        "ARM64" { return "devimon-windows-arm64.exe" }
        default {
            throw "unsupported Windows architecture: $arch"
        }
    }
}

Write-Host "Fetching latest release..."
$release = Invoke-RestMethod `
    -Headers @{ "User-Agent" = "devimon-installer" } `
    -Uri "https://api.github.com/repos/$Repo/releases/latest"

$tag = $release.tag_name
$assetName = Get-WindowsAssetName
$asset = $release.assets | Where-Object { $_.name -eq $assetName } | Select-Object -First 1

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if ($asset) {
    Write-Host "Downloading $assetName ($tag)..."
    Invoke-WebRequest `
        -Headers @{ "User-Agent" = "devimon-installer" } `
        -Uri $asset.browser_download_url `
        -OutFile $BinPath

    Write-Host ""
    Write-Host "Devimon $tag installed to $BinPath"
} elseif (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "No Windows binary is published for $assetName."
    Write-Host "Falling back to cargo install..."
    cargo install --git $GitUrl --locked --force
    Write-Host ""
    Write-Host "Devimon installed via cargo."
    Write-Host "Binary: $env:USERPROFILE\.cargo\bin\devimon.exe"
} else {
    throw "No Windows binary was found for $assetName and cargo is not installed."
}

$pathEntries = $env:PATH -split ";"
if ($pathEntries -notcontains $InstallDir) {
    Write-Host ""
    Write-Host "Add this directory to PATH if needed:"
    Write-Host "  $InstallDir"
}

Write-Host ""
Write-Host "Get started:"
Write-Host "  devimon spawn Embit --species ember"
Write-Host "  devimon"
