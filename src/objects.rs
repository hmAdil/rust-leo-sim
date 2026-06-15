use crate::config::{PropagatorType, SimConfig};
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Uniform, Normal};
use rayon::prelude::*;
use sgp4::{Elements, Constants};
use std::f64::consts::PI;

/// Type of space object: functional satellite or debris
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObjectType {
    Satellite,
    Debris,
}

pub struct ObjectPool {
    pub id: Vec<usize>,        // Sequential ID for naming (OBJ_001, etc)
    pub radius: Vec<f64>,      // Orbital radius from Earth's core (km)
    pub incl: Vec<f64>,        // Inclination (radians)
    pub theta0: Vec<f64>,      // Initial true anomaly (radians)
    pub period: Vec<f64>,      // Orbital period (seconds)
    pub pos: Vec<[f64; 3]>,    // Position vector from Earth's core [x, y, z] (km)
    pub vel: Vec<[f64; 3]>,    // Velocity vector [vx, vy, vz] (km/s)
    pub sgp4_constants: Option<Vec<Constants>>,  // Pre-computed SGP4 constants (cached!)
    pub object_type: Vec<ObjectType>, // Satellite or Debris
    pub size_meters: Vec<f64>,        // Characteristic size (meters)
    pub rcs: Vec<f64>,                // Radar cross section (m^2)
    sim_time: f64,             // Current simulation time (seconds)
    propagator: PropagatorType, // Propagation method
}

