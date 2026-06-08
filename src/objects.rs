use crate::config::SimConfig;
use rand::SeedableRng;
use rand_distr::{Distribution, Uniform};
use rayon::prelude::*;
use std::f64::consts::PI;

pub struct ObjectPool {
    pub id: Vec<usize>,        // Sequential ID for naming (OBJ_001, etc)
    pub radius: Vec<f64>,      // Orbital radius from Earth's core (km)
    pub incl: Vec<f64>,        // Inclination (radians)
    pub theta0: Vec<f64>,      // Initial true anomaly (radians)
    pub period: Vec<f64>,      // Orbital period (seconds)
    pub pos: Vec<[f64; 3]>,    // Position vector from Earth's core [x, y, z] (km)
    pub vel: Vec<[f64; 3]>,    // Velocity vector [vx, vy, vz] (km/s)
    sim_time: f64,             // Current simulation time (seconds)
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

        // LEO range: 200-2000 km altitude above Earth surface (6371 km radius)
        let radius_dist = Uniform::new(6571.0, 8371.0);
        let incl_dist = Uniform::new(0.0, PI);
        let theta0_dist = Uniform::new(0.0, 2.0 * PI);
        let mu = 398600.4418; // Earth's gravitational parameter (km³/s²)

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
        }

        Self {
            id,
            radius,
            incl,
            theta0,
            period,
            pos,
            vel,
            sim_time: 0.0,
        }
    }

    pub fn propagate(&mut self, dt: f64) {
        self.sim_time += dt;
        let t = self.sim_time;
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
