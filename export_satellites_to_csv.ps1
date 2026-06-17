# Export satellite data from TLE files to CSV
param(
    [string]$OutputFile = "satellites.csv"
)

$DatasetDir = "dataset"

if (!(Test-Path $DatasetDir)) {
    Write-Host "Dataset directory not found. Run download_tle_data.ps1 first."
    exit 1
}

# Get all TLE files
$tleFiles = Get-ChildItem -Path $DatasetDir -Filter "*.txt"

if ($tleFiles.Count -eq 0) {
    Write-Host "No TLE files found in dataset/. Run download_tle_data.ps1 first."
    exit 1
}

Write-Host "Found $($tleFiles.Count) TLE files"

# Create CSV
$csv = @()
$id = 0

foreach ($file in $tleFiles) {
    $group = $file.BaseName
    Write-Host "  Processing: $group"
    
    $lines = Get-Content $file.FullName
    
    # Parse TLE format (3 lines per satellite)
    for ($i = 0; $i -lt $lines.Count; $i += 3) {
        if ($i + 2 -lt $lines.Count) {
            $name = $lines[$i].Trim()
            $line1 = $lines[$i + 1]
            $line2 = $lines[$i + 2]
            
            # Validate TLE lines
            if ($line1 -match "^1 " -and $line2 -match "^2 ") {
                # Extract NORAD ID from line 1 (columns 3-7)
                $noradId = $line1.Substring(2, 5).Trim()
                
                # Extract inclination from line 2 (columns 9-16)
                $inclination = $line2.Substring(8, 8).Trim()
                
                $csv += [PSCustomObject]@{
                    ID = $id++
                    Name = $name
                    NORAD_ID = $noradId
                    Group = $group
                    Inclination_deg = $inclination
                    Type = "Satellite"
                }
            }
        }
    }
}

Write-Host "Parsed $id satellites"

# Export to CSV
$csv | Export-Csv -Path $OutputFile -NoTypeInformation

Write-Host "Exported to: $OutputFile"
Write-Host ""
Write-Host "Sample data:"
$csv | Select-Object -First 5 | Format-Table
