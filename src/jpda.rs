// Joint Probabilistic Data Association (JPDA) tracker.
/// 
/// Alternative to nearest-neighbor association that computes association
/// probabilities for all observation-track pairs within the gate threshold.

use crate::ground_truth::EvaluationMetrics;
use crate::objects::ObjectPool;
use crate::sensor::ObservationRecord;
use crate::tracker::{Track, TrackStatus};
// No unused imports needed
use std::collections::HashMap;

const BUCKET_SIZE: f64 = 50.0;

/// JPDA track manager - alternative to nearest-neighbor tracker
pub struct JpdaTrackManager {
    pub active: Vec<Track>,
    pub lost: Vec<Track>,
    pub buckets: HashMap<(i32, i32, i32), Vec<usize>>,
    pub confirmed_count: usize,
    pub tentative_count: usize,
    pub next_track_id: u64,
    pub gate_threshold: f64,
    pub dt_ms: u64,
}

impl JpdaTrackManager {
    /// Create a new JPDA track manager
    pub fn new(gate_threshold: f64, dt_ms: u64) -> Self {
        Self {
            active: Vec::new(),
            lost: Vec::new(),
            buckets: HashMap::new(),
            confirmed_count: 0,
            tentative_count: 0,
            next_track_id: 0,
            gate_threshold,
            dt_ms,
        }
    }

    fn bucket_key(pos: &[f64; 3]) -> (i32, i32, i32) {
        (
            (pos[0] / BUCKET_SIZE).floor() as i32,
            (pos[1] / BUCKET_SIZE).floor() as i32,
            (pos[2] / BUCKET_SIZE).floor() as i32,
        )
    }

    fn rebuild_buckets(&mut self) {
        self.buckets.clear();
        for (idx, track) in self.active.iter().enumerate() {
            self.buckets.entry(Self::bucket_key(&track.predicted_pos)).or_default().push(idx);
        }
    }

    fn refresh_counts(&mut self) {
        self.confirmed_count = 0;
        self.tentative_count = 0;
        for track in &self.active {
            match track.status {
                TrackStatus::Confirmed => self.confirmed_count += 1,
                TrackStatus::Tentative => self.tentative_count += 1,
                TrackStatus::Lost => {}
            }
        }
    }

