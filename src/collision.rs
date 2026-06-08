use crate::config::SimConfig;
use crate::passive::{estimate_from_observations, StateEstimate};
use crate::tracker::{Track, TrackStatus};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const BUCKET_KM: f64 = 200.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionPair {
    pub track_a: u64,
    pub track_b: u64,
    pub time_to_closest_approach_s: f64,
    pub miss_distance_km: f64,
    pub closest_approach_ms: u64,
    pub combined_uncertainty_km: f64,
}

pub struct CollisionDetector {
    threshold_km: f64,
    horizon_s: f64,
    pos_noise_km: f64,
    vel_noise_km_s: f64,
}

impl CollisionDetector {
    pub fn new(config: &SimConfig) -> Self {
        Self {
            threshold_km: config.collision_threshold_km,
            horizon_s: config.collision_horizon_s,
            pos_noise_km: config.pos_noise_std,
            vel_noise_km_s: config.vel_noise_std,
        }
    }

    pub fn scan_tracks(&self, tracks: &[&Track], query_ms: u64) -> Vec<CollisionPair> {
        let snapshots: Vec<(u64, StateEstimate)> = tracks
            .par_iter()
            .filter(|t| t.status == TrackStatus::Confirmed)
            .filter_map(|t| {
                estimate_from_observations(
                    &t.observations,
                    query_ms,
                    self.pos_noise_km,
                    self.vel_noise_km_s,
                )
                .map(|est| (t.track_id, est))
            })
            .collect();

        if snapshots.len() < 2 {
            return Vec::new();
        }

        let mut buckets: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (idx, (_, est)) in snapshots.iter().enumerate() {
            let key = (
                (est.position[0] / BUCKET_KM).floor() as i32,
                (est.position[1] / BUCKET_KM).floor() as i32,
                (est.position[2] / BUCKET_KM).floor() as i32,
            );
            buckets.entry(key).or_default().push(idx);
        }

        let mut pairs: Vec<CollisionPair> = Vec::new();
        // Note: seen is now handled per-bucket in parallel processing
        let _ = (); // placeholder to avoid unused variable warning

        // Process buckets in parallel
        let bucket_pairs: Vec<_> = buckets
            .par_iter()
            .flat_map(|(cell, indices)| {
                let mut local_pairs = Vec::new();
                let mut local_seen = HashMap::new();
                
                // Check pairs within the same bucket
                for a in 0..indices.len() {
                    for b in (a + 1)..indices.len() {
                        if let Some(pair) = try_pair(
                            &snapshots,
                            indices[a],
                            indices[b],
                            query_ms,
                            self.threshold_km,
                            self.horizon_s,
                            &mut local_seen,
                        ) {
                            local_pairs.push(pair);
                        }
                    }
                }
                
                // Check pairs with neighboring buckets
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        for dz in -1..=1 {
                            if dx == 0 && dy == 0 && dz == 0 {
                                continue;
                            }
                            let neighbor = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                            let Some(neighbor_idxs) = buckets.get(&neighbor) else {
                                continue;
                            };
                            for &i in indices {
                                for &j in neighbor_idxs {
                                    if let Some(pair) = try_pair(
                                        &snapshots,
                                        i,
                                        j,
                                        query_ms,
                                        self.threshold_km,
                                        self.horizon_s,
                                        &mut local_seen,
                                    ) {
                                        local_pairs.push(pair);
                                    }
                                }
                            }
                        }
                    }
                }
                
                local_pairs
            })
            .collect();

        pairs.extend(bucket_pairs);

        pairs.sort_by(|a, b| {
            a.miss_distance_km
                .partial_cmp(&b.miss_distance_km)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        pairs.truncate(500);
        pairs
    }
}

fn pair_key(a: u64, b: u64) -> (u64, u64) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn try_pair(
    snapshots: &[(u64, StateEstimate)],
    i: usize,
    j: usize,
    query_ms: u64,
    threshold_km: f64,
    horizon_s: f64,
    seen: &mut HashMap<(u64, u64), ()>,
) -> Option<CollisionPair> {
    if i == j {
        return None;
    }
    let (id_a, est_a) = &snapshots[i];
    let (id_b, est_b) = &snapshots[j];
    let key = pair_key(*id_a, *id_b);
    if seen.contains_key(&key) {
        return None;
    }
    seen.insert(key, ());
    closest_approach_pair(*id_a, est_a, *id_b, est_b, query_ms, threshold_km, horizon_s)
}

fn closest_approach_pair(
    track_a: u64,
    a: &StateEstimate,
    track_b: u64,
    b: &StateEstimate,
    query_ms: u64,
    threshold_km: f64,
    horizon_s: f64,
) -> Option<CollisionPair> {
    let p_rel = [
        a.position[0] - b.position[0],
        a.position[1] - b.position[1],
        a.position[2] - b.position[2],
    ];
    let v_rel = [
        a.velocity[0] - b.velocity[0],
        a.velocity[1] - b.velocity[1],
        a.velocity[2] - b.velocity[2],
    ];

    let v_rel_sq = v_rel[0] * v_rel[0] + v_rel[1] * v_rel[1] + v_rel[2] * v_rel[2];
    let (t_ca, miss) = if v_rel_sq < 1e-12 {
        let d = (p_rel[0] * p_rel[0] + p_rel[1] * p_rel[1] + p_rel[2] * p_rel[2]).sqrt();
        (0.0, d)
    } else {
        let t = -(p_rel[0] * v_rel[0] + p_rel[1] * v_rel[1] + p_rel[2] * v_rel[2]) / v_rel_sq;
        if t < 0.0 || t > horizon_s {
            return None;
        }
        let cx = p_rel[0] + v_rel[0] * t;
        let cy = p_rel[1] + v_rel[1] * t;
        let cz = p_rel[2] + v_rel[2] * t;
        (t, (cx * cx + cy * cy + cz * cz).sqrt())
    };

    if miss > threshold_km {
        return None;
    }

    let closing = p_rel[0] * v_rel[0] + p_rel[1] * v_rel[1] + p_rel[2] * v_rel[2];
    if closing >= 0.0 && v_rel_sq >= 1e-12 {
        return None;
    }

    Some(CollisionPair {
        track_a,
        track_b,
        time_to_closest_approach_s: t_ca,
        miss_distance_km: miss,
        closest_approach_ms: query_ms + (t_ca * 1000.0) as u64,
        combined_uncertainty_km: a.position_uncertainty_km + b.position_uncertainty_km,
    })
}