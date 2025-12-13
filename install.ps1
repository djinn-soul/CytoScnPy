# Configuration
$Repo = "djinn09/CytoScnPy" # Replace with your actual username/repo
$BinaryName = "cytoscnpy-mcp.exe"
$AssetName = "cytoscnpy-mcp-windows-x64.exe"
$InstallDir = "$env:LOCALAPPDATA\Programs\CytoScnPy"

Write-Host "Fetching latest release from $Repo..."

# Get Latest Release URL
try {
    $LatestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $AssetUrl = $LatestRelease.assets | Where-Object { $_.name -eq $AssetName } | Select-Object -ExpandProperty browser_download_url
} catch {
    Write-Error "Failed to fetch release info. Ensure the repository is public or you have access."
    exit 1
}

if (-not $AssetUrl) {
    Write-Error "Could not find asset '$AssetName' in the latest release."
    exit 1
}

# Create Install Directory
if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$OutputPath = Join-Path -Path $InstallDir -ChildPath $BinaryName

Write-Host "Downloading to $OutputPath..."
Invoke-WebRequest -Uri $AssetUrl -OutFile $OutputPath

# Add to PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($UserPath -split ";" -contains $InstallDir)) {
    Write-Host "Adding $InstallDir to User PATH..."
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added to PATH. Please restart your terminal/IDE to use 'cytoscnpy-mcp' command."
} else {
    Write-Host "Already in PATH."
}

Write-Host "Installation complete! Binary location: $OutputPath"
