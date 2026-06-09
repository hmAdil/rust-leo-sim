use crate::config::{PropagatorType, SimConfig};
use rand::SeedableRng;
use rand_distr::{Distribution, Uniform};
use rayon::prelude::*;
use sgp4::{Elements, Constants};
use std::f64::consts::PI;

pub struct ObjectPool {
    pub id: Vec<usize>,        // Sequential ID for naming (OBJ_001, etc)
    pub radius: Vec<f64>,      // Orbital radius from Earth's core (km)
    pub incl: Vec<f64>,        // Inclination (radians)
    pub theta0: Vec<f64>,      // Initial true anomaly (radians)
    pub period: Vec<f64>,      // Orbital period (seconds)
    pub pos: Vec<[f64; 3]>,    // Position vector from Earth's core [x, y, z] (km)
    pub vel: Vec<[f64; 3]>,    // Velocity vector [vx, vy, vz] (km/s)
    pub sgp4_constants: Option<Vec<Constants>>,  // Pre-computed SGP4 constants (cached!)
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

        // LEO range with more variation: 200-2000 km altitude above Earth surface (6371 km radius)
        // This creates objects at various orbital heights for more diverse visualization
        let radius_dist = Uniform::new(6571.0, 8371.0);
        let incl_dist = Uniform::new(0.0, PI);  // Full range of inclinations (0° to 180°)
        let theta0_dist = Uniform::new(0.0, 2.0 * PI);
        let mu = 398600.4418; // Earth's gravitational parameter (km³/s²)

        // For SGP4, we'll also need eccentricity and other elements
        let ecc_dist = Uniform::new(0.0, 0.01);  // Nearly circular for LEO

        let mut sgp4_constants = if config.propagator == PropagatorType::Sgp4 {
            Some(Vec::with_capacity(n))
        } else {
            None
        };

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
                let raan = Uniform::new(0.0, 2.0 * PI).sample(&mut rng); // Right ascension of ascending node
                let argp = Uniform::new(0.0, 2.0 * PI).sample(&mut rng); // Argument of perigee
                
                // Convert orbital parameters to TLE-like elements
                // Mean motion in revolutions per day
                let mean_motion = (86400.0 / p) as f32;
                
                // Create SGP4 elements using the current API
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
                    drag_term: 0.0001,  // Small drag for LEO
                    revolution_number: 0,
                    classification: sgp4::Classification::Unclassified,
                    ephemeris_type: 0,
                    element_set_number: 999,
                };
                
                // Pre-compute constants (this is the expensive part - do it once!)
                if let Ok(constants) = Constants::from_elements(&elements) {
                    constants_vec.push(constants);
                } else {
                    // Fallback if constants creation fails
                    eprintln!("Warning: Failed to create SGP4 constants for object {}", i);
                }
            }
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
            sim_time: 0.0,
            propagator: config.propagator,
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
}
