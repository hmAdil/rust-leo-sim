use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PropagatorType {
    SimpleKeplerian,  // Current simple circular orbits
    Sgp4,            // SGP4/SDP4 propagator for realistic orbits
}

/// Tracker type for data association
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrackerType {
    NearestNeighbor,
    Jpda,
}

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
    pub propagator: PropagatorType,  // Choose propagation method
    pub tracker_type: TrackerType,   // Choose tracking algorithm
    pub stress_test: bool,           // Use clustered object distribution
    // Satellite/debris categorization and sizing
    pub satellite_ratio: f64,        // Fraction of objects that are satellites (0.0-1.0)
    pub satellite_size_mean: f64,    // Mean size for satellites in meters
    pub satellite_size_std: f64,     // Standard deviation for satellite size in meters
    pub debris_size_min: f64,        // Minimum debris size in meters
    pub debris_size_max: f64,        // Maximum debris size in meters
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            n_objects: 100_000,
            n_sensors: 20,  // More observatories for comprehensive global coverage
            dt: 30.0,       // Slower time step for clearer visualization
            steps: 100,
            seed: 42,
            fov_half_angle: std::f64::consts::PI / 3.0,
            pos_noise_std: 0.5,
            vel_noise_std: 0.005,
            gate_threshold: 17.0,
            collision_threshold_km: 10.0,
            collision_horizon_s: 600.0,
            propagator: PropagatorType::SimpleKeplerian,  // Default to simple for speed
            tracker_type: TrackerType::NearestNeighbor,  // Default to nearest-neighbor
            stress_test: false,                          // Default to uniform distribution
            // Satellite/debris categorization and sizing
            satellite_ratio: 0.15,        // 15% satellites, 85% debris
            satellite_size_mean: 2.0,     // meters
            satellite_size_std: 1.0,      // meters
            debris_size_min: 0.01,        // 1 cm
            debris_size_max: 2.0,         // meters
        }
    }
}

impl SimConfig {
    pub fn for_gui() -> Self {
        Self {
            n_objects: 300,    // Moderate number for clear visualization
            n_sensors: 24,     // Even more sensors for complete global coverage
            dt: 20.0,          // Slower updates for easier tracking
            steps: usize::MAX,
            ..Self::default()
        }
    }
}