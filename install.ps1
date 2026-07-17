# Configuration
$Repo = "djinn-soul/CytoScnPy"
$BinaryName = "cytoscnpy.exe"
$AssetName = "cytoscnpy-windows-x64.exe"
$InstallDir = "$env:LOCALAPPDATA\Programs\CytoScnPy"
$ReleaseBase = "https://github.com/$Repo/releases/latest/download"

$TempDir = Join-Path ([IO.Path]::GetTempPath()) ("cytoscnpy-" + [guid]::NewGuid())
$DownloadPath = Join-Path $TempDir $AssetName
$ChecksumPath = Join-Path $TempDir "SHA256SUMS.txt"
$OutputPath = Join-Path -Path $InstallDir -ChildPath $BinaryName

New-Item -ItemType Directory -Path $TempDir | Out-Null
try {
    # WARNING: Never install the downloaded executable before this release
    # checksum verification succeeds.
    Write-Host "Downloading the latest release from $Repo..."
    Invoke-WebRequest -Uri "$ReleaseBase/$AssetName" -OutFile $DownloadPath -MaximumRedirection 5
    Invoke-WebRequest -Uri "$ReleaseBase/SHA256SUMS.txt" -OutFile $ChecksumPath -MaximumRedirection 5

    $AssetPattern = [regex]::Escape($AssetName)
    $ExpectedHash = $null
    foreach ($Line in Get-Content -LiteralPath $ChecksumPath) {
        if ($Line -match "^([0-9a-fA-F]{64})\s+\*?$AssetPattern$") {
            $ExpectedHash = $Matches[1]
            break
        }
    }

    if (-not $ExpectedHash) {
        throw "Release checksum is missing or malformed for $AssetName."
    }

    $ActualHash = (Get-FileHash -LiteralPath $DownloadPath -Algorithm SHA256).Hash
    if (-not $ActualHash.Equals($ExpectedHash, [StringComparison]::OrdinalIgnoreCase)) {
        throw "SHA-256 verification failed for $AssetName."
    }

    if (-not (Test-Path -LiteralPath $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    Write-Host "Installing to $OutputPath..."
    Move-Item -LiteralPath $DownloadPath -Destination $OutputPath -Force
} catch {
    Write-Error "Installation failed: $($_.Exception.Message)"
    exit 1
} finally {
    Remove-Item -LiteralPath $DownloadPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $TempDir -Force -ErrorAction SilentlyContinue
}

# Add to PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($UserPath -split ";" -contains $InstallDir)) {
    Write-Host "Adding $InstallDir to User PATH..."
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added to PATH. Please restart your terminal/IDE."
} else {
    Write-Host "Already in PATH."
}

Write-Host ""
Write-Host "Success! CytoScnPy CLI installed."
Write-Host ""
Write-Host "Usage:"
Write-Host "  cytoscnpy .                    # Analyze current directory"
Write-Host "  cytoscnpy mcp-server           # Start MCP server for AI assistants"
Write-Host ""
Write-Host "For MCP configuration (Claude, Cursor, Copilot), see:"
Write-Host "  https://github.com/djinn-soul/CytoScnPy/blob/main/cytoscnpy-mcp/README.md"
