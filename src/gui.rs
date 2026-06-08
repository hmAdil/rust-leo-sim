use crate::collision::CollisionPair;
use crate::config::SimConfig;
use crate::sim::{Simulation, StepSummary};
use eframe::egui;
use egui::Color32;
use std::collections::HashSet;

pub fn run(config: SimConfig) {
    let mut gui_config = SimConfig::for_gui();
    gui_config.seed = config.seed;
    gui_config.n_sensors = config.n_sensors;
    gui_config.collision_threshold_km = config.collision_threshold_km;
    gui_config.collision_horizon_s = config.collision_horizon_s;
    if config.n_objects != SimConfig::default().n_objects {
        gui_config.n_objects = config.n_objects;
    }
    let config = gui_config;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_title("LEO Observatory Network — 3D Space Object Tracking"),
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
    query_time_s: f64,
    speed: f32,
    camera_rotation_x: f32,
    camera_rotation_y: f32,
    camera_distance: f32,
    detected_objects_flash: HashSet<usize>,
    detecting_sensors: std::collections::HashMap<usize, Vec<u32>>,  // obj_idx -> sensor_ids
    flash_timer: f32,
    show_earth: bool,
    show_observatories: bool,
    show_orbits: bool,
    frame_counter: u32,
}

impl LeoApp {
    fn new(config: SimConfig) -> Self {
        Self {
            sim: Simulation::new(config),
            playing: false,
            last_summary: None,
            collisions: Vec::new(),
            selected_track: None,
            query_time_s: 0.0,
            speed: 0.5,
            camera_rotation_x: 30.0,
            camera_rotation_y: 45.0,
            camera_distance: 15000.0,
            detected_objects_flash: HashSet::new(),
            detecting_sensors: std::collections::HashMap::new(),
            flash_timer: 0.0,
            show_earth: true,
            show_observatories: true,
            show_orbits: true,
            frame_counter: 0,
        }
    }

