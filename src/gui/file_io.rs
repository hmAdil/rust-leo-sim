use crate::objects::{ObjectType, ObjectPool};
use crate::config::SimConfig;
use std::fs::File;
use std::io::BufRead;
use std::path::Path;

/// CSV object structure for importing
#[derive(Debug, Clone)]
pub struct CsvObject {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub object_type: Option<ObjectType>,
    pub size_meters: Option<f64>,
    pub rcs: Option<f64>,
}

/// Import catalog from CSV file
pub fn import_csv_catalog(path: &Path) -> Result<Vec<CsvObject>, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open CSV file: {}", e))?;
    
    let reader = std::io::BufReader::new(file);
    let mut lines = reader.lines();
    
    // Parse header
    let header = lines.next()
        .ok_or("Empty CSV file".to_string())?
        .map_err(|e| format!("Failed to read header: {}", e))?;
    let headers: Vec<&str> = header.split(',').map(|h| h.trim()).collect();
    
    // Validate required columns
    let required_columns = ["name", "x", "y", "z", "vx", "vy", "vz"];
    for req in &required_columns {
        if !headers.iter().any(|h| h.to_lowercase() == *req) {
            return Err(format!("Missing required column: {}", req));
        }
    }
    
    // Find column indices
    let get_col_idx = |name: &str| -> usize {
        headers.iter().position(|h| h.to_lowercase() == name)
            .unwrap_or(0)
    };
    
    let name_idx = get_col_idx("name");
    let x_idx = get_col_idx("x");
    let y_idx = get_col_idx("y");
    let z_idx = get_col_idx("z");
    let vx_idx = get_col_idx("vx");
    let vy_idx = get_col_idx("vy");
    let vz_idx = get_col_idx("vz");
    
    let type_idx = headers.iter().position(|h| h.to_lowercase() == "type");
    let size_idx = headers.iter().position(|h| h.to_lowercase() == "size_meters");
    let rcs_idx = headers.iter().position(|h| h.to_lowercase() == "rcs");
    
    let mut objects = Vec::new();
    let mut line_num = 1;
    
    for line in lines {
        line_num += 1;
        let line = line.map_err(|e| format!("Failed to read line {}: {}", line_num, e))?;
        
        let fields: Vec<&str> = line.split(',').map(|f| f.trim()).collect();
        
        // Parse position (required)
        let x = fields.get(x_idx)
            .ok_or_else(|| format!("Missing x at line {}", line_num))?
            .parse::<f64>()
            .map_err(|e| format!("Invalid x value at line {}: {}", line_num, e))?;
        
        let y = fields.get(y_idx)
            .ok_or_else(|| format!("Missing y at line {}", line_num))?
            .parse::<f64>()
            .map_err(|e| format!("Invalid y value at line {}: {}", line_num, e))?;
        
        let z = fields.get(z_idx)
            .ok_or_else(|| format!("Missing z at line {}", line_num))?
            .parse::<f64>()
            .map_err(|e| format!("Invalid z value at line {}: {}", line_num, e))?;
        
        // Parse velocity (required)
        let vx = fields.get(vx_idx)
            .ok_or_else(|| format!("Missing vx at line {}", line_num))?
            .parse::<f64>()
            .map_err(|e| format!("Invalid vx value at line {}: {}", line_num, e))?;
        
        let vy = fields.get(vy_idx)
            .ok_or_else(|| format!("Missing vy at line {}", line_num))?
            .parse::<f64>()
            .map_err(|e| format!("Invalid vy value at line {}: {}", line_num, e))?;
        
        let vz = fields.get(vz_idx)
            .ok_or_else(|| format!("Missing vz at line {}", line_num))?
            .parse::<f64>()
            .map_err(|e| format!("Invalid vz value at line {}: {}", line_num, e))?;
        
        // Parse optional fields
        let name = fields.get(name_idx)
            .unwrap_or(&"UNKNOWN")
            .to_string();
        
        let object_type = match type_idx.and_then(|idx| fields.get(idx)) {
            Some(v) => match v.to_lowercase().as_str() {
                "satellite" => Some(ObjectType::Satellite),
                "debris" => Some(ObjectType::Debris),
                _ => None,
            },
            None => None,
        };
        
        let size_meters = size_idx.and_then(|idx| fields.get(idx))
            .and_then(|v| v.parse::<f64>().ok());
        
        let rcs = rcs_idx.and_then(|idx| fields.get(idx))
            .and_then(|v| v.parse::<f64>().ok());
        
        objects.push(CsvObject {
            name,
            x, y, z, vx, vy, vz,
            object_type,
            size_meters,
            rcs,
        });
    }
    
    Ok(objects)
}

/// Convert CSV objects to ObjectPool
pub fn csv_to_object_pool(csv_objects: &[CsvObject], config: &SimConfig) -> ObjectPool {
    let n = csv_objects.len();
    
    let id: Vec<usize> = (0..n).collect();
    let names: Vec<String> = csv_objects.iter().map(|o| o.name.clone()).collect();
    
    let mut radius = Vec::with_capacity(n);
    let mut incl = Vec::with_capacity(n);
    let mut theta0 = Vec::with_capacity(n);
    let mut period = Vec::with_capacity(n);
    let pos: Vec<[f64; 3]> = csv_objects.iter().map(|o| [o.x, o.y, o.z]).collect();
    let vel: Vec<[f64; 3]> = csv_objects.iter().map(|o| [o.vx, o.vy, o.vz]).collect();
    let object_type: Vec<ObjectType> = csv_objects.iter()
        .map(|o| o.object_type.unwrap_or(ObjectType::Satellite))
        .collect();
    let size_meters: Vec<f64> = csv_objects.iter()
        .map(|o| o.size_meters.unwrap_or(2.0))
        .collect();
    let rcs: Vec<f64> = csv_objects.iter()
        .map(|o| o.rcs.unwrap_or_else(|| size_meters.iter().map(|&s| s * s).collect::<Vec<_>>().get(0).copied().unwrap_or(1.0)))
        .collect();
    
    let mu = 398600.4418;
    
    // Calculate orbital parameters from position/velocity
    for pos in pos.iter() {
        let r = (pos[0].powi(2) + pos[1].powi(2) + pos[2].powi(2)).sqrt();
        radius.push(r);
        
        // Calculate inclination
        let inc = (pos[2] / r).asin().abs();
        incl.push(inc);
        
        // Calculate true anomaly
        let theta = if pos[0] != 0.0 || pos[2] != 0.0 {
            (pos[1] / (pos[0].powi(2) + pos[2].powi(2)).sqrt()).atan()
        } else {
            0.0
        };
        theta0.push(theta);
        
        // Calculate period (assuming circular orbit)
        let p = 2.0 * std::f64::consts::PI * (r.powi(3) / mu).sqrt();
        period.push(p);
    }
    
    ObjectPool {
        id,
        names,
        radius,
        incl,
        theta0,
        period,
        pos,
        vel,
        sgp4_constants: None,
        object_type,
        size_meters,
        rcs,
        sim_time: 0.0,
        propagator: config.propagator,
    }
}