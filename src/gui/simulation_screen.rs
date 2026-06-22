use crate::bench::BenchmarkHarness;
use crate::collision::CollisionPair;
use crate::config::{PropagatorType, SimConfig, TrackerType};
use crate::sim::{Simulation, StepSummary};
use std::collections::HashSet;

/// State for the simulation screen - extracted from LeoApp
pub struct SimulationScreenState {
    pub sim: Simulation,
    pub playing: bool,
    pub last_summary: Option<StepSummary>,
    pub collisions: Vec<CollisionPair>,
    pub selected_track: Option<u64>,
    pub inspected_object: Option<usize>,
    pub query_time_s: f64,
    pub speed: f32,
    pub camera_rotation_x: f32,
    pub camera_rotation_y: f32,
    pub camera_distance: f32,
    pub detected_objects_flash: HashSet<usize>,
    pub detecting_sensors: std::collections::HashMap<usize, Vec<u32>>,
    pub tracked_object_ids: HashSet<usize>,
    pub flash_timer: f32,
    pub show_earth: bool,
    pub show_observatories: bool,
    pub show_orbits: bool,
    pub show_metrics: bool,
    pub frame_counter: u32,

    // --- Run metrics accumulators ---
    pub bench: BenchmarkHarness,
    pub cum_tp: usize,
    pub cum_fp: usize,
    pub cum_fn: usize,
    pub cum_ospa_sum: f64,
    pub cum_ospa_count: usize,
    pub cum_steps: usize,

    // --- Time-series history for plots ---
    pub ospa_history: Vec<[f64; 2]>,
    pub f1_history: Vec<[f64; 2]>,

    // --- Live config controls (require sim reset to apply) ---
    pub pending_tracker: TrackerType,
    pub pending_sgp4: bool,
    pub pending_stress_test: bool,
    pub config_dirty: bool,

    // --- Export status message ---
    pub export_status: Option<String>,
}

impl SimulationScreenState {
    pub fn new(config: SimConfig) -> Self {
        let pending_tracker = config.tracker_type;
        let pending_sgp4 = config.propagator == PropagatorType::Sgp4;
        let pending_stress_test = config.stress_test;
        Self {
            sim: Simulation::new(config),
            playing: false,
            last_summary: None,
            collisions: Vec::new(),
            selected_track: None,
            inspected_object: None,
            query_time_s: 0.0,
            speed: 0.5,
            camera_rotation_x: 30.0,
            camera_rotation_y: 45.0,
            camera_distance: 15000.0,
            detected_objects_flash: HashSet::new(),
            detecting_sensors: std::collections::HashMap::new(),
            tracked_object_ids: HashSet::new(),
            flash_timer: 0.0,
            show_earth: true,
            show_observatories: true,
            show_orbits: true,
            show_metrics: true,
            frame_counter: 0,
            bench: BenchmarkHarness::new(),
            cum_tp: 0,
            cum_fp: 0,
            cum_fn: 0,
            cum_ospa_sum: 0.0,
            cum_ospa_count: 0,
            cum_steps: 0,
            ospa_history: Vec::new(),
            f1_history: Vec::new(),
            pending_tracker,
            pending_sgp4,
            pending_stress_test,
            config_dirty: false,
            export_status: None,
        }
    }

    pub fn get_confirmed_tracks(&self) -> Vec<&crate::tracker::Track> {
        if self.sim.config.tracker_type == TrackerType::Jpda {
            self.sim.tracker_jpda.as_ref().unwrap().confirmed_track_refs()
        } else {
            self.sim.tracker_nn.confirmed_track_refs()
        }
    }

    pub fn get_track(&self, track_id: u64) -> Option<&crate::tracker::Track> {
        if self.sim.config.tracker_type == TrackerType::Jpda {
            self.sim.tracker_jpda.as_ref().unwrap().get_track(track_id)
        } else {
            self.sim.tracker_nn.get_track(track_id)
        }
    }

    /// Find the track (if any) associated with a given object index
    pub fn find_track_for_object(&self, object_id: usize) -> Option<u64> {
        self.get_confirmed_tracks()
            .iter()
            .find(|t| t.object_id == object_id)
            .map(|t| t.track_id)
    }