    fn advance(&mut self) {
        if self.sim.finished {
            return;
        }
        
        // Collect currently detected object indices and which sensors saw them
        let mut newly_detected = HashSet::new();
        let mut sensor_detections: std::collections::HashMap<usize, Vec<u32>> = std::collections::HashMap::new();
        
        let confirmed_tracks = self.sim.tracker.confirmed_track_refs();
        for track in &confirmed_tracks {
            newly_detected.insert(track.object_id);
            
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
        self.flash_timer = 1.0; // Flash for 1 second
        
        let result = self.sim.step_once();
        self.query_time_s = result.summary.sim_time_s;
        self.last_summary = Some(result.summary);
        self.collisions = result.collisions;
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
                if ui.button(if self.playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                    self.playing = !self.playing;
                }
                if ui.button("⏭ Step").clicked() {
                    self.advance();
                }
                if ui.button("⟲ Reset").clicked() {
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
                ui.label(format!("⏱ Sim: {:.0}s", self.query_time_s));
                if let Some(s) = &self.last_summary {
                    ui.label(format!(
                        "🛰 Objects: {} | 📡 Detected: {} ({:.1}%) | ⚠ Collisions: {}",
                        s.objects_propagated,
                        s.cataloged_objects,
                        (s.cataloged_objects as f64 / s.objects_propagated as f64) * 100.0,
                        s.collision_candidates
                    ));
                }
            });
        });

        egui::SidePanel::left("detections")
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.heading("📡 Live Detection Log");
                ui.separator();
                
                if !self.detected_objects_flash.is_empty() && self.flash_timer > 0.0 {
                    ui.colored_label(Color32::from_rgb(100, 255, 100), "● OBJECTS DETECTED!");
                    ui.separator();
                    
                    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        let mut detected_list: Vec<_> = self.detected_objects_flash.iter().copied().collect();
                        detected_list.sort();
                        
                        for obj_idx in detected_list.iter().take(50) {
                            let obj_name = self.sim.objects.get_name(*obj_idx);
                            let pos = self.sim.objects.get_position(*obj_idx);
                            
                            // Check if this object is in a collision warning
                            let has_collision = self.collisions.iter().any(|pair| {
                                if let Some(track_a) = self.sim.tracker.confirmed_track_refs()
                                    .iter().find(|t| t.track_id == pair.track_a) {
                                    if track_a.object_id == *obj_idx {
                                        return true;
                                    }
                                }
                                if let Some(track_b) = self.sim.tracker.confirmed_track_refs()
                                    .iter().find(|t| t.track_id == pair.track_b) {
                                    if track_b.object_id == *obj_idx {
                                        return true;
                                    }
                                }
                                false
                            });
                            
                            let collision_marker = if has_collision { " ⚠ COLLISION" } else { "" };
                            let color = if has_collision { 
                                Color32::from_rgb(255, 100, 100) 
                            } else { 
                                Color32::from_rgb(150, 255, 150) 
                            };
                            
                            ui.colored_label(
                                color,
                                format!("✓ {} | T={:.0}s | Collision={}{}", 
                                    obj_name, 
                                    self.query_time_s,
                                    has_collision,
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
                            
                            ui.label(format!("   Pos: [{:.0}, {:.0}, {:.0}] km", 
                                pos[0], pos[1], pos[2]));
                            ui.separator();
                        }
                        if detected_list.len() > 50 {
                            ui.label(format!("... and {} more", detected_list.len() - 50));
                        }
                    });
                } else {
                    ui.colored_label(Color32::GRAY, "○ Scanning for objects...");
                }
                
                ui.separator();
                ui.heading("🔭 Observatory Network");
                ui.label(format!("Active Stations: {}", self.sim.sensors.len()));
                ui.label(format!("Coverage: Fibonacci sphere distribution"));
                ui.label(format!("FOV per station: {:.0}° cone", self.sim.config.fov_half_angle.to_degrees()));
                
                ui.separator();
                ui.heading("⚠ Collision Warnings");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.collisions.is_empty() {
                        ui.label("✓ No collision risks detected.");
                    } else {
                        ui.colored_label(
                            Color32::from_rgb(255, 150, 50),
                            format!("{} ACTIVE WARNINGS", self.collisions.len())
                        );
                        ui.separator();
                    }
                    for pair in &self.collisions {
                        let label = format!(
                            "Track #{} ↔ #{}\nMiss Δ: {:.2} km\nTime to CA: {:.0}s",
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
                        }
                    }
                });
            });

        egui::SidePanel::right("track_inspector")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.heading("🛰 Object Inspector");
                ui.separator();
                if let Some(track_id) = self.selected_track {
                    if let Some(track) = self.sim.tracker.get_track(track_id) {
                        let object_name = self.sim.objects.get_name(track.object_id);
                        ui.label(format!("Object: {}", object_name));
                        ui.label(format!("Track ID: {}", track.track_id));
                        ui.label(format!("Status: {:?}", track.status));
                        ui.label(format!("Observations: {}", track.history().len()));
                        ui.label(format!("Confidence: {:.2}", track.confidence));

                        let query_ms = (self.query_time_s * 1000.0) as u64;
                        if let Some(est) = track.estimate_at(
                            query_ms,
                            self.sim.config.pos_noise_std,
                            self.sim.config.vel_noise_std,
                        ) {
                            ui.separator();
                            ui.label("📍 State Vector (Earth's core origin):");
                            ui.monospace(format!(
                                "r = [{:.1}, {:.1}, {:.1}] km",
                                est.position[0], est.position[1], est.position[2]
                            ));
                            ui.monospace(format!(
                                "v = [{:.4}, {:.4}, {:.4}] km/s",
                                est.velocity[0], est.velocity[1], est.velocity[2]
                            ));
                            ui.label(format!("Uncertainty: ±{:.2} km", est.position_uncertainty_km));
                        }

                        ui.separator();
                        ui.label("📡 Detection History:");
                        egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
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
                } else {
                    ui.label("Select a collision pair or track to inspect.");
                    ui.separator();
                    ui.label("📊 Catalog Statistics:");
                    ui.label(format!("Detected Objects: {}", self.sim.catalog.len()));
                    ui.label(format!("Total Simulated: {}", self.sim.config.n_objects));
                    let detection_rate = (self.sim.catalog.len() as f64 / self.sim.config.n_objects as f64) * 100.0;
                    ui.label(format!("Detection Rate: {:.2}%", detection_rate));
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("🌍 3D View:");
                ui.checkbox(&mut self.show_earth, "Earth");
                ui.checkbox(&mut self.show_observatories, "Observatories");
                ui.checkbox(&mut self.show_orbits, "Orbital Paths");
                ui.separator();
                ui.label("Camera: Drag to rotate | Scroll to zoom");
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
                    // Draw observatory as a larger marker with outline
                    painter.circle_filled(pos, 5.0, Color32::from_rgb(255, 150, 50));
                    painter.circle_stroke(pos, 7.0, egui::Stroke::new(1.5, Color32::from_rgb(255, 200, 100)));
                }
            }

            // Draw orbiting objects
            let sample_rate = if self.sim.config.n_objects > 1000 { 10 } else { 1 };
            
            for (idx, pos_3d) in self.sim.objects.pos.iter().enumerate() {
                if idx % sample_rate != 0 {
                    continue;
                }

                let pos = project(*pos_3d);
                let is_detected = self.detected_objects_flash.contains(&idx) && self.flash_timer > 0.0;
                
                // Check if it's part of a confirmed track
                let is_tracked = self.sim.tracker.confirmed_track_refs()
                    .iter()
                    .any(|track| track.object_id == idx);

                let (color, radius) = if is_detected {
                    // Flash bright when detected
                    (Color32::from_rgb(100, 255, 100), 5.0)
                } else if is_tracked {
                    // Confirmed track - blue
                    (Color32::from_rgb(100, 150, 255), 3.0)
                } else {
                    // Undetected - dim gray
                    (Color32::from_rgb(60, 60, 60), 2.0)
                };

                painter.circle_filled(pos, radius, color);
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
                    if let Some(track) = self.sim.tracker.get_track(track_id) {
                        let pos = project(track.predicted_pos);
                        painter.circle_stroke(pos, 8.0, egui::Stroke::new(2.0, Color32::RED));
                    }
                }
            }

            // Draw selected track path
            if let Some(sel_id) = self.selected_track {
                if let Some(track) = self.sim.tracker.get_track(sel_id) {
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
        });
    }
}