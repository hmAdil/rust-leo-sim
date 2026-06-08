use crate::objects::ObjectPool;
use crate::tracker::Track;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Represents a detected object entry in the catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub object_name: String,           // OBJ_XXXXXX format
    pub first_detection_time: f64,     // Timestamp of first detection (seconds)
    pub last_detection_time: f64,      // Timestamp of last detection (seconds)
    pub position: [f64; 3],            // Last known position from Earth's core [x, y, z] (km)
    pub velocity: [f64; 3],            // Last known velocity [vx, vy, vz] (km/s)
    pub detection_count: usize,        // Number of times detected
    pub tracking_confidence: f32,      // Tracking confidence (0.0 - 1.0)
}

/// Catalog of detected objects (only objects seen by observatories)
pub struct ObjectCatalog {
    entries: HashMap<String, CatalogEntry>,
}

impl ObjectCatalog {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Update catalog with detected tracks
    pub fn update_from_tracks(
        &mut self,
        tracks: &[&Track],
        objects: &ObjectPool,
        current_time: f64,
    ) {
        for track in tracks {
            let object_name = objects.get_name(track.object_id);
            
            self.entries
                .entry(object_name.clone())
                .and_modify(|entry| {
                    entry.last_detection_time = current_time;
                    entry.position = track.predicted_pos;
                    entry.velocity = track.predicted_vel;
                    entry.detection_count += 1;
                    entry.tracking_confidence = track.confidence;
                })
                .or_insert(CatalogEntry {
                    object_name,
                    first_detection_time: current_time,
                    last_detection_time: current_time,
                    position: track.predicted_pos,
                    velocity: track.predicted_vel,
                    detection_count: 1,
                    tracking_confidence: track.confidence,
                });
        }
    }

    /// Export catalog to CSV file
    pub fn export_to_csv<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        // Write CSV header
        writeln!(
            file,
            "Object_Name,First_Detection_Time_s,Last_Detection_Time_s,Position_X_km,Position_Y_km,Position_Z_km,Velocity_X_km_s,Velocity_Y_km_s,Velocity_Z_km_s,Detection_Count,Tracking_Confidence"
        )?;

        // Sort entries by object name for consistent output
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| a.object_name.cmp(&b.object_name));

        // Write each entry
        for entry in entries {
            writeln!(
                file,
                "{},{:.2},{:.2},{:.6},{:.6},{:.6},{:.8},{:.8},{:.8},{},{:.4}",
                entry.object_name,
                entry.first_detection_time,
                entry.last_detection_time,
                entry.position[0],
                entry.position[1],
                entry.position[2],
                entry.velocity[0],
                entry.velocity[1],
                entry.velocity[2],
                entry.detection_count,
                entry.tracking_confidence
            )?;
        }

        Ok(())
    }

    /// Get number of cataloged objects
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if catalog is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all catalog entries
    #[allow(dead_code)]
    pub fn entries(&self) -> &HashMap<String, CatalogEntry> {
        &self.entries
    }
}
