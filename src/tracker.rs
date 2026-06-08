use crate::config::SimConfig;
use crate::ground_truth::{EvaluationMetrics, GroundTruthTable};
use crate::objects::ObjectPool;
use crate::passive::{estimate_from_observations, observation_history};
use crate::sensor::ObservationRecord;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const BUCKET_SIZE: f64 = 50.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrackStatus {
    Tentative,
    Confirmed,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub track_id: u64,
    pub observations: Vec<ObservationRecord>,
    pub predicted_pos: [f64; 3],
    pub predicted_vel: [f64; 3],
    pub last_updated_ms: u64,
    pub confidence: f32,
    pub status: TrackStatus,
    pub object_id: usize,  // References object by index for OBJ_XXXXXX naming
}

impl Track {
    pub fn new(
        track_id: u64,
        observation: ObservationRecord,
        object_id: usize,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            track_id,
            observations: vec![observation.clone()],
            predicted_pos: observation.position,
            predicted_vel: observation.velocity,
            last_updated_ms: timestamp_ms,
            confidence: 0.5,
            status: TrackStatus::Tentative,
            object_id,
        }
    }

    pub fn predict(&mut self, dt: f64) {
        if let Some(last_obs) = self.observations.last() {
            self.predicted_pos[0] = last_obs.position[0] + last_obs.velocity[0] * dt;
            self.predicted_pos[1] = last_obs.position[1] + last_obs.velocity[1] * dt;
            self.predicted_pos[2] = last_obs.position[2] + last_obs.velocity[2] * dt;
            self.predicted_vel = last_obs.velocity;
        }
    }

    pub fn update(&mut self, observation: ObservationRecord, timestamp_ms: u64) {
        self.observations.push(observation);
        self.last_updated_ms = timestamp_ms;
        if self.observations.len() >= 3 {
            self.status = TrackStatus::Confirmed;
            self.confidence = 0.9;
        } else {
            self.confidence = 0.3 + (self.observations.len() as f32) * 0.2;
        }
    }

    pub fn history(&self) -> &[ObservationRecord] {
        observation_history(&self.observations)
    }

    pub fn estimate_at(
        &self,
        query_ms: u64,
        pos_noise_km: f64,
        vel_noise_km_s: f64,
    ) -> Option<crate::passive::StateEstimate> {
        estimate_from_observations(
            &self.observations,
            query_ms,
            pos_noise_km,
            vel_noise_km_s,
        )
    }
}

struct ActiveTrack {
    track: Track,
    bucket: (i32, i32, i32),
}

pub struct TrackManager {
    active: Vec<ActiveTrack>,
    lost: Vec<Track>,
    buckets: HashMap<(i32, i32, i32), Vec<usize>>,
    confirmed_count: usize,
    tentative_count: usize,
    next_track_id: u64,
    gate_threshold: f64,
    dt_ms: u64,
}

impl TrackManager {
    pub fn new(config: &SimConfig) -> Self {
        Self {
            active: Vec::new(),
            lost: Vec::new(),
            buckets: HashMap::new(),
            confirmed_count: 0,
            tentative_count: 0,
            next_track_id: 0,
            gate_threshold: config.gate_threshold,
            dt_ms: (config.dt * 1000.0) as u64,
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
        for (idx, entry) in self.active.iter().enumerate() {
            self.buckets.entry(entry.bucket).or_default().push(idx);
        }
    }

    fn refresh_counts(&mut self) {
        self.confirmed_count = 0;
        self.tentative_count = 0;
        for entry in &self.active {
            match entry.track.status {
                TrackStatus::Confirmed => self.confirmed_count += 1,
                TrackStatus::Tentative => self.tentative_count += 1,
                TrackStatus::Lost => {}
            }
        }
    }

    pub fn update(
        &mut self,
        observations: &[ObservationRecord],
        object_indices: &[usize],
        timestamp_ms: u64,
        dt: f64,
        ground_truth: &GroundTruthTable,
        objects: &ObjectPool,
    ) -> EvaluationMetrics {
        // Parallel predict and bucket update
        let dt_ms = self.dt_ms;
        self.active.par_iter_mut().for_each(|entry| {
            entry.track.predict(dt);
            entry.bucket = Self::bucket_key(&entry.track.predicted_pos);
            if entry.track.status == TrackStatus::Confirmed {
                let steps_since =
                    (timestamp_ms.saturating_sub(entry.track.last_updated_ms)) / dt_ms;
                if steps_since >= 3 {
                    entry.track.status = TrackStatus::Lost;
                }
            }
        });

        // Collect lost tracks
        let mut lost_tracks: Vec<Track> = Vec::new();
        let mut retained_active: Vec<ActiveTrack> = Vec::new();
        
        for entry in self.active.drain(..) {
            if entry.track.status == TrackStatus::Lost {
                lost_tracks.push(entry.track);
            } else {
                retained_active.push(entry);
            }
        }
        self.active = retained_active;
        self.lost.extend(lost_tracks);

        self.rebuild_buckets();

        let mut matched_obs = vec![false; observations.len()];
        let mut matched_tracks = vec![false; self.active.len()];
        let mut metrics = EvaluationMetrics::new();

        // Process observations - use sequential for correctness with shared state
        for (obs_idx, obs) in observations.iter().enumerate() {
            if matched_obs[obs_idx] {
                continue;
            }

            let obs_bucket = Self::bucket_key(&obs.position);
            let mut best_track_idx: Option<usize> = None;
            let mut best_distance = self.gate_threshold;

            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let bucket = (obs_bucket.0 + dx, obs_bucket.1 + dy, obs_bucket.2 + dz);
                        let Some(track_indices) = self.buckets.get(&bucket) else {
                            continue;
                        };
                        for &track_idx in track_indices {
                            if matched_tracks[track_idx] {
                                continue;
                            }
                            let predicted = self.active[track_idx].track.predicted_pos;
                            let dxp = obs.position[0] - predicted[0];
                            let dyp = obs.position[1] - predicted[1];
                            let dzp = obs.position[2] - predicted[2];
                            let distance = (dxp * dxp + dyp * dyp + dzp * dzp).sqrt();
                            if distance < best_distance {
                                best_distance = distance;
                                best_track_idx = Some(track_idx);
                            }
                        }
                    }
                }
            }

