/// CelesTrak integration - download and parse real satellite TLE data
use reqwest::blocking;
use reqwest::header;
use sgp4::{Elements, Constants};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

const DATASET_DIR: &str = "dataset";

/// Download TLE data from CelesTrak for a specific satellite group
/// Caches data locally in dataset/ folder
pub fn download_tle_data(group: &str) -> Result<String, Box<dyn Error>> {
    // Create dataset directory if it doesn't exist
    fs::create_dir_all(DATASET_DIR)?;
    
    let cache_file = format!("{}/{}.txt", DATASET_DIR, group);
    
    // Try to read from cache first
    if Path::new(&cache_file).exists() {
        eprintln!("📂 Loading cached TLE data from: {}", cache_file);
        return Ok(fs::read_to_string(&cache_file)?);
    }
    
    // Download if not cached - use the supplemental GP data format which is more reliable
    let url = format!(
        "https://celestrak.org/NORAD/elements/gp.php?GROUP={}&FORMAT=tle",
        group
    );
    
    eprintln!("🌐 Downloading TLE data from CelesTrak: GROUP={}", group);
    
    // Create a client with proper headers to avoid 403
    let client = blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;
    
    let response = client.get(&url)
        .header(header::ACCEPT, "text/plain")
        .send()?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {} - Try using 'stations' or 'gps-ops' groups which are smaller", response.status()).into());
    }
    
    let text = response.text()?;
    
    if text.is_empty() {
        return Err("Received empty response from CelesTrak".into());
    }
    
    eprintln!("✓ Downloaded {} bytes of TLE data", text.len());
    
    // Cache the data
    eprintln!("💾 Caching TLE data to: {}", cache_file);
    let mut file = fs::File::create(&cache_file)?;
    file.write_all(text.as_bytes())?;
    
    Ok(text)
}

/// Parse TLE text data into SGP4 Elements
pub fn parse_tle_to_elements(tle_text: &str) -> Result<Vec<Elements>, Box<dyn Error>> {
    let lines: Vec<&str> = tle_text.lines().collect();
    let mut elements_vec = Vec::new();
    
    // TLE format: 3 lines per satellite (name + 2 TLE lines)
    let mut i = 0;
    while i + 2 < lines.len() {
        let name = lines[i].trim();
        let line1 = lines[i + 1].trim();
        let line2 = lines[i + 2].trim();
        
        // Validate TLE lines start with "1 " and "2 "
        if line1.starts_with("1 ") && line2.starts_with("2 ") {
            match Elements::from_tle(Some(name.to_string()), line1.as_bytes(), line2.as_bytes()) {
                Ok(elements) => {
                    elements_vec.push(elements);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse TLE for {}: {}", name, e);
                }
            }
        }
        
        i += 3;
    }
    
    eprintln!("Successfully parsed {} satellites from TLE data", elements_vec.len());
    Ok(elements_vec)
}

/// Parse TLE text and create SGP4 Constants (pre-computed for fast propagation)
pub fn parse_tle_to_constants(tle_text: &str) -> Result<Vec<(String, Constants)>, Box<dyn Error>> {
    let elements = parse_tle_to_elements(tle_text)?;
    let mut constants_vec = Vec::new();
    
    for elem in elements {
        let name = elem.object_name.clone().unwrap_or_else(|| format!("SAT_{}", elem.norad_id));
        match Constants::from_elements(&elem) {
            Ok(constants) => {
                constants_vec.push((name, constants));
            }
            Err(e) => {
                eprintln!("Warning: Failed to create SGP4 constants: {}", e);
            }
        }
    }
    
    eprintln!("Created SGP4 constants for {} satellites", constants_vec.len());
    Ok(constants_vec)
}

/// Download and parse TLE data in one step
pub fn fetch_satellites(group: &str) -> Result<Vec<(String, Constants)>, Box<dyn Error>> {
    let tle_text = download_tle_data(group)?;
    parse_tle_to_constants(&tle_text)
}

/// Get list of available CelesTrak groups
pub fn available_groups() -> Vec<(&'static str, &'static str)> {
    vec![
        ("stations", "Space stations (ISS, CSS, etc.)"),
        ("active", "All active satellites"),
        ("starlink", "Starlink constellation"),
        ("gps-ops", "GPS operational satellites"),
        ("weather", "Weather satellites"),
        ("galileo", "Galileo navigation satellites"),
        ("glonass-ops", "GLONASS operational satellites"),
        ("beidou", "BeiDou navigation satellites"),
        ("amateur", "Amateur radio satellites"),
        ("cubesats", "CubeSats"),
        ("geo", "Geostationary satellites"),
        ("military", "Military satellites"),
        ("science", "Science satellites"),
        ("engineering", "Engineering satellites"),
    ]
}

/// Fetch satellites from multiple groups and combine them
pub fn fetch_multiple_groups(groups: &[&str]) -> Result<Vec<(String, Constants)>, Box<dyn Error>> {
    let mut all_satellites = Vec::new();
    
    for group in groups {
        eprintln!("📡 Loading group: {}", group);
        match fetch_satellites(group) {
            Ok(mut sats) => {
                eprintln!("  ✓ Loaded {} satellites from {}", sats.len(), group);
                all_satellites.append(&mut sats);
            }
            Err(e) => {
                eprintln!("  ⚠ Failed to load {}: {}", group, e);
            }
        }
    }
    
    eprintln!("🛰  Total satellites loaded: {}", all_satellites.len());
    Ok(all_satellites)
}

/// Export satellites to CSV format
pub fn export_to_csv(satellites: &[(String, Constants)], filename: &str) -> Result<(), Box<dyn Error>> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create(filename)?;
    
    // Write CSV header
    writeln!(file, "Name,NORAD_ID,Epoch,Inclination_deg,RAAN_deg,Eccentricity,ArgPerigee_deg,MeanAnomaly_deg,MeanMotion_revPerDay")?;
    
    // Initial propagation to get orbital elements
    let minutes_since_epoch = 0.0;
    
    for (name, constants) in satellites {
        if let Ok(_prediction) = constants.propagate(sgp4::MinutesSinceEpoch(minutes_since_epoch)) {
            // Note: SGP4 Constants don't expose orbital elements directly
            // We'd need to store the Elements separately to export them
            // For now, just export name and basic info
            writeln!(file, "\"{}\",Unknown,Unknown,Unknown,Unknown,Unknown,Unknown,Unknown,Unknown", 
                name.replace("\"", "\"\""))?;
        }
    }
    
    eprintln!("💾 Exported {} satellites to {}", satellites.len(), filename);
    Ok(())
}

/// Helper to get common satellite groups for real-time mode
pub fn get_realtime_groups() -> Vec<&'static str> {
    vec![
        "stations",   // ~25 objects - ISS, Tiangong, etc.
        "gps-ops",    // ~30 objects - GPS constellation
        "weather",    // ~60 objects - NOAA, GOES, etc.
    ]
}

/// Helper to get a larger set of satellites
pub fn get_extended_groups() -> Vec<&'static str> {
    vec![
        "stations",
        "gps-ops",
        "galileo",
        "glonass-ops",
        "weather",
        "amateur",
        "science",
    ]
}
