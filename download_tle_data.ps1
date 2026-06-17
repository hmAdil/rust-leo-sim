# PowerShell script to manually download CelesTrak TLE data
# Usage: .\download_tle_data.ps1 <group-name>
# Example: .\download_tle_data.ps1 stations

param(
    [Parameter(Mandatory=$true)]
    [string]$Group
)

$DatasetDir = "dataset"
$Url = "https://celestrak.org/NORAD/elements/gp.php?GROUP=$Group&FORMAT=tle"
$OutputFile = "$DatasetDir/$Group.txt"

# Create dataset directory if it doesn't exist
if (!(Test-Path $DatasetDir)) {
    New-Item -ItemType Directory -Path $DatasetDir | Out-Null
    Write-Host "Created dataset directory"
}

Write-Host "Downloading TLE data for group: $Group"
Write-Host "URL: $Url"

try {
    # Download with proper headers
    $headers = @{
        "User-Agent" = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        "Accept" = "text/plain"
    }
    
    Invoke-WebRequest -Uri $Url -OutFile $OutputFile -Headers $headers -UseBasicParsing
    
    $fileSize = (Get-Item $OutputFile).Length
    Write-Host "✓ Successfully downloaded $fileSize bytes to: $OutputFile"
    
    # Count satellites
    $lines = Get-Content $OutputFile
    $satCount = [math]::Floor($lines.Count / 3)
    Write-Host "✓ Parsed approximately $satCount satellites"
    
} catch {
    Write-Host "❌ Failed to download: $_"
    Write-Host ""
    Write-Host "Alternative: Manually download from:"
    Write-Host "  https://celestrak.org/NORAD/elements/"
    Write-Host "  Save the TLE file as: $OutputFile"
    exit 1
}

Write-Host ""
Write-Host "You can now run:"
Write-Host "  cargo run --release -- --realtime-gui --celestrak-group $Group"