            if let Some(track_idx) = best_track_idx {
                matched_tracks[track_idx] = true;
                matched_obs[obs_idx] = true;
                let was_confirmed =
                    self.active[track_idx].track.status == TrackStatus::Confirmed;
                self.active[track_idx].track.update(obs.clone(), timestamp_ms);
                let new_bucket =
                    Self::bucket_key(&self.active[track_idx].track.predicted_pos);
                self.active[track_idx].bucket = new_bucket;

                let track = &self.active[track_idx].track;
                if track.status == TrackStatus::Confirmed {
                    if let (Some(true_id), Some(&_obj_idx)) = (
                        ground_truth.get_object_id(obs_idx),
                        object_indices.get(obs_idx),
                    ) {
                        if was_confirmed || track.observations.len() == 3 {
                            if track.object_id == true_id {
                                metrics.true_positives += 1;
                            } else {
                                metrics.false_positives += 1;
                            }
                        }
                    }
                }
            } else if let Some(&obj_idx) = object_indices.get(obs_idx) {
                let object_id = objects.get_id(obj_idx);
                let track = Track::new(self.next_track_id, obs.clone(), object_id, timestamp_ms);
                let bucket = Self::bucket_key(&track.predicted_pos);
                self.active.push(ActiveTrack { track, bucket });
                self.next_track_id += 1;
                matched_obs[obs_idx] = true;
            }
        }

        // Count false negatives for confirmed tracks
        for entry in &self.active {
            if entry.track.status == TrackStatus::Confirmed {
                let steps_since =
                    (timestamp_ms.saturating_sub(entry.track.last_updated_ms)) / self.dt_ms;
                if steps_since >= 1 {
                    metrics.false_negatives += 1;
                }
            }
        }

        self.rebuild_buckets();
        self.refresh_counts();
        metrics
    }

    pub fn confirmed_track_refs(&self) -> Vec<&Track> {
        self.active
            .iter()
            .filter(|e| e.track.status == TrackStatus::Confirmed)
            .map(|e| &e.track)
            .collect()
    }

    #[allow(dead_code)]
    pub fn tentative_track_refs(&self) -> Vec<&Track> {
        self.active
            .iter()
            .filter(|e| e.track.status == TrackStatus::Tentative)
            .map(|e| &e.track)
            .collect()
    }

    pub fn get_track(&self, track_id: u64) -> Option<&Track> {
        self.active
            .iter()
            .find(|e| e.track.track_id == track_id)
            .map(|e| &e.track)
            .or_else(|| self.lost.iter().find(|t| t.track_id == track_id))
    }

    pub fn confirmed_count(&self) -> usize {
        self.confirmed_count
    }

    pub fn tentative_count(&self) -> usize {
        self.tentative_count
    }

    pub fn lost_tracks(&self) -> &[Track] {
        &self.lost
    }

    #[allow(dead_code)]
    pub fn lost_track_refs(&self) -> Vec<&Track> {
        self.lost.iter().collect()
    }
}
