use crate::catalog::ObjectCatalog;
use crate::collision::{CollisionDetector, CollisionPair};
use crate::config::{SimConfig, TrackerType};
use crate::ground_truth::{EvaluationMetrics, GroundTruthTable};
use crate::hungarian::compute_ospa;
use crate::objects::ObjectPool;
use crate::sensor::{create_sensors, GroundStation};
use crate::spatial::SpatialIndex;
use crate::tracker::TrackManager;
use crate::jpda::JpdaTrackManager;
use rayon::prelude::*;
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Serialize)]
pub struct StepSummary {
    pub step: usize,
    pub sim_time_s: f64,
    pub wall_time_ms: u128,
    pub objects_propagated: usize,
    pub observations_total: usize,
    pub tracks_confirmed: usize,
    pub tracks_tentative: usize,
    pub tracks_lost: usize,
    pub association_tp: usize,
    pub association_fp: usize,
    pub association_fn: usize,
    pub collision_candidates: usize,
    pub cataloged_objects: usize,
    pub ospa: f64,
}

pub struct StepResult {
    pub summary: StepSummary,
    pub metrics: EvaluationMetrics,
    pub collisions: Vec<CollisionPair>,
}

pub struct Simulation {
    pub config: SimConfig,
    pub objects: ObjectPool,
    pub sensors: Vec<GroundStation>,
    pub spatial_index: SpatialIndex,
    pub tracker_nn: TrackManager,
    pub tracker_jpda: Option<JpdaTrackManager>,
    pub ground_truth: GroundTruthTable,
    pub collision_detector: CollisionDetector,
    pub catalog: ObjectCatalog,
    pub step: usize,
    pub finished: bool,
}

impl Simulation {
    pub fn new(config: SimConfig) -> Self {
        let tracker_jpda = if config.tracker_type == TrackerType::Jpda {
            Some(JpdaTrackManager::new(config.gate_threshold, (config.dt * 1000.0) as u64))
        } else {
            None
        };
        
        Self {
            objects: ObjectPool::new(&config),
            sensors: create_sensors(&config),
            spatial_index: SpatialIndex::new(),
            tracker_nn: TrackManager::new(&config),
            tracker_jpda,
            ground_truth: GroundTruthTable::new(),
            collision_detector: CollisionDetector::new(&config),
            catalog: ObjectCatalog::new(),
            step: 0,
            finished: false,
            config,
        }
    }

    pub fn sim_time_ms(&self) -> u64 {
        (self.step as f64 * self.config.dt * 1000.0) as u64
    }

    pub fn step_once(&mut self) -> StepResult {
        let step_start = Instant::now();
        let step = self.step;

        // Propagate all objects in 3D space
        self.objects.propagate(self.config.dt);
        self.spatial_index.rebuild(&self.objects);

        let timestamp_ms = self.sim_time_ms();
        let current_time_s = step as f64 * self.config.dt;

        // All observatories observe objects within their vision cones
        let batches: Vec<_> = self
            .sensors
            .par_iter_mut()
            .map(|sensor| sensor.observe(&self.objects, &self.spatial_index, timestamp_ms))
            .collect();

        let mut all_observations = Vec::new();
        let mut all_object_indices = Vec::new();
        for batch in batches {
            all_observations.extend(batch.observations);
            all_object_indices.extend(batch.object_indices);
        }

        self.ground_truth
            .record_observations(&all_object_indices, &self.objects);
        
        // Use appropriate tracker based on config
        let metrics = if self.config.tracker_type == TrackerType::Jpda {
            self.tracker_jpda.as_mut().unwrap().update(
                &all_observations,
                &all_object_indices,
                timestamp_ms,
                self.config.dt,
                &mut self.ground_truth,
                &self.objects,
            )
        } else {
            self.tracker_nn.update(
                &all_observations,
                &all_object_indices,
                timestamp_ms,
                self.config.dt,
                &self.ground_truth,
                &self.objects,
            )
        };
        self.ground_truth.clear();

        // Get confirmed tracks from appropriate tracker
        let confirmed = if self.config.tracker_type == TrackerType::Jpda {
            self.tracker_jpda.as_ref().unwrap().confirmed_track_refs()
        } else {
            self.tracker_nn.confirmed_track_refs()
        };
        
        let collisions = self
            .collision_detector
            .scan_tracks(&confirmed, timestamp_ms);

        // Update catalog with detected objects only
        self.catalog.update_from_tracks(&confirmed, &self.objects, current_time_s);

        // Compute OSPA metric — compare confirmed track positions against TRUE object positions
        // (not noisy observations, which conflates sensor noise with association error)
        let predicted_positions: Vec<[f64; 3]> = confirmed.iter().map(|t| t.predicted_pos).collect();
        // Deduplicate object indices — same object may be seen by multiple sensors
        let mut seen_ids = std::collections::HashSet::new();
        let ground_truth_positions: Vec<[f64; 3]> = all_object_indices
            .iter()
            .filter(|&&idx| seen_ids.insert(idx))
            .map(|&idx| self.objects.get_position(idx))
            .collect();

        // OSPA optimization: short-circuit if empty or cap at 200 points to avoid O(n³) Hungarian algorithm explosion
        let ospa = if predicted_positions.is_empty() || ground_truth_positions.is_empty() {
            100.0 // cutoff value when no comparison possible
        } else {
            // Cap vectors at 200 points max to limit Hungarian algorithm to 200x200 worst case
            let pred_cap: &[[f64; 3]] = &predicted_positions[..std::cmp::min(predicted_positions.len(), 200)];
            let gt_cap: &[[f64; 3]] = &ground_truth_positions[..std::cmp::min(ground_truth_positions.len(), 200)];
            compute_ospa(pred_cap, gt_cap, 100.0, 2.0)
        };

        let step_duration = step_start.elapsed();
        let summary = StepSummary {
            step,
            sim_time_s: current_time_s,
            wall_time_ms: step_duration.as_millis(),
            objects_propagated: self.config.n_objects,
            observations_total: all_observations.len(),
            tracks_confirmed: if self.config.tracker_type == TrackerType::Jpda {
                self.tracker_jpda.as_ref().unwrap().confirmed_count()
            } else {
                self.tracker_nn.confirmed_count()
            },
            tracks_tentative: if self.config.tracker_type == TrackerType::Jpda {
                self.tracker_jpda.as_ref().unwrap().tentative_count()
            } else {
                self.tracker_nn.tentative_count()
            },
            tracks_lost: if self.config.tracker_type == TrackerType::Jpda {
                self.tracker_jpda.as_ref().unwrap().lost_tracks().len()
            } else {
                self.tracker_nn.lost_tracks().len()
            },
            association_tp: metrics.true_positives,
            association_fp: metrics.false_positives,
            association_fn: metrics.false_negatives,
            collision_candidates: collisions.len(),
            cataloged_objects: self.catalog.len(),
            ospa,
        };

        self.step += 1;
        if self.step >= self.config.steps {
            self.finished = true;
        }

        StepResult {
            summary,
            metrics,
            collisions,
        }
    }
}