impl ObjectPool {
    pub fn new(config: &SimConfig) -> Self {
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

        let n = config.n_objects;
        let mut id = Vec::with_capacity(n);
        let mut radius = Vec::with_capacity(n);
        let mut incl = Vec::with_capacity(n);
        let mut theta0 = Vec::with_capacity(n);
        let mut period = Vec::with_capacity(n);
        let mut pos = Vec::with_capacity(n);
        let mut vel = Vec::with_capacity(n);

        let mu = 398600.4418; // Earth's gravitational parameter (km³/s²)

        // For SGP4, we'll also need eccentricity and other elements
        let ecc_dist = Uniform::new(0.0, 0.01);  // Nearly circular for LEO

        let mut sgp4_constants = if config.propagator == PropagatorType::Sgp4 {
            Some(Vec::with_capacity(n))
        } else {
            None
        };

        let mut object_type: Vec<ObjectType> = Vec::new();
        let mut size_meters: Vec<f64> = Vec::new();
        let mut rcs: Vec<f64> = Vec::new();

        if config.stress_test {
            // Stress test mode: clustered objects in dense orbital shells
            // Vectors for new properties
            let mut object_type_vec = Vec::with_capacity(n);
            let mut size_meters_vec = Vec::with_capacity(n);
            let mut rcs_vec = Vec::with_capacity(n);
            Self::create_stress_test_objects(
                &mut rng,
                n,
                &mut id,
                &mut radius,
                &mut incl,
                &mut theta0,
                &mut period,
                &mut pos,
                &mut vel,
                &mut object_type_vec,
                &mut size_meters_vec,
                &mut rcs_vec,
                &mut sgp4_constants,
                mu,
                config,
                &ecc_dist,
            );
            // Assign the new vectors to the struct fields
            object_type = object_type_vec;
            size_meters = size_meters_vec;
            rcs = rcs_vec;
        } else {
            // Normal mode: uniform LEO distribution
            let radius_dist = Uniform::new(6571.0, 8371.0);
            let incl_dist = Uniform::new(0.0, PI);  // Full range of inclinations (0° to 180°)
            let theta0_dist = Uniform::new(0.0, 2.0 * PI);
            // Size distributions
            let satellite_size_dist = Normal::new(config.satellite_size_mean, config.satellite_size_std).unwrap();
            let debris_size_dist = Uniform::new(config.debris_size_min, config.debris_size_max);

            // Vectors for new properties
            let mut object_type_vec = Vec::with_capacity(n);
            let mut size_meters_vec = Vec::with_capacity(n);
            let mut rcs_vec = Vec::with_capacity(n);

            for i in 0..n {
                id.push(i); // Sequential ID for OBJ naming
                let r: f64 = radius_dist.sample(&mut rng);
                let inc = incl_dist.sample(&mut rng);
                let t0 = theta0_dist.sample(&mut rng);
                let p = 2.0 * PI * (r.powi(3) / mu).sqrt();

                radius.push(r);
                incl.push(inc);
                theta0.push(t0);
                period.push(p);

                // Determine object type and size
                let is_satellite = rng.gen::<f64>() < config.satellite_ratio;
                let size = if is_satellite {
                    // Satellite size: normal distribution, ensure positive
                    satellite_size_dist.sample(&mut rng).max(0.01)
                } else {
                    // Debris size: uniform distribution
                    debris_size_dist.sample(&mut rng)
                };
                let object_type = if is_satellite {
                    ObjectType::Satellite
                } else {
                    ObjectType::Debris
                };
                let rcs = size * size; // Simple RCS model

                object_type_vec.push(object_type);
                size_meters_vec.push(size);
                rcs_vec.push(rcs);

                // Position in 3D space with Earth's core at origin
                let x = r * t0.cos() * inc.cos();
                let y = r * t0.sin();
                let z = r * t0.cos() * inc.sin();
                pos.push([x, y, z]);

                // Circular orbital velocity
                let v_mag = (mu / r).sqrt();
                vel.push([
                    -v_mag * t0.sin() * inc.cos(),
                    v_mag * t0.cos(),
                    -v_mag * t0.sin() * inc.sin(),
                ]);

                // Create and cache SGP4 constants if needed
                if let Some(ref mut constants_vec) = sgp4_constants {
                    let ecc = ecc_dist.sample(&mut rng);
                    let raan = Uniform::new(0.0, 2.0 * PI).sample(&mut rng);
                    let argp = Uniform::new(0.0, 2.0 * PI).sample(&mut rng);

                    let mean_motion = (86400.0 / p) as f32;

                    use sgp4::chrono::NaiveDate;
                    let datetime = NaiveDate::from_ymd_opt(2021, 1, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap();

                    let elements = Elements {
                        object_name: Some(format!("OBJ_{:06}", i)),
                        international_designator: None,
                        norad_id: i as u64,
                        datetime,
                        inclination: inc,
                        right_ascension: raan,
                        eccentricity: ecc,
                        argument_of_perigee: argp,
                        mean_anomaly: t0,
                        mean_motion: mean_motion as f64,
                        mean_motion_dot: 0.0,
                        mean_motion_ddot: 0.0,
                        drag_term: 0.0001,
                        revolution_number: 0,
                        classification: sgp4::Classification::Unclassified,
                        ephemeris_type: 0,
                        element_set_number: 999,
                    };

                    if let Ok(constants) = Constants::from_elements(&elements) {
                        constants_vec.push(constants);
                    } else {
                        eprintln!("Warning: Failed to create SGP4 constants for object {}", i);
                    }
                }
            }

            // Assign the new vectors to the struct fields
            object_type = object_type_vec;
            size_meters = size_meters_vec;
            rcs = rcs_vec;
        }

        Self {
            id,
            radius,
            incl,
            theta0,
            period,
            pos,
            vel,
            sgp4_constants,
            object_type,
            size_meters,
            rcs,
            sim_time: 0.0,
            propagator: config.propagator,
        }
    }

    /// Create objects in clustered orbital shells for stress testing
    fn create_stress_test_objects(
        rng: &mut rand::rngs::StdRng,
        n: usize,
        id: &mut Vec<usize>,
        radius: &mut Vec<f64>,
        incl: &mut Vec<f64>,
        theta0: &mut Vec<f64>,
        period: &mut Vec<f64>,
        pos: &mut Vec<[f64; 3]>,
        vel: &mut Vec<[f64; 3]>,
        object_type: &mut Vec<ObjectType>,
        size_meters: &mut Vec<f64>,
        rcs: &mut Vec<f64>,
        sgp4_constants: &mut Option<Vec<Constants>>,
        mu: f64,
        config: &SimConfig, // We need config for the size distributions
        ecc_dist: &Uniform<f64>,
    ) {
        // 5 orbital shells at specific altitudes (km from Earth's core)
        let shell_radii = [6921.0, 6971.0, 7021.0, 7071.0, 7121.0]; // 550, 600, 650, 700, 750 km altitude
        let objects_per_shell = n / shell_radii.len();

        // Generate 3 random hotspots per shell
        let mut hotspots: Vec<([f64; 3], f64)> = Vec::new(); // (center, radius)
        for shell_r in &shell_radii {
            for _ in 0..3 {
                // Random point on sphere
                let lat = Uniform::new(-PI/2.0, PI/2.0).sample(rng);
                let lon = Uniform::new(0.0, 2.0 * PI).sample(rng);
                let x = shell_r * lat.cos() * lon.cos();
                let y = shell_r * lat.cos() * lon.sin();
                let z = shell_r * lat.sin();
                hotspots.push(([x, y, z], 50.0)); // 50 km radius hotspot
            }
        }

        // Size distributions
        let satellite_size_dist = Normal::new(config.satellite_size_mean, config.satellite_size_std).unwrap();
        let debris_size_dist = Uniform::new(config.debris_size_min, config.debris_size_max);

        for i in 0..n {
            id.push(i);

            // Select shell
            let shell_idx = i / objects_per_shell;
            let shell_idx = shell_idx.min(shell_radii.len() - 1);
            let r = shell_radii[shell_idx];

            // 70% clustered, 30% uniform
            let use_cluster = rng.sample(&Uniform::new(0, 10)) < 7;

            let (inc, t0) = if use_cluster {
                // Pick a random hotspot and place object near it
                let hotspot_idx = (i % 3) + shell_idx * 3;
                let (center, _hotspot_r) = hotspots[hotspot_idx];

                // Generate random offset within hotspot
                let offset_dist = Normal::new(0.0, 25.0).unwrap(); // 25 km std dev
                let offset_x = offset_dist.sample(rng);
                let offset_y = offset_dist.sample(rng);
                let offset_z = offset_dist.sample(rng);

                // Convert position back to spherical coords
                let x = center[0] + offset_x;
                let y = center[1] + offset_y;
                let z = center[2] + offset_z;

                // Clamp to shell radius
                let actual_r = (x*x + y*y + z*z).sqrt();
                let scale = r / actual_r;
                let x = x * scale;
                let y = y * scale;
                let z = z * scale;

                // Get inclination and true anomaly from position
                let t0 = (x != 0.0 || z != 0.0).then(|| (x*x + z*z).sqrt())
                    .map(|r_xz| (y / r_xz).atan2(x / r_xz))
                    .unwrap_or(0.0);
                let inc = (x != 0.0 || y != 0.0 || z != 0.0).then(|| z / (x*x + y*y + z*z).sqrt())
                    .map(|cos_inc| cos_inc.acos())
                    .unwrap_or(0.0);

                (inc, t0)
            } else {
                // Uniform distribution
                let inc = rng.sample(&Uniform::new(0.0, PI));
                let t0 = rng.sample(&Uniform::new(0.0, 2.0 * PI));
                (inc, t0)
            };

            // Determine object type and size
            let is_satellite = rng.gen::<f64>() < config.satellite_ratio;
            let size = if is_satellite {
                // Satellite size: normal distribution, ensure positive
                satellite_size_dist.sample(rng).max(0.01)
            } else {
                // Debris size: uniform distribution
                debris_size_dist.sample(rng)
            };
            let object_type_val = if is_satellite {
                ObjectType::Satellite
            } else {
                ObjectType::Debris
            };
            let rcs_val = size * size; // Simple RCS model

            object_type.push(object_type_val);
            size_meters.push(size);
            rcs.push(rcs_val);

            let p = 2.0 * PI * (r.powi(3) / mu).sqrt();

            radius.push(r);
            incl.push(inc);
            theta0.push(t0);
            period.push(p);

            // Position in 3D space
            let x = r * t0.cos() * inc.cos();
            let y = r * t0.sin();
            let z = r * t0.cos() * inc.sin();
            pos.push([x, y, z]);

            // Circular orbital velocity
            let v_mag = (mu / r).sqrt();
            vel.push([
                -v_mag * t0.sin() * inc.cos(),
                v_mag * t0.cos(),
                -v_mag * t0.sin() * inc.sin(),
            ]);

            // SGP4 constants
            if let Some(ref mut constants_vec) = sgp4_constants {
                let ecc = ecc_dist.sample(rng);
                let raan = Uniform::new(0.0, 2.0 * PI).sample(rng);
                let argp = Uniform::new(0.0, 2.0 * PI).sample(rng);

                let mean_motion = (86400.0 / p) as f32;

                use sgp4::chrono::NaiveDate;
                let datetime = NaiveDate::from_ymd_opt(2021, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap();

                let elements = Elements {
                    object_name: Some(format!("OBJ_{:06}", i)),
                    international_designator: None,
                    norad_id: i as u64,
                    datetime,
                    inclination: inc,
                    right_ascension: raan,
                    eccentricity: ecc,
                    argument_of_perigee: argp,
                    mean_anomaly: t0,
                    mean_motion: mean_motion as f64,
                    mean_motion_dot: 0.0,
                    mean_motion_ddot: 0.0,
                    drag_term: 0.0001,
                    revolution_number: 0,
                    classification: sgp4::Classification::Unclassified,
                    ephemeris_type: 0,
                    element_set_number: 999,
                };

                if let Ok(constants) = Constants::from_elements(&elements) {
                    constants_vec.push(constants);
                } else {
                    eprintln!("Warning: Failed to create SGP4 constants for object {}", i);
                }
            }
        }
    }

    pub fn propagate(&mut self, dt: f64) {
        self.sim_time += dt;
        let t = self.sim_time;

        match self.propagator {
            PropagatorType::SimpleKeplerian => {
                self.propagate_keplerian(t);
            }
            PropagatorType::Sgp4 => {
                self.propagate_sgp4(dt);
            }
        }
    }

    fn propagate_keplerian(&mut self, t: f64) {
        let mu = 398600.4418; // Earth's gravitational parameter

        // Parallel propagation of all objects in 3D space
        self.pos
            .par_iter_mut()
            .zip(self.vel.par_iter_mut())
            .zip(&self.radius)
            .zip(&self.incl)
            .zip(&self.theta0)
            .zip(&self.period)
            .for_each(|(((((pos, vel), &r), &incl), &t0), &p)| {
                let theta = t0 + (2.0 * PI / p) * t;
                
                // Update position vector from Earth's core
                pos[0] = r * theta.cos() * incl.cos();
                pos[1] = r * theta.sin();
                pos[2] = r * theta.cos() * incl.sin();

                // Update velocity vector
                let v_mag = (mu / r).sqrt();
                vel[0] = -v_mag * theta.sin() * incl.cos();
                vel[1] = v_mag * theta.cos();
                vel[2] = -v_mag * theta.sin() * incl.sin();
            });
    }

    fn propagate_sgp4(&mut self, _dt: f64) {
        if let Some(ref constants_vec) = self.sgp4_constants {
            let minutes_since_epoch = self.sim_time / 60.0;
            
            // Parallel propagation using cached constants - MUCH faster!
            self.pos
                .par_iter_mut()
                .zip(self.vel.par_iter_mut())
                .zip(constants_vec.par_iter())
                .for_each(|((pos, vel), constants)| {
                    // Just propagate - constants are already computed!
                    if let Ok(prediction) = constants.propagate(sgp4::MinutesSinceEpoch(minutes_since_epoch)) {
                        pos[0] = prediction.position[0];
                        pos[1] = prediction.position[1];
                        pos[2] = prediction.position[2];
                        vel[0] = prediction.velocity[0];
                        vel[1] = prediction.velocity[1];
                        vel[2] = prediction.velocity[2];
                    }
                });
        }
    }

    pub fn get_position(&self, idx: usize) -> [f64; 3] {
        self.pos[idx]
    }

    pub fn get_velocity(&self, idx: usize) -> [f64; 3] {
        self.vel[idx]
    }

    pub fn get_id(&self, idx: usize) -> usize {
        self.id[idx]
    }

    pub fn get_name(&self, idx: usize) -> String {
        format!("OBJ_{:06}", self.id[idx])
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.id.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    /// Get the object type (Satellite or Debris)
    pub fn get_object_type(&self, idx: usize) -> ObjectType {
        self.object_type[idx]
    }

    /// Get the size of the object in meters
    pub fn get_size_meters(&self, idx: usize) -> f64 {
        self.size_meters[idx]
    }

    /// Get the radar cross section in m^2
    pub fn get_rcs(&self, idx: usize) -> f64 {
        self.rcs[idx]
    }
}