    pub fn advance(&mut self) {
        if self.sim.finished {
            return;
        }

        // Collect currently detected object indices and which sensors saw them
        let mut newly_detected = HashSet::new();
        let mut sensor_detections: std::collections::HashMap<usize, Vec<u32>> = std::collections::HashMap::new();

        let confirmed_tracks = self.get_confirmed_tracks();

        // Precompute the set of object indices currently part of a confirmed track,
        // so the render loop doesn't need to rebuild/scan this Vec per object per frame.
        let mut tracked_ids = HashSet::with_capacity(confirmed_tracks.len());

        for track in &confirmed_tracks {
            newly_detected.insert(track.object_id);
            tracked_ids.insert(track.object_id);

            // Track which sensors detected this object
            let sensor_ids: Vec<u32> = track.observations.iter()
                .filter(|obs| obs.timestamp_ms == (self.sim.step as f64 * self.sim.config.dt * 1000.0) as u64)
                .map(|obs| obs.sensor_id)
                .collect();

            if !sensor_ids.is_empty() {
                sensor_detections.insert(track.object_id, sensor_ids);
            }
        }

        self.detected_objects_flash = newly_detected;
        self.detecting_sensors = sensor_detections;
        self.tracked_object_ids = tracked_ids;
        self.flash_timer = 1.0; // Flash for 1 second

        let result = self.sim.step_once();
        self.query_time_s = result.summary.sim_time_s;

        // Accumulate run metrics
        self.bench.record_step(result.summary.wall_time_ms);
        self.cum_tp += result.metrics.true_positives;
        self.cum_fp += result.metrics.false_positives;
        self.cum_fn += result.metrics.false_negatives;
        self.cum_ospa_sum += result.summary.ospa;
        self.cum_ospa_count += 1;
        self.cum_steps += 1;

        // Time-series history for plots
        self.ospa_history.push([self.query_time_s, result.summary.ospa]);
        self.f1_history.push([self.query_time_s, result.metrics.f1()]);

        self.last_summary = Some(result.summary);
        self.collisions = result.collisions;
    }

    /// Cumulative precision across the whole run so far
    pub fn cum_precision(&self) -> f64 {
        let total = self.cum_tp + self.cum_fp;
        if total == 0 { 0.0 } else { self.cum_tp as f64 / total as f64 }
    }

    /// Cumulative recall across the whole run so far
    pub fn cum_recall(&self) -> f64 {
        let total = self.cum_tp + self.cum_fn;
        if total == 0 { 0.0 } else { self.cum_tp as f64 / total as f64 }
    }

    /// Cumulative F1 across the whole run so far
    pub fn cum_f1(&self) -> f64 {
        let p = self.cum_precision();
        let r = self.cum_recall();
        if p + r == 0.0 { 0.0 } else { 2.0 * p * r / (p + r) }
    }

    /// Running mean OSPA across the whole run so far
    pub fn cum_mean_ospa(&self) -> f64 {
        if self.cum_ospa_count == 0 { 0.0 } else { self.cum_ospa_sum / self.cum_ospa_count as f64 }
    }

    /// Export the current catalog to CSV and a metrics snapshot to JSON.
    pub fn export_data(&mut self) {
        let csv_path = "detected_objects_catalog.csv";
        let json_path = "run_metrics_snapshot.json";

        let csv_result = self.sim.catalog.export_to_csv(csv_path);

        #[derive(serde::Serialize)]
        struct MetricsSnapshot {
            sim_time_s: f64,
            steps: usize,
            cumulative_precision: f64,
            cumulative_recall: f64,
            cumulative_f1: f64,
            mean_ospa: f64,
            last_step_ospa: Option<f64>,
            tracks_confirmed: Option<usize>,
            tracks_tentative: Option<usize>,
            tracks_lost: Option<usize>,
            cataloged_objects: usize,
            total_objects: usize,
            benchmark: crate::bench::BenchmarkReport,
        }

        let snapshot = MetricsSnapshot {
            sim_time_s: self.query_time_s,
            steps: self.cum_steps,
            cumulative_precision: self.cum_precision(),
            cumulative_recall: self.cum_recall(),
            cumulative_f1: self.cum_f1(),
            mean_ospa: self.cum_mean_ospa(),
            last_step_ospa: self.last_summary.as_ref().map(|s| s.ospa),
            tracks_confirmed: self.last_summary.as_ref().map(|s| s.tracks_confirmed),
            tracks_tentative: self.last_summary.as_ref().map(|s| s.tracks_tentative),
            tracks_lost: self.last_summary.as_ref().map(|s| s.tracks_lost),
            cataloged_objects: self.sim.catalog.len(),
            total_objects: self.sim.config.n_objects,
            benchmark: self.bench.report(),
        };

        let json_result = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .and_then(|json| std::fs::write(json_path, json));

        self.export_status = Some(match (csv_result, json_result) {
            (Ok(_), Ok(_)) => format!("Exported {} and {}", csv_path, json_path),
            (Err(e), _) => format!("Catalog export failed: {}", e),
            (_, Err(e)) => format!("Metrics export failed: {}", e),
        });
    }
}