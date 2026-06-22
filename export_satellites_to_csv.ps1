# Export satellite data from TLE files to CSV for LEO Simulator
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
$processedCount = 0

# Constants for orbit calculations
$GM = 398600.4418  # Earth gravitational parameter (km^3/s^2)
$earthRadius = 6378.137  # Earth radius (km)

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
                try {
                    # Parse TLE elements from line 2
                    # Columns: 09-16 inclination (degrees)
                    # Columns: 18-25 right ascension of ascending node (degrees)
                    # Columns: 27-33 eccentricity (decimal point assumed, leading zeros)
                    # Columns: 35-42 argument of perigee (degrees)
                    # Columns: 44-51 mean anomaly (degrees)
                    # Columns: 53-63 mean motion (revolutions per day)
                    
                    $inclination = [double]$line2.Substring(8, 8).Trim()
                    $raan = [double]$line2.Substring(17, 8).Trim()  # Right Ascension of Ascending Node
                    
                    # Parse eccentricity (decimal point assumed, format: .0000000)
                    $eccentricityStr = $line2.Substring(26, 7).Trim()
                    if ($eccentricityStr -match "^0\.?") {
                        $eccentricity = [double]"0.$($eccentricityStr.Substring(1))"
                    } else {
                        $eccentricity = [double]$eccentricityStr / 10000000
                    }
                    
                    $argPerigee = [double]$line2.Substring(34, 8).Trim()  # Argument of perigee
                    $meanAnomaly = [double]$line2.Substring(43, 8).Trim()  # Mean anomaly
                    
                    # Parse mean motion (revolutions per day)
                    $meanMotionStr = $line2.Substring(52, 11).Trim()
                    $meanMotion = [double]$meanMotionStr
                    
                    # Convert to orbital parameters
                    # Semi-major axis from mean motion: a = (GM / (n^2))^(1/3)
                    # where n = mean motion in rad/s
                    $n_rad_per_sec = $meanMotion * 2 * [Math]::PI / 86400.0  # Convert rev/day to rad/s
                    $semiMajorAxis = [Math]::Pow($GM / ($n_rad_per_sec * $n_rad_per_sec), 1.0/3.0)
                    
                    # Skip if semi-major axis is too large (likely geostationary or high orbit)
                    if ($semiMajorAxis -gt 50000) {
                        Write-Host "    Skipping $name - likely geostationary (a = $([math]::Round($semiMajorAxis,2)) km)"
                        continue
                    }
                    
                    # Convert angles to radians
                    $incl_rad = $inclination * [Math]::PI / 180.0
                    $raan_rad = $raan * [Math]::PI / 180.0
                    $argPerigee_rad = $argPerigee * [Math]::PI / 180.0
                    $meanAnomaly_rad = $meanAnomaly * [Math]::PI / 180.0
                    
                    # Solve Kepler's equation for eccentric anomaly (using Newton's method)
                    $eccentricAnomaly = $meanAnomaly_rad
                    for ($j = 0; $j -lt 10; $j++) {
                        $delta = ($eccentricAnomaly - $eccentricity * [Math]::Sin($eccentricAnomaly) - $meanAnomaly_rad) / (1 - $eccentricity * [Math]::Cos($eccentricAnomaly))
                        $eccentricAnomaly -= $delta
                        if ([Math]::Abs($delta) -lt 1e-8) { break }
                    }
                    
                    # Calculate true anomaly
                    $trueAnomaly = 2 * [Math]::Atan2(
                        [Math]::Sqrt(1 + $eccentricity) * [Math]::Sin($eccentricAnomaly / 2),
                        [Math]::Sqrt(1 - $eccentricity) * [Math]::Cos($eccentricAnomaly / 2)
                    )
                    
                    # Calculate distance and velocity in orbital plane
                    $distance = $semiMajorAxis * (1 - $eccentricity * [Math]::Cos($eccentricAnomaly))
                    $velocity = [Math]::Sqrt($GM * (2/$distance - 1/$semiMajorAxis))
                    
                    # Position in orbital plane (perifocal coordinates)
                    $x_orb = $distance * [Math]::Cos($trueAnomaly)
                    $y_orb = $distance * [Math]::Sin($trueAnomaly)
                    
                    # Velocity in orbital plane
                    $p = $semiMajorAxis * (1 - $eccentricity * $eccentricity)
                    $h = [Math]::Sqrt($GM * $p)
                    $vx_orb = -($GM/$h) * [Math]::Sin($trueAnomaly)
                    $vy_orb = ($GM/$h) * ($eccentricity + [Math]::Cos($trueAnomaly))
                    
                    # Transform to Earth-centered inertial coordinates (ECI)
                    # Using rotation matrices: Rz(Ω) * Rx(i) * Rz(ω)
                    
                    # Position transformation
                    $x = $x_orb * ([Math]::Cos($argPerigee_rad) * [Math]::Cos($raan_rad) - [Math]::Sin($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Sin($raan_rad)) - 
                         $y_orb * ([Math]::Sin($argPerigee_rad) * [Math]::Cos($raan_rad) + [Math]::Cos($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Sin($raan_rad))
                    
                    $y = $x_orb * ([Math]::Cos($argPerigee_rad) * [Math]::Sin($raan_rad) + [Math]::Sin($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Cos($raan_rad)) + 
                         $y_orb * (-[Math]::Sin($argPerigee_rad) * [Math]::Sin($raan_rad) + [Math]::Cos($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Cos($raan_rad))
                    
                    $z = $x_orb * ([Math]::Sin($argPerigee_rad) * [Math]::Sin($incl_rad)) + 
                         $y_orb * ([Math]::Cos($argPerigee_rad) * [Math]::Sin($incl_rad))
                    
                    # Velocity transformation
                    $vx = $vx_orb * ([Math]::Cos($argPerigee_rad) * [Math]::Cos($raan_rad) - [Math]::Sin($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Sin($raan_rad)) - 
                          $vy_orb * ([Math]::Sin($argPerigee_rad) * [Math]::Cos($raan_rad) + [Math]::Cos($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Sin($raan_rad))
                    
                    $vy = $vx_orb * ([Math]::Cos($argPerigee_rad) * [Math]::Sin($raan_rad) + [Math]::Sin($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Cos($raan_rad)) + 
                          $vy_orb * (-[Math]::Sin($argPerigee_rad) * [Math]::Sin($raan_rad) + [Math]::Cos($argPerigee_rad) * [Math]::Cos($incl_rad) * [Math]::Cos($raan_rad))
                    
                    $vz = $vx_orb * ([Math]::Sin($argPerigee_rad) * [Math]::Sin($incl_rad)) + 
                          $vy_orb * ([Math]::Cos($argPerigee_rad) * [Math]::Sin($incl_rad))
                    
                    # Calculate altitude
                    $altitude = [Math]::Sqrt($x*$x + $y*$y + $z*$z) - $earthRadius
                    
                    # Only include LEO objects (altitude < 2000 km)
                    if ($altitude -gt 2000) {
                        Write-Host "    Skipping $name - altitude $([math]::Round($altitude,2)) km (not LEO)"
                        continue
                    }
                    
                    # Determine object type based on name and group
                    $objectType = "satellite"
                    if ($group -eq "stations" -or $name -match "ISS|Tiangong|TSS") {
                        $objectType = "satellite"
                    } elseif ($name -match "DEB|Rocket|Debris") {
                        $objectType = "debris"
                    }
                    
                    # Calculate size and RCS
                    # Space stations are larger, debris is smaller
                    if ($objectType -eq "satellite") {
                        $sizeMeters = Get-Random -Minimum 2.0 -Maximum 20.0
                    } else {
                        $sizeMeters = Get-Random -Minimum 0.1 -Maximum 2.0
                    }
                    
                    $rcs = $sizeMeters * $sizeMeters * 0.1
                    
                    $csv += [PSCustomObject]@{
                        name = $name.Replace(',', ' ').Replace('"', '')  # Clean name for CSV
                        x = [math]::Round($x, 6)
                        y = [math]::Round($y, 6)
                        z = [math]::Round($z, 6)
                        vx = [math]::Round($vx, 6)
                        vy = [math]::Round($vy, 6)
                        vz = [math]::Round($vz, 6)
                        type = $objectType
                        size_meters = [math]::Round($sizeMeters, 3)
                        rcs = [math]::Round($rcs, 6)
                    }
                    
                    $processedCount++
                    
                    # Progress indicator
                    if ($processedCount % 100 -eq 0) {
                        Write-Host "    Processed $processedCount satellites..."
                    }
                    
                } catch {
                    Write-Host "    Error parsing $name : $_"
                }
            }
        }
    }
}

Write-Host ""
Write-Host "Successfully parsed $processedCount LEO satellites"

if ($processedCount -eq 0) {
    Write-Host "No valid LEO satellites found in TLE files"
    exit 1
}

# Export to CSV
$csv | Export-Csv -Path $OutputFile -NoTypeInformation

Write-Host "Exported to: $OutputFile"
Write-Host ""
Write-Host "Sample data (first 5 records):"
$csv | Select-Object -First 5 | Format-Table name, x, y, z, vx, vy, vz, type

Write-Host ""
Write-Host "CSV file created with the following columns:"
Write-Host "  name, x, y, z, vx, vy, vz, type, size_meters, rcs"
Write-Host ""
Write-Host "You can now import this CSV file into the LEO Simulator"