    /// Update tracks with observations using JPDA
    pub fn update(
        &mut self,
        observations: &[ObservationRecord],
        object_indices: &[usize],
        timestamp_ms: u64,
        dt: f64,
        ground_truth: &mut crate::ground_truth::GroundTruthTable,
        objects: &ObjectPool,
    ) -> EvaluationMetrics {
        // Predict all tracks
        for track in &mut self.active {
            track.predict(dt);
        }

        // Rebuild buckets after prediction
        self.rebuild_buckets();

        // Record ground truth
        ground_truth.record_observations(object_indices, objects);

        let mut metrics = EvaluationMetrics::new();

        // For each track, find all observations within gate and compute association probabilities
        for track_idx in 0..self.active.len() {
            let track = &self.active[track_idx];
            let predicted = track.predicted_pos;

            // Find all observations within gate
            let mut candidates: Vec<(usize, f64)> = Vec::new(); // (obs_idx, distance)
            for (obs_idx, obs) in observations.iter().enumerate() {
                let dx = obs.position[0] - predicted[0];
                let dy = obs.position[1] - predicted[1];
                let dz = obs.position[2] - predicted[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist <= self.gate_threshold {
                    candidates.push((obs_idx, dist));
                }
            }

            if candidates.is_empty() {
                // No observations in gate - check if track should be marked as missed
                let steps_since = (timestamp_ms.saturating_sub(track.last_updated_ms)) / self.dt_ms;
                if track.status == TrackStatus::Confirmed && steps_since >= 1 {
                    metrics.false_negatives += 1;
                }
                continue;
            }

            // Compute likelihoods using Gaussian gating function
            let mut likelihoods: Vec<f64> = Vec::new();
            for (_, dist) in &candidates {
                let likelihood = (-0.5 * dist * dist / (self.gate_threshold * self.gate_threshold)).exp();
                likelihoods.push(likelihood);
            }

            // Normalize to get probabilities
            let total_likelihood: f64 = likelihoods.iter().sum();
            if total_likelihood == 0.0 {
                continue;
            }

            let probabilities: Vec<f64> = likelihoods.iter().map(|l| l / total_likelihood).collect();

            // Check if any observation has probability > 0.1
            let max_prob = probabilities.iter().cloned().fold(0.0, f64::max);
            if max_prob <= 0.1 {
                // Missed - no strong association
                if self.active[track_idx].status == TrackStatus::Confirmed {
                    metrics.false_negatives += 1;
                }
                continue;
            }

            // JPDA soft update: weighted combination of all gated observations
            let mut weighted_pos = [0.0f64; 3];
            let mut weighted_vel = [0.0f64; 3];
            for (k, &(obs_idx, _)) in candidates.iter().enumerate() {
                let w = probabilities[k];
                let obs = &observations[obs_idx];
                weighted_pos[0] += w * obs.position[0];
                weighted_pos[1] += w * obs.position[1];
                weighted_pos[2] += w * obs.position[2];
                weighted_vel[0] += w * obs.velocity[0];
                weighted_vel[1] += w * obs.velocity[1];
                weighted_vel[2] += w * obs.velocity[2];
            }

            // Build a synthetic blended observation and update the track
            let best_obs_idx = candidates
                .iter()
                .enumerate()
                .max_by(|(i, _), (j, _)| {
                    probabilities[*i]
                        .partial_cmp(&probabilities[*j])
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(_, &(obs_idx, _))| obs_idx)
                .unwrap();

            let mut blended_obs = observations[best_obs_idx].clone();
            blended_obs.position = weighted_pos;
            blended_obs.velocity = weighted_vel;

            let was_confirmed = self.active[track_idx].status == TrackStatus::Confirmed;
            self.active[track_idx].update(blended_obs, timestamp_ms);

            // Metrics: attribute to highest-probability observation's ground truth
            if self.active[track_idx].status == TrackStatus::Confirmed {
                if let Some(true_id) = ground_truth.get_object_id(best_obs_idx) {
                    if was_confirmed || self.active[track_idx].observations.len() == 3 {
                        if self.active[track_idx].object_id == true_id {
                            metrics.true_positives += 1;
                        } else {
                            metrics.false_positives += 1;
                        }
                    }
                }
            }
        }

        // Create new tracks for unmatched observations
        let mut matched_obs: Vec<bool> = vec![false; observations.len()];
        
        // Mark observations that were used
        for track in &self.active {
            if let Some(last_obs) = track.observations.last() {
                for (obs_idx, obs) in observations.iter().enumerate() {
                    if obs.timestamp_ms == last_obs.timestamp_ms
                        && obs.position == last_obs.position
                    {
                        matched_obs[obs_idx] = true;
                    }
                }
            }
        }

        // For simplicity, use the first observation in gate for each unmatched
        for (obs_idx, obs) in observations.iter().enumerate() {
            if matched_obs[obs_idx] {
                continue;
            }

            // Check if this observation is close to any existing track
            let mut is_new = true;
            for track in &self.active {
                let dx = obs.position[0] - track.predicted_pos[0];
                let dy = obs.position[1] - track.predicted_pos[1];
                let dz = obs.position[2] - track.predicted_pos[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist <= self.gate_threshold {
                    is_new = false;
                    break;
                }
            }

            if is_new {
                if let Some(&obj_idx) = object_indices.get(obs_idx) {
                    let object_id = objects.get_id(obj_idx);
                    let track = Track::new(self.next_track_id, obs.clone(), object_id, timestamp_ms);
                    self.active.push(track);
                    self.next_track_id += 1;
                }
            }
        }

        // Handle lost tracks
        let mut lost_tracks: Vec<Track> = Vec::new();
        let mut retained_active: Vec<Track> = Vec::new();

        for track in self.active.drain(..) {
            if track.status == TrackStatus::Lost {
                lost_tracks.push(track);
            } else {
                retained_active.push(track);
            }
        }
        self.active = retained_active;
        self.lost.extend(lost_tracks);

        self.rebuild_buckets();
        self.refresh_counts();

        metrics
    }

    /// Get references to confirmed tracks
    pub fn confirmed_track_refs(&self) -> Vec<&Track> {
        self.active
            .iter()
            .filter(|t| t.status == TrackStatus::Confirmed)
            .collect()
    }

    /// Get a track by ID
    pub fn get_track(&self, track_id: u64) -> Option<&Track> {
        self.active
            .iter()
            .find(|t| t.track_id == track_id)
            .map(|t| t)
            .or_else(|| self.lost.iter().find(|t| t.track_id == track_id))
    }

    /// Get confirmed count
    pub fn confirmed_count(&self) -> usize {
        self.confirmed_count
    }

    /// Get tentative count
    pub fn tentative_count(&self) -> usize {
        self.tentative_count
    }

    /// Get lost tracks
    pub fn lost_tracks(&self) -> &[Track] {
        &self.lost
    }
}