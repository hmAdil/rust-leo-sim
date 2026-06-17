use crate::bench::BenchmarkHarness;
use crate::collision::CollisionPair;
use crate::config::{PropagatorType, SimConfig, TrackerType};
use crate::sim::{Simulation, StepSummary};
use eframe::egui;
use egui::Color32;
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::HashSet;

pub fn run(config: SimConfig) {
    let mut gui_config = SimConfig::for_gui();
    gui_config.seed = config.seed;
    gui_config.n_sensors = config.n_sensors;
    gui_config.collision_threshold_km = config.collision_threshold_km;
    gui_config.collision_horizon_s = config.collision_horizon_s;
    gui_config.tracker_type = config.tracker_type;
    if config.n_objects != SimConfig::default().n_objects {
        gui_config.n_objects = config.n_objects;
    }
    let config = gui_config;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_title("LEO Observatory Network - 3D Space Object Tracking"),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "leo_sim_gui",
        options,
        Box::new(|_| Ok(Box::new(LeoApp::new(config)))),
    );
}

struct LeoApp {
    sim: Simulation,
    playing: bool,
    last_summary: Option<StepSummary>,
    collisions: Vec<CollisionPair>,
    selected_track: Option<u64>,
    inspected_object: Option<usize>,
    query_time_s: f64,
    speed: f32,
    camera_rotation_x: f32,
    camera_rotation_y: f32,
    camera_distance: f32,
    detected_objects_flash: HashSet<usize>,
    detecting_sensors: std::collections::HashMap<usize, Vec<u32>>,
    tracked_object_ids: HashSet<usize>,
    flash_timer: f32,
    show_earth: bool,
    show_observatories: bool,
    show_orbits: bool,
    show_metrics: bool,
    frame_counter: u32,

    // --- Run metrics accumulators ---
    bench: BenchmarkHarness,
    cum_tp: usize,
    cum_fp: usize,
    cum_fn: usize,
    cum_ospa_sum: f64,
    cum_ospa_count: usize,
    cum_steps: usize,

    // --- Time-series history for plots ---
    ospa_history: Vec<[f64; 2]>,
    f1_history: Vec<[f64; 2]>,

    // --- Live config controls (require sim reset to apply) ---
    pending_tracker: TrackerType,
    pending_sgp4: bool,
    pending_stress_test: bool,
    config_dirty: bool,

    // --- Export status message ---
    export_status: Option<String>,
}

