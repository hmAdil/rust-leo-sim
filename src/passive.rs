use crate::sensor::ObservationRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEstimate {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub anchor_timestamp_ms: u64,
    pub position_uncertainty_km: f64,
}

/// Linear propagation from a single snapshot: p(T) = p₀ + v₀ × (T − t₀).
pub fn propagate_snapshot(
    position: [f64; 3],
    velocity: [f64; 3],
    snapshot_ms: u64,
    query_ms: u64,
    pos_noise_km: f64,
    vel_noise_km_s: f64,
) -> StateEstimate {
    let dt_s = ms_to_s(query_ms, snapshot_ms);
    let uncertainty = pos_noise_km + vel_noise_km_s * dt_s.abs();
    StateEstimate {
        position: [
            position[0] + velocity[0] * dt_s,
            position[1] + velocity[1] * dt_s,
            position[2] + velocity[2] * dt_s,
        ],
        velocity,
        anchor_timestamp_ms: snapshot_ms,
        position_uncertainty_km: uncertainty,
    }
}

/// Estimate state at `query_ms` using the observation closest in time.
pub fn estimate_from_observations(
    observations: &[ObservationRecord],
    query_ms: u64,
    pos_noise_km: f64,
    vel_noise_km_s: f64,
) -> Option<StateEstimate> {
    let anchor = observations
        .iter()
        .min_by_key(|o| o.timestamp_ms.abs_diff(query_ms))?;
    Some(propagate_snapshot(
        anchor.position,
        anchor.velocity,
        anchor.timestamp_ms,
        query_ms,
        pos_noise_km,
        vel_noise_km_s,
    ))
}

pub fn observation_history(observations: &[ObservationRecord]) -> &[ObservationRecord] {
    observations
}

fn ms_to_s(query_ms: u64, anchor_ms: u64) -> f64 {
    (query_ms as f64 - anchor_ms as f64) / 1000.0
}
