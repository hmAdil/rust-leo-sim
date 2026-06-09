use crate::config::SimConfig;
use crate::objects::ObjectPool;
use crate::spatial::SpatialIndex;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRecord {
    pub sensor_id: u32,
    pub timestamp_ms: u64,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub snr: f32,
}

pub struct SensorObservationBatch {
    pub observations: Vec<ObservationRecord>,
    pub object_indices: Vec<usize>,
}

/// Ground-based observatory with vision cone
pub struct GroundStation {
    pub id: u32,
    #[allow(dead_code)]
    pub name: String,
    pub position: [f64; 3],        // Position on Earth's surface (from core) [x, y, z] (km)
    pub zenith: [f64; 3],          // Zenith direction (pointing away from Earth's center)
    pub fov_half_angle: f64,       // Field of view half-angle (radians) - defines vision cone
    pos_noise: Normal<f64>,
    vel_noise: Normal<f64>,
    rng: rand::rngs::StdRng,
    observation_buffer: Vec<ObservationRecord>,
    object_index_buffer: Vec<usize>,
}

impl GroundStation {
    pub fn new(id: u32, lat: f64, lon: f64, config: &SimConfig) -> Self {
        let r_earth = 6371.0; // Earth's radius (km)
        
        // Position on Earth's surface in 3D space (Earth's core as origin)
        // lat and lon are in radians
        // Standard spherical to Cartesian conversion:
        // x = r * cos(lat) * cos(lon)
        // y = r * cos(lat) * sin(lon)  
        // z = r * sin(lat)
        let x = r_earth * lat.cos() * lon.cos();
        let y = r_earth * lat.cos() * lon.sin();
        let z = r_earth * lat.sin();

        // Zenith points away from Earth's center (outward normal)
        let zenith_norm = (x * x + y * y + z * z).sqrt();
        let zenith = [x / zenith_norm, y / zenith_norm, z / zenith_norm];

        // Generate observatory name
        let name = format!("OBS_{:02}", id);

        Self {
            id,
            name,
            position: [x, y, z],
            zenith,
            fov_half_angle: config.fov_half_angle,
            pos_noise: Normal::new(0.0, config.pos_noise_std).unwrap(),
            vel_noise: Normal::new(0.0, config.vel_noise_std).unwrap(),
            rng: rand::rngs::StdRng::seed_from_u64(config.seed.wrapping_add(id as u64 + 1)),
            observation_buffer: Vec::with_capacity(4096),
            object_index_buffer: Vec::with_capacity(4096),
        }
    }

    /// Observe objects within the vision cone
    pub fn observe(
        &mut self,
        objects: &ObjectPool,
        spatial_index: &SpatialIndex,
        timestamp_ms: u64,
    ) -> SensorObservationBatch {
        self.observation_buffer.clear();
        self.object_index_buffer.clear();

        let candidate_indices = spatial_index.query_nearby(&self.position, self.fov_half_angle);
        let cos_half_angle = self.fov_half_angle.cos();

        for &idx in &candidate_indices {
            let obj_pos = objects.get_position(idx);
            let obj_vel = objects.get_velocity(idx);

            // Vector from observatory to object
            let dx = obj_pos[0] - self.position[0];
            let dy = obj_pos[1] - self.position[1];
            let dz = obj_pos[2] - self.position[2];

            // Check if object is above horizon (dot product with zenith > 0)
            let dot = dx * self.zenith[0] + dy * self.zenith[1] + dz * self.zenith[2];
            if dot <= 0.0 {
                continue; // Object is below horizon
            }

            // Check if object is within vision cone (FOV)
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            let cos_angle = dot / dist;
            if cos_angle < cos_half_angle {
                continue; // Object is outside FOV cone
            }

            // Object is detected - add to observations with noise
            self.observation_buffer.push(ObservationRecord {
                sensor_id: self.id,
                timestamp_ms,
                position: [
                    obj_pos[0] + self.pos_noise.sample(&mut self.rng),
                    obj_pos[1] + self.pos_noise.sample(&mut self.rng),
                    obj_pos[2] + self.pos_noise.sample(&mut self.rng),
                ],
                velocity: [
                    obj_vel[0] + self.vel_noise.sample(&mut self.rng),
                    obj_vel[1] + self.vel_noise.sample(&mut self.rng),
                    obj_vel[2] + self.vel_noise.sample(&mut self.rng),
                ],
                snr: ((1.0 / dist) * 1e6) as f32,
            });
            self.object_index_buffer.push(idx);
        }

        SensorObservationBatch {
            observations: std::mem::take(&mut self.observation_buffer),
            object_indices: std::mem::take(&mut self.object_index_buffer),
        }
    }
}

/// Create observatories distributed evenly around the globe using sunflower seed arrangement
pub fn create_sensors(config: &SimConfig) -> Vec<GroundStation> {
    let mut sensors = Vec::with_capacity(config.n_sensors);
    
    // Sunflower seed arrangement for optimal sphere coverage
    let n = config.n_sensors as f64;
    let golden_ratio = (1.0 + 5.0f64.sqrt()) / 2.0;
    
    for i in 0..config.n_sensors {
        let k = i as f64 + 0.5; // Offset by 0.5 to avoid poles
        
        // Latitude: evenly distributed from -π/2 to π/2
        let lat = (2.0 * k / n - 1.0).asin();
        
        // Longitude: golden angle spiral for even azimuthal distribution
        let lon = 2.0 * PI * k / golden_ratio;
        
        sensors.push(GroundStation::new(i as u32, lat, lon, config));
    }

    sensors
}