impl LeoApp {
    fn new(config: SimConfig) -> Self {
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

    /// Create a new LeoApp in real-time mode (automatically playing at 1x speed)
    fn new_realtime(config: SimConfig) -> Self {
        let mut app = Self::new(config);
        app.playing = true;  // Start playing automatically
        app.speed = 1.0;     // Real-time speed (1x)
        app
    }

    fn get_confirmed_tracks(&self) -> Vec<&crate::tracker::Track> {
        if self.sim.config.tracker_type == TrackerType::Jpda {
            self.sim.tracker_jpda.as_ref().unwrap().confirmed_track_refs()
        } else {
            self.sim.tracker_nn.confirmed_track_refs()
        }
    }

    fn get_track(&self, track_id: u64) -> Option<&crate::tracker::Track> {
        if self.sim.config.tracker_type == TrackerType::Jpda {
            self.sim.tracker_jpda.as_ref().unwrap().get_track(track_id)
        } else {
            self.sim.tracker_nn.get_track(track_id)
        }
    }

    /// Find the track (if any) associated with a given object index
    fn find_track_for_object(&self, object_id: usize) -> Option<u64> {
        self.get_confirmed_tracks()
            .iter()
            .find(|t| t.object_id == object_id)
            .map(|t| t.track_id)
    }

    fn advance(&mut self) {
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
    fn cum_precision(&self) -> f64 {
        let total = self.cum_tp + self.cum_fp;
        if total == 0 { 0.0 } else { self.cum_tp as f64 / total as f64 }
    }

    /// Cumulative recall across the whole run so far
    fn cum_recall(&self) -> f64 {
        let total = self.cum_tp + self.cum_fn;
        if total == 0 { 0.0 } else { self.cum_tp as f64 / total as f64 }
    }

    /// Cumulative F1 across the whole run so far
    fn cum_f1(&self) -> f64 {
        let p = self.cum_precision();
        let r = self.cum_recall();
        if p + r == 0.0 { 0.0 } else { 2.0 * p * r / (p + r) }
    }

    /// Running mean OSPA across the whole run so far
    fn cum_mean_ospa(&self) -> f64 {
        if self.cum_ospa_count == 0 { 0.0 } else { self.cum_ospa_sum / self.cum_ospa_count as f64 }
    }

    /// Apply pending config changes by rebuilding the simulation from scratch.
    fn apply_config_and_reset(&mut self) {
        let mut config = self.sim.config.clone();
        config.tracker_type = self.pending_tracker;
        config.propagator = if self.pending_sgp4 {
            PropagatorType::Sgp4
        } else {
            PropagatorType::SimpleKeplerian
        };
        config.stress_test = self.pending_stress_test;
        *self = Self::new(config);
    }

    /// Export the current catalog to CSV and a metrics snapshot to JSON.
    fn export_data(&mut self) {
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

impl eframe::App for LeoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update flash timer
        if self.flash_timer > 0.0 {
            self.flash_timer -= ctx.input(|i| i.stable_dt);
            if self.flash_timer < 0.0 {
                self.flash_timer = 0.0;
                self.detected_objects_flash.clear();
            }
        }

        if self.playing && !self.sim.finished {
            // Handle fractional speeds
            self.frame_counter += 1;
            let frames_per_step = (1.0 / self.speed).max(1.0) as u32;

            if self.frame_counter >= frames_per_step {
                self.frame_counter = 0;
                let steps_this_frame = self.speed.max(1.0) as usize;

                for _ in 0..steps_this_frame {
                    if self.sim.finished {
                        break;
                    }
                    self.advance();
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.playing { "Pause" } else { "Play" }).clicked() {
                    self.playing = !self.playing;
                }
                if ui.button("Step").clicked() {
                    self.advance();
                }
                if ui.button("Reset").clicked() {
                    let config = self.sim.config.clone();
                    *self = Self::new(config);
                }
                ui.separator();

                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.speed, 0.1..=5.0)
                    .text("x")
                    .logarithmic(true));
                ui.label(format!("{:.1}x", self.speed));

                ui.separator();
                ui.label(format!("Sim: {:.0}s", self.query_time_s));
                if let Some(s) = &self.last_summary {
                    ui.label(format!(
                        "Objects: {} | Detected: {} ({:.1}%) | Collisions: {}",
                        s.objects_propagated,
                        s.cataloged_objects,
                        (s.cataloged_objects as f64 / s.objects_propagated as f64) * 100.0,
                        s.collision_candidates
                    ));
                }

                ui.separator();
                ui.checkbox(&mut self.show_metrics, "Run Metrics");

                ui.separator();
                if ui.button("Export CSV + JSON").clicked() {
                    self.export_data();
                }
                if let Some(status) = &self.export_status {
                    ui.label(status);
                }
            });
        });

        egui::SidePanel::left("detections")
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.heading("Live Detection Log");
                ui.separator();

                if !self.detected_objects_flash.is_empty() && self.flash_timer > 0.0 {
                    ui.colored_label(Color32::from_rgb(100, 255, 100), "OBJECTS DETECTED");
                    ui.separator();

                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        let mut detected_list: Vec<_> = self.detected_objects_flash.iter().copied().collect();
                        detected_list.sort();

                        for obj_idx in detected_list.iter().take(50) {
                            let obj_name = self.sim.objects.get_name(*obj_idx);
                            let pos = self.sim.objects.get_position(*obj_idx);

                            // Check if this object is in a collision warning
                            let has_collision = self.collisions.iter().any(|pair| {
                                if let Some(track_a) = self.get_confirmed_tracks()
                                    .iter().find(|t| t.track_id == pair.track_a) {
                                    if track_a.object_id == *obj_idx {
                                        return true;
                                    }
                                }
                                if let Some(track_b) = self.get_confirmed_tracks()
                                    .iter().find(|t| t.track_id == pair.track_b) {
                                    if track_b.object_id == *obj_idx {
                                        return true;
                                    }
                                }
                                false
                            });

                            let collision_marker = if has_collision { " COLLISION" } else { "" };
                            let color = if has_collision {
                                Color32::from_rgb(255, 100, 100)
                            } else {
                                Color32::from_rgb(150, 255, 150)
                            };

                            let obj_type = self.sim.objects.get_object_type(*obj_idx);
                            let obj_size = self.sim.objects.get_size_meters(*obj_idx);
                            let (type_icon, type_label) = match obj_type {
                                crate::objects::ObjectType::Satellite => ("ð°", "SAT"),
                                crate::objects::ObjectType::Debris => ("ð", "DBR"),
                            };
                            ui.colored_label(
                                color,
                                format!("{} {} {} | {:.2}m | T={:.0}s{}",
                                    type_icon,
                                    obj_name,
                                    type_label,
                                    obj_size,
                                    self.query_time_s,
                                    collision_marker
                                )
                            );

                            // Show which observatories detected this object
                            if let Some(sensor_ids) = self.detecting_sensors.get(obj_idx) {
                                let obs_names: Vec<String> = sensor_ids.iter()
                                    .map(|id| format!("OBS_{:02}", id))
                                    .collect();
                                ui.label(format!("   Detected by: {}", obs_names.join(", ")));
                            }

                            ui.label(format!("   Pos: [{:.0}, {:.0}, {:.0}] km", pos[0], pos[1], pos[2]));
                            ui.separator();
                        }
                        if detected_list.len() > 50 {
                            ui.label(format!("... and {} more", detected_list.len() - 50));
                        }
                    });
                } else {
                    ui.colored_label(Color32::GRAY, "Scanning for objects...");
                }

                ui.separator();
                ui.heading("Observatory Network");
                ui.label(format!("Active Stations: {}", self.sim.sensors.len()));
                ui.label("Coverage: Fibonacci sphere distribution".to_string());
                ui.label(format!("FOV per station: {:.0} deg cone", self.sim.config.fov_half_angle.to_degrees()));

                ui.separator();
                ui.heading("Collision Warnings");
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    if self.collisions.is_empty() {
                        ui.label("No collision risks detected.");
                    } else {
                        ui.colored_label(
                            Color32::from_rgb(255, 150, 50),
                            format!("{} ACTIVE WARNINGS", self.collisions.len())
                        );
                        ui.separator();
                    }
                    for pair in &self.collisions {
                        let label = format!(
                            "Track #{} <-> #{}\nMiss: {:.2} km\nTime to CA: {:.0}s",
                            pair.track_a,
                            pair.track_b,
                            pair.miss_distance_km,
                            pair.time_to_closest_approach_s
                        );
                        if ui.selectable_label(
                            self.selected_track == Some(pair.track_a),
                            label,
                        ).clicked() {
                            self.selected_track = Some(pair.track_a);
                            self.inspected_object = None;
                        }
                    }
                });

                ui.separator();
                ui.heading("Run Configuration");
                ui.label("Changes below require a simulation reset.");

                ui.horizontal(|ui| {
                    ui.label("Tracker:");
                    egui::ComboBox::from_id_salt("tracker_select")
                        .selected_text(match self.pending_tracker {
                            TrackerType::NearestNeighbor => "Nearest Neighbor",
                            TrackerType::Jpda => "JPDA",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.pending_tracker, TrackerType::NearestNeighbor, "Nearest Neighbor");
                            ui.selectable_value(&mut self.pending_tracker, TrackerType::Jpda, "JPDA");
                        });
                });

                if ui.checkbox(&mut self.pending_sgp4, "Use SGP4 propagator").changed() {}
                if ui.checkbox(&mut self.pending_stress_test, "Stress test (clustered objects)").changed() {}

                let dirty = self.pending_tracker != self.sim.config.tracker_type
                    || self.pending_sgp4 != (self.sim.config.propagator == PropagatorType::Sgp4)
                    || self.pending_stress_test != self.sim.config.stress_test;
                self.config_dirty = dirty;

                if dirty {
                    ui.colored_label(Color32::from_rgb(255, 200, 100), "Pending changes not applied.");
                }
                if ui.add_enabled(dirty, egui::Button::new("Apply & Reset")).clicked() {
                    self.apply_config_and_reset();
                }
            });

        egui::SidePanel::right("track_inspector")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.heading("Object Inspector");
                ui.separator();
                if let Some(track_id) = self.selected_track {
                    if let Some(track) = self.get_track(track_id) {
                        let object_name = self.sim.objects.get_name(track.object_id);
                        ui.label(format!("Object: {}", object_name));
                        ui.label(format!("Track ID: {}", track.track_id));
                        ui.label(format!("Status: {:?}", track.status));
                        ui.label(format!("Observations: {}", track.history().len()));
                        ui.label(format!("Confidence: {:.2}", track.confidence));

                        // Object type and physical properties
                        let obj_type = self.sim.objects.get_object_type(track.object_id);
                        let obj_size = self.sim.objects.get_size_meters(track.object_id);
                        let obj_rcs = self.sim.objects.get_rcs(track.object_id);
                        let (type_icon, type_str) = match obj_type {
                            crate::objects::ObjectType::Satellite => ("🛰", "Satellite"),
                            crate::objects::ObjectType::Debris => ("🗑", "Debris"),
                        };
                        ui.label(format!("Type: {} {}", type_icon, type_str));
                        ui.label(format!("Size: {:.3} m", obj_size));
                        ui.label(format!("RCS: {:.4} m²", obj_rcs));

                        let query_ms = (self.query_time_s * 1000.0) as u64;
                        if let Some(est) = track.estimate_at(
                            query_ms,
                            self.sim.config.pos_noise_std,
                            self.sim.config.vel_noise_std,
                        ) {
                            ui.separator();
                            ui.label("State Vector (Earth core origin):");
                            ui.monospace(format!(
                                "r = [{:.1}, {:.1}, {:.1}] km",
                                est.position[0], est.position[1], est.position[2]
                            ));
                            ui.monospace(format!(
                                "v = [{:.4}, {:.4}, {:.4}] km/s",
                                est.velocity[0], est.velocity[1], est.velocity[2]
                            ));
                            ui.label(format!("Uncertainty: +/- {:.2} km", est.position_uncertainty_km));
                        }

                        ui.separator();
                        ui.label("Detection History:");
                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            for (i, obs) in track.history().iter().enumerate() {
                                ui.label(format!(
                                    "#{} t={}s OBS_{:02} r=[{:.0},{:.0},{:.0}]",
                                    i,
                                    obs.timestamp_ms / 1000,
                                    obs.sensor_id,
                                    obs.position[0],
                                    obs.position[1],
                                    obs.position[2]
                                ));
                            }
                        });
                    } else {
                        ui.label("Track not found.");
                    }
                } else if let Some(obj_idx) = self.inspected_object {
                    // Object clicked directly in the 3D view but has no confirmed track
                    let obj_name = self.sim.objects.get_name(obj_idx);
                    let pos = self.sim.objects.get_position(obj_idx);
                    let vel = self.sim.objects.get_velocity(obj_idx);
                    let obj_type2 = self.sim.objects.get_object_type(obj_idx);
                    let obj_size2 = self.sim.objects.get_size_meters(obj_idx);
                    let obj_rcs2 = self.sim.objects.get_rcs(obj_idx);
                    let (type_icon2, type_str2) = match obj_type2 {
                        crate::objects::ObjectType::Satellite => ("🛰", "Satellite"),
                        crate::objects::ObjectType::Debris => ("🗑", "Debris"),
                    };
                    ui.label(format!("Object: {}", obj_name));
                    ui.label("No confirmed track for this object yet.");
                    ui.separator();
                    ui.label(format!("Type: {} {}", type_icon2, type_str2));
                    ui.label(format!("Size: {:.3} m", obj_size2));
                    ui.label(format!("RCS: {:.4} m²", obj_rcs2));
                    ui.separator();
                    ui.label("True State (Earth core origin):");
                    ui.monospace(format!("r = [{:.1}, {:.1}, {:.1}] km", pos[0], pos[1], pos[2]));
                    ui.monospace(format!("v = [{:.4}, {:.4}, {:.4}] km/s", vel[0], vel[1], vel[2]));
                } else {
                    ui.label("Click an object in the 3D view, or select a collision pair, to inspect.");
                    ui.separator();
                    ui.label("Catalog Statistics:");
                    ui.label(format!("Detected Objects: {}", self.sim.catalog.len()));
                    ui.label(format!("Total Simulated: {}", self.sim.config.n_objects));
                    let detection_rate = (self.sim.catalog.len() as f64 / self.sim.config.n_objects as f64) * 100.0;
                    ui.label(format!("Detection Rate: {:.2}%", detection_rate));
                }
            });

        // --- Run Metrics side panel ---
        if self.show_metrics {
            egui::SidePanel::right("run_metrics")
                .resizable(true)
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.heading("Run Metrics");
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // --- Tracking Health ---
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Tracking Health").strong());
                            if let Some(s) = &self.last_summary {
                                ui.label(format!("Confirmed: {}", s.tracks_confirmed));
                                ui.label(format!("Tentative: {}", s.tracks_tentative));
                                ui.label(format!("Lost: {}", s.tracks_lost));
                                let detection_rate = if s.objects_propagated > 0 {
                                    (s.cataloged_objects as f64 / s.objects_propagated as f64) * 100.0
                                } else { 0.0 };
                                ui.label(format!("Detection rate: {:.2}%", detection_rate));
                            } else {
                                ui.label("No data yet.");
                            }
                            ui.small("Confirmed = stable tracks (>=3 detections).\nTentative = newly forming.\nLost = no recent updates.");
                        });

                        ui.separator();

                        // --- Association Accuracy ---
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Association Accuracy (cumulative)").strong());
                            ui.label(format!("Precision: {:.3}", self.cum_precision()));
                            ui.label(format!("Recall: {:.3}", self.cum_recall()));
                            ui.label(format!("F1: {:.3}", self.cum_f1()));
                            if let Some(s) = &self.last_summary {
                                ui.label(format!("OSPA (this step): {:.2} km", s.ospa));
                            }
                            ui.label(format!("OSPA (mean): {:.2} km", self.cum_mean_ospa()));
                            ui.small("Precision: of claimed tracks, % correctly matched to the right object.\nRecall: of confirmed tracks, % that kept getting updated.\nOSPA: avg position error (km) vs ground truth, capped at 100km.");
                        });

                        ui.separator();

                        // --- Time-series plots ---
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("OSPA over time").strong());
                            let points: PlotPoints = self.ospa_history.clone().into();
                            Plot::new("ospa_plot")
                                .height(120.0)
                                .show_axes([true, true])
                                .show(ui, |plot_ui| {
                                    plot_ui.line(Line::new(points).name("OSPA (km)"));
                                });
                        });

                        ui.group(|ui| {
                            ui.label(egui::RichText::new("F1 over time").strong());
                            let points: PlotPoints = self.f1_history.clone().into();
                            Plot::new("f1_plot")
                                .height(120.0)
                                .show_axes([true, true])
                                .include_y(0.0)
                                .include_y(1.0)
                                .show(ui, |plot_ui| {
                                    plot_ui.line(Line::new(points).name("F1"));
                                });
                        });

                        ui.separator();

                        // --- Collision Watch ---
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Collision Watch").strong());
                            ui.label(format!("Active warnings: {}", self.collisions.len()));
                            if let Some(closest) = self.collisions.iter()
                                .min_by(|a, b| a.miss_distance_km.partial_cmp(&b.miss_distance_km).unwrap()) {
                                ui.label(format!("Closest miss: {:.2} km", closest.miss_distance_km));
                                ui.label(format!("Time to CA: {:.0} s", closest.time_to_closest_approach_s));
                            } else {
                                ui.label("No close approaches.");
                            }
                            ui.small("Pairs of confirmed tracks predicted to pass within the collision threshold during the look-ahead horizon.");
                        });

                        ui.separator();

                        // --- Performance ---
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Performance").strong());
                            if let Some(s) = &self.last_summary {
                                ui.label(format!("Step time: {} ms", s.wall_time_ms));
                            }
                            let report = self.bench.report();
                            ui.label(format!("Mean step: {} ms", report.mean_step_time_ms));
                            ui.label(format!("p95: {} ms | p99: {} ms", report.p95_step_time_ms, report.p99_step_time_ms));
                            ui.label(format!("Steps so far: {}", self.cum_steps));
                            ui.small("Step time = wall-clock time for one simulation step (propagate + observe + track + collide + catalog).");
                        });
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("3D View:");
                ui.checkbox(&mut self.show_earth, "Earth");
                ui.checkbox(&mut self.show_observatories, "Observatories");
                ui.checkbox(&mut self.show_orbits, "Orbital Paths");
                ui.separator();
                ui.label("Camera: Drag to rotate | Scroll to zoom | Click an object to inspect");
            });

            // 3D viewport
            let (response, painter) = ui.allocate_painter(
                ui.available_size(),
                egui::Sense::click_and_drag(),
            );

            // Handle camera controls
            if response.dragged() {
                let delta = response.drag_delta();
                self.camera_rotation_y += delta.x * 0.5;
                self.camera_rotation_x -= delta.y * 0.5;
                self.camera_rotation_x = self.camera_rotation_x.clamp(-89.0, 89.0);
            }

            if response.hover_pos().is_some() {
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    self.camera_distance *= 1.0 - scroll * 0.001;
                    self.camera_distance = self.camera_distance.clamp(8000.0, 30000.0);
                }
            }

            let rect = response.rect;
            let center = rect.center();
            let scale = (rect.width().min(rect.height()) / 2.0) / self.camera_distance;

            // Project 3D to 2D
            let project = |pos: [f64; 3]| -> egui::Pos2 {
                let x = pos[0] as f32;
                let y = pos[1] as f32;
                let z = pos[2] as f32;

                // Apply rotations
                let rot_x = self.camera_rotation_x.to_radians();
                let rot_y = self.camera_rotation_y.to_radians();

                // Rotate around Y axis
                let x1 = x * rot_y.cos() + z * rot_y.sin();
                let z1 = -x * rot_y.sin() + z * rot_y.cos();

                // Rotate around X axis
                let y2 = y * rot_x.cos() - z1 * rot_x.sin();
                let _z2 = y * rot_x.sin() + z1 * rot_x.cos();

                egui::pos2(
                    center.x + x1 * scale,
                    center.y - y2 * scale,
                )
            };

            // Draw Earth sphere
            if self.show_earth {
                let earth_radius = 6371.0;
                let num_segments = 64;

                // Draw sphere outline (multiple circles)
                for i in 0..12 {
                    let lat = (i as f32 - 6.0) * std::f32::consts::PI / 12.0;
                    let r = earth_radius * lat.cos() as f64;
                    let z = earth_radius * lat.sin() as f64;

                    let mut points = Vec::new();
                    for j in 0..=num_segments {
                        let angle = (j as f64 / num_segments as f64) * 2.0 * std::f64::consts::PI;
                        let x = r * angle.cos();
                        let y = r * angle.sin();
                        points.push(project([x, y, z]));
                    }
                    painter.add(egui::Shape::line(
                        points,
                        egui::Stroke::new(1.0, Color32::from_rgb(80, 120, 200)),
                    ));
                }

                // Vertical circles
                for i in 0..8 {
                    let lon = (i as f64 / 8.0) * 2.0 * std::f64::consts::PI;
                    let mut points = Vec::new();
                    for j in 0..=num_segments {
                        let lat = (j as f64 / num_segments as f64 - 0.5) * std::f64::consts::PI;
                        let r = earth_radius * lat.cos();
                        let x = r * lon.cos();
                        let y = r * lon.sin();
                        let z = earth_radius * lat.sin();
                        points.push(project([x, y, z]));
                    }
                    painter.add(egui::Shape::line(
                        points,
                        egui::Stroke::new(1.0, Color32::from_rgb(80, 120, 200)),
                    ));
                }
            }

            // Draw observatories with better visibility
            if self.show_observatories {
                for sensor in &self.sim.sensors {
                    let pos = project(sensor.position);
                    painter.circle_filled(pos, 5.0, Color32::from_rgb(255, 150, 50));
                    painter.circle_stroke(pos, 7.0, egui::Stroke::new(1.5, Color32::from_rgb(255, 200, 100)));
                }
            }

            // Draw orbiting objects
            let sample_rate = if self.sim.config.n_objects > 1000 { 10 } else { 1 };

            // Track the closest object to a click, if the viewport was clicked this frame
            let click_pos = if response.clicked() {
                response.interact_pointer_pos()
            } else {
                None
            };
            let mut closest_click: Option<(usize, f32)> = None;

            for (idx, pos_3d) in self.sim.objects.pos.iter().enumerate() {
                if idx % sample_rate != 0 {
                    continue;
                }

                let pos = project(*pos_3d);
                let is_detected = self.detected_objects_flash.contains(&idx) && self.flash_timer > 0.0;
                let is_tracked = self.tracked_object_ids.contains(&idx);

                let obj_type = self.sim.objects.get_object_type(idx);
                let is_satellite = obj_type == crate::objects::ObjectType::Satellite;
                let (color, radius) = if is_detected {
                    if is_satellite {
                        (Color32::from_rgb(255, 255, 100), 6.0)  // yellow = detected satellite
                    } else {
                        (Color32::from_rgb(100, 255, 100), 4.0)  // green = detected debris
                    }
                } else if is_tracked {
                    if is_satellite {
                        (Color32::from_rgb(255, 200, 50), 4.0)   // amber = tracked satellite
                    } else {
                        (Color32::from_rgb(100, 150, 255), 3.0)  // blue = tracked debris
                    }
                } else {
                    if is_satellite {
                        (Color32::from_rgb(100, 100, 60), 2.5)   // dim yellow = undetected satellite
                    } else {
                        (Color32::from_rgb(60, 60, 60), 1.5)     // dark gray = undetected debris
                    }
                };

                painter.circle_filled(pos, radius, color);

                if let Some(click) = click_pos {
                    let dist = pos.distance(click);
                    if dist <= 8.0 {
                        match closest_click {
                            Some((_, best_dist)) if best_dist <= dist => {}
                            _ => closest_click = Some((idx, dist)),
                        }
                    }
                }
            }

            // Handle click-to-inspect
            if let Some((obj_idx, _)) = closest_click {
                if let Some(track_id) = self.find_track_for_object(obj_idx) {
                    self.selected_track = Some(track_id);
                    self.inspected_object = None;
                } else {
                    self.selected_track = None;
                    self.inspected_object = Some(obj_idx);
                }
            }

            // Draw orbital paths if enabled
            if self.show_orbits {
                let sample_orbits = 20.min(self.sim.config.n_objects / 10);
                for i in 0..sample_orbits {
                    let idx = i * (self.sim.config.n_objects / sample_orbits.max(1));
                    if idx >= self.sim.objects.radius.len() {
                        break;
                    }

                    let r = self.sim.objects.radius[idx];
                    let incl = self.sim.objects.incl[idx];

                    let mut orbit_points = Vec::new();
                    for j in 0..=64 {
                        let theta = (j as f64 / 64.0) * 2.0 * std::f64::consts::PI;
                        let x = r * theta.cos() * incl.cos();
                        let y = r * theta.sin();
                        let z = r * theta.cos() * incl.sin();
                        orbit_points.push(project([x, y, z]));
                    }

                    painter.add(egui::Shape::line(
                        orbit_points,
                        egui::Stroke::new(0.5, Color32::from_rgba_premultiplied(100, 100, 100, 50)),
                    ));
                }
            }

            // Draw collision warnings
            for pair in self.collisions.iter().take(10) {
                for track_id in [pair.track_a, pair.track_b] {
                    if let Some(track) = self.get_track(track_id) {
                        let pos = project(track.predicted_pos);
                        painter.circle_stroke(pos, 8.0, egui::Stroke::new(2.0, Color32::RED));
                    }
                }
            }

            // Draw selected track path
            if let Some(sel_id) = self.selected_track {
                if let Some(track) = self.get_track(sel_id) {
                    let history_points: Vec<_> = track.history()
                        .iter()
                        .map(|obs| project(obs.position))
                        .collect();

                    if history_points.len() >= 2 {
                        painter.add(egui::Shape::line(
                            history_points,
                            egui::Stroke::new(2.0, Color32::YELLOW),
                        ));
                    }
                }
            }

            // Highlight inspected object (no track) with a ring
            if let Some(obj_idx) = self.inspected_object {
                let pos = project(self.sim.objects.get_position(obj_idx));
                painter.circle_stroke(pos, 8.0, egui::Stroke::new(2.0, Color32::YELLOW));
            }
        });
    }
}


/// Run GUI in real-time mode - uses the same GUI but with real CelesTrak data fed in gradually
pub fn run_realtime(config: SimConfig) {
    let mut gui_config = SimConfig::for_realtime_gui();
    gui_config.seed = config.seed;
    gui_config.n_sensors = config.n_sensors;
    gui_config.collision_threshold_km = config.collision_threshold_km;
    gui_config.collision_horizon_s = config.collision_horizon_s;
    gui_config.tracker_type = config.tracker_type;
    
    // Only override celestrak_group if explicitly provided
    if config.celestrak_group.is_some() {
        gui_config.celestrak_group = config.celestrak_group.clone();
        gui_config.celestrak_multi_group = false; // Single group if specified
    }
    // Otherwise keep the default "realtime" multi-group setting
    
    if config.n_objects != SimConfig::default().n_objects {
        gui_config.n_objects = config.n_objects;
    }
    
    let config = gui_config;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_title("LEO Observatory Network - REAL-TIME Live Tracking"),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "leo_sim_realtime_gui",
        options,
        Box::new(|_| Ok(Box::new(LeoApp::new_realtime(config)))),
    );
}
