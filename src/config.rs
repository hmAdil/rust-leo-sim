use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub n_objects: usize,
    pub n_sensors: usize,
    pub dt: f64,           // simulation time step in seconds
    pub steps: usize,
    pub seed: u64,
    pub fov_half_angle: f64,  // sensor field of view half-angle in radians
    pub pos_noise_std: f64,   // position noise standard deviation in km
    pub vel_noise_std: f64,   // velocity noise standard deviation in km/s
    pub gate_threshold: f64,
    pub collision_threshold_km: f64,
    pub collision_horizon_s: f64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            n_objects: 100_000,
            n_sensors: 8,
            dt: 10.0,
            steps: 100,
            seed: 42,
            fov_half_angle: std::f64::consts::PI / 3.0,
            pos_noise_std: 0.5,
            vel_noise_std: 0.005,
            gate_threshold: 5.0,
            collision_threshold_km: 10.0,
            collision_horizon_s: 600.0,
        }
    }
}

impl SimConfig {
    pub fn for_gui() -> Self {
        Self {
            n_objects: 500,    // Start with fewer objects for clear visualization
            steps: usize::MAX,
            ..Self::default()
        }
    }
}
