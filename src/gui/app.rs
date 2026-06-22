use crate::gui::state::{AppState, ConfigScreenState, DataSource, Preset};
use eframe::egui;

pub struct UnifiedApp {
    pub state: AppState,
}

impl UnifiedApp {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl eframe::App for UnifiedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle state transitions first
        let transition = if let AppState::Config(config_state) = &mut self.state {
            if config_state.launch_requested {
                Some(config_state.config.clone())
            } else {
                None
            }
        } else {
            None
        };

        // Apply state transition
        if let Some(config) = transition {
            if let AppState::Config(config_state) = &mut self.state {
                config_state.launch_requested = false;
            }
            // For now, always transition to simulation (benchmark would be different)
            self.state = AppState::Simulation(Box::new(
                crate::gui::simulation_screen::SimulationScreenState::new(config)
            ));
        }

        match &mut self.state {
            AppState::Config(config_state) => {
                render_config_screen(config_state, ctx);
            }
            AppState::Simulation(sim_state) => {
                render_simulation_screen(ctx, sim_state);
            }
            AppState::Benchmark(benchmark_state) => {
                // Check for thread completion
                if let Some(handle) = &benchmark_state.thread_handle {
                    if handle.is_finished() {
                        benchmark_state.completed = true;
                        benchmark_state.results_path = Some("density_sweep_results.json".to_string());
                        benchmark_state.thread_handle = None;
                    }
                }
                
                render_benchmark_overlay(ctx, benchmark_state);
            }
        }
    }
}

fn render_config_screen(state: &mut ConfigScreenState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("LEO Observatory Configuration");
            ui.separator();
        });
        
        // Preset buttons section
        ui.horizontal(|ui| {
            ui.label("Presets:");
            if ui.button("Default").clicked() {
                Preset::Default.apply(&mut state.config);
                state.is_benchmark_mode = false;
                state.validation_error = None;
            }
            if ui.button("Stress Test").clicked() {
                Preset::StressTest.apply(&mut state.config);
                state.is_benchmark_mode = false;
                state.validation_error = None;
            }
            if ui.button("High Fidelity").clicked() {
                Preset::HighFidelity.apply(&mut state.config);
                state.is_benchmark_mode = false;
                state.validation_error = None;
            }
            if ui.button("Benchmark").clicked() {
                Preset::Benchmark.apply(&mut state.config);
                state.is_benchmark_mode = true;
                state.validation_error = None;
            }
        });
        
        ui.separator();
        
        // Data source section
        ui.horizontal(|ui| {
            ui.label("Data Source:");
            ui.radio_value(&mut state.data_source, DataSource::Generate, "Generate Synthetic Objects");
            ui.radio_value(&mut state.data_source, DataSource::ImportCSV, "Import CSV Catalog");
        });
        
        ui.separator();
        
        // CSV file picker (shown only when ImportCSV is selected)
        if state.data_source == DataSource::ImportCSV {
            ui.horizontal(|ui| {
                if ui.button("Select CSV File...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV Files", &["csv"])
                        .pick_file() 
                    {
                        state.csv_path = Some(path.display().to_string());
                        state.csv_object_count = None;
                        state.validation_error = None;
                    }
                }
                
                if let Some(path) = &state.csv_path {
                    ui.label(format!("Selected: {}", path));
                }
            });
        }
        
        ui.separator();
        
        // Parameters section
        ui.collapsing("Simulation Parameters", |ui| {
            ui.horizontal(|ui| {
                ui.label("n_objects:");
                let enabled = state.data_source != DataSource::ImportCSV;
                ui.add_enabled(enabled, egui::Slider::new(&mut state.config.n_objects, 1..=100_000)
                    .text("objects"));
            });
            
            ui.horizontal(|ui| {
                ui.label("n_sensors:");
                ui.add(egui::Slider::new(&mut state.config.n_sensors, 1..=100).text("sensors"));
            });
            
            ui.horizontal(|ui| {
                ui.label("dt:");
                ui.add(egui::Slider::new(&mut state.config.dt, 0.1..=1000.0)
                    .text("seconds")
                    .logarithmic(true));
            });
            
            ui.horizontal(|ui| {
                ui.label("steps:");
                ui.add(egui::Slider::new(&mut state.config.steps, 1..=10_000).text("steps"));
            });
            
            ui.horizontal(|ui| {
                ui.label("seed:");
                let seed_text = state.config.seed.to_string();
                let mut seed_text = seed_text;
                ui.text_edit_singleline(&mut seed_text);
                if let Ok(seed) = seed_text.parse::<u64>() {
                    state.config.seed = seed;
                }
            });
        });
        
        ui.separator();
        
        // Propagator selection
        ui.horizontal(|ui| {
            ui.label("Propagator:");
            ui.radio_value(&mut state.config.propagator, crate::config::PropagatorType::SimpleKeplerian, "Simple Keplerian");
            ui.radio_value(&mut state.config.propagator, crate::config::PropagatorType::Sgp4, "SGP4");
        });
        
        // Tracker selection
        ui.horizontal(|ui| {
            ui.label("Tracker:");
            ui.radio_value(&mut state.config.tracker_type, crate::config::TrackerType::NearestNeighbor, "Nearest Neighbor");
            ui.radio_value(&mut state.config.tracker_type, crate::config::TrackerType::Jpda, "JPDA");
        });
        
        ui.separator();
        
        // Sensor parameters
        ui.collapsing("Sensor Parameters", |ui| {
            ui.horizontal(|ui| {
                ui.label("FOV Half Angle:");
                ui.add(egui::Slider::new(&mut state.config.fov_half_angle, 0.01..=1.57)
                    .text("radians"));
            });
            
            ui.horizontal(|ui| {
                ui.label("Position Noise Std:");
                ui.add(egui::Slider::new(&mut state.config.pos_noise_std, 0.0..=10.0)
                    .text("km"));
            });
            
            ui.horizontal(|ui| {
                ui.label("Velocity Noise Std:");
                ui.add(egui::Slider::new(&mut state.config.vel_noise_std, 0.0..=1.0)
                    .text("km/s"));
            });
            
            ui.horizontal(|ui| {
                ui.label("Gate Threshold:");
                ui.add(egui::Slider::new(&mut state.config.gate_threshold, 1.0..=100.0)
                    .text("km"));
            });
        });
        
        ui.separator();
        
        // Collision parameters
        ui.collapsing("Collision Parameters", |ui| {
            ui.horizontal(|ui| {
                ui.label("Collision Threshold:");
                ui.add(egui::Slider::new(&mut state.config.collision_threshold_km, 1.0..=1000.0)
                    .text("km"));
            });
            
            ui.horizontal(|ui| {
                ui.label("Collision Horizon:");
                ui.add(egui::Slider::new(&mut state.config.collision_horizon_s, 60.0..=3600.0)
                    .text("seconds"));
            });
            
            ui.checkbox(&mut state.config.stress_test, "Stress Test (Clustered Object Distribution)");
        });
        
        ui.separator();
        
        // Validation error display
        if let Some(error) = &state.validation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), error);
        }
        
        // Launch button
        let launch_text = if state.is_benchmark_mode { "Start Benchmark" } else { "Launch Simulation" };
        ui.vertical_centered(|ui| {
            if ui.button(launch_text).clicked() {
                match crate::gui::config_screen::validate_config(state) {
                    Ok(_) => {
                        state.validation_error = None;
                        state.launch_requested = true;
                    }
                    Err(e) => {
                        state.validation_error = Some(e);
                    }
                }
            }
        });
    });
}

fn render_simulation_screen(ctx: &egui::Context, state: &mut Box<crate::gui::simulation_screen::SimulationScreenState>) {
    // Update flash timer
    if state.flash_timer > 0.0 {
        state.flash_timer -= ctx.input(|i| i.stable_dt);
        if state.flash_timer < 0.0 {
            state.flash_timer = 0.0;
            state.detected_objects_flash.clear();
        }
    }

    if state.playing && !state.sim.finished {
        // Handle fractional speeds
        state.frame_counter += 1;
        let frames_per_step = (1.0 / state.speed).max(1.0) as u32;

        if state.frame_counter >= frames_per_step {
            state.frame_counter = 0;
            let steps_this_frame = state.speed.max(1.0) as usize;

            for _ in 0..steps_this_frame {
                if state.sim.finished {
                    break;
                }
                state.advance();
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    // Top bar with config button and stats
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Config").clicked() {
                // Will be handled by parent
            }
            
            ui.separator();
            
            if ui.button(if state.playing { "Pause" } else { "Play" }).clicked() {
                state.playing = !state.playing;
            }
            if ui.button("Step").clicked() {
                state.advance();
            }
            if ui.button("Reset").clicked() {
                let config = state.sim.config.clone();
                *state = Box::new(crate::gui::simulation_screen::SimulationScreenState::new(config));
            }
            
            ui.separator();
            
            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut state.speed, 0.1..=5.0)
                .text("x")
                .logarithmic(true));
            ui.label(format!("{:.1}x", state.speed));

            ui.separator();
            ui.label(format!("Sim: {:.0}s", state.query_time_s));
            if let Some(s) = &state.last_summary {
                ui.label(format!(
                    "Step {}/{} | Detected: {} ({:.1}%)",
                    s.step,
                    state.sim.config.steps,
                    s.cataloged_objects,
                    (s.cataloged_objects as f64 / s.objects_propagated as f64) * 100.0
                ));
            }

            ui.separator();
            ui.checkbox(&mut state.show_metrics, "Run Metrics");

            ui.separator();
            if ui.button("Export CSV + JSON").clicked() {
                state.export_data();
            }
            if let Some(status) = &state.export_status {
                ui.label(status);
            }
        });
    });

    // Left side panel - Detection Log
    egui::SidePanel::left("detections")
        .resizable(true)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.heading("Live Detection Log");
            ui.separator();

            if !state.detected_objects_flash.is_empty() && state.flash_timer > 0.0 {
                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "OBJECTS DETECTED");
                ui.separator();
            } else {
                ui.colored_label(egui::Color32::GRAY, "Scanning for objects...");
            }

            ui.separator();
            if ui.button("Export Catalog CSV").clicked() {
                let _ = state.sim.catalog.export_to_csv("detected_objects_catalog.csv");
            }
        });

    // Main 3D view
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("3D View:");
            ui.checkbox(&mut state.show_earth, "Earth");
            ui.checkbox(&mut state.show_observatories, "Observatories");
            ui.checkbox(&mut state.show_orbits, "Orbital Paths");
        });
        
        // Simplified 3D rendering placeholder
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        // Handle camera controls
        if response.dragged() {
            let delta = response.drag_delta();
            state.camera_rotation_y += delta.x * 0.5;
            state.camera_rotation_x -= delta.y * 0.5;
            state.camera_rotation_x = state.camera_rotation_x.clamp(-89.0, 89.0);
        }

        if response.hover_pos().is_some() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                state.camera_distance *= 1.0 - scroll * 0.001;
                state.camera_distance = state.camera_distance.clamp(8000.0, 30000.0);
            }
        }

        let rect = response.rect;
        let center = rect.center();
        let scale = (rect.width().min(rect.height()) / 2.0) / state.camera_distance;

        // Project 3D to 2D
        let project = |pos: [f64; 3]| -> egui::Pos2 {
            let x = pos[0] as f32;
            let y = pos[1] as f32;
            let z = pos[2] as f32;

            // Apply rotations
            let rot_x = state.camera_rotation_x.to_radians();
            let rot_y = state.camera_rotation_y.to_radians();

            // Rotate around Y axis
            let x1 = x * rot_y.cos() + z * rot_y.sin();
            let z1 = -x * rot_y.sin() + z * rot_y.cos();

            // Rotate around X axis
            let y2 = y * rot_x.cos() - z1 * rot_x.sin();

            egui::pos2(
                center.x + x1 * scale,
                center.y - y2 * scale,
            )
        };

// Draw Earth sphere (wireframe)
        if state.show_earth {
            let earth_radius = 6371.0;
            let num_segments = 64;

            // Draw sphere outline (multiple circles at different latitudes)
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
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 120, 200)),
                ));
            }

            // Vertical circles (longitude lines)
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
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 120, 200)),
                ));
            }
        }

// Draw observatories
        if state.show_observatories {
            for sensor in &state.sim.sensors {
                let pos = project(sensor.position);
                painter.circle_filled(pos, 5.0, egui::Color32::from_rgb(255, 150, 50));
                painter.circle_stroke(pos, 7.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 200, 100)));
            }
        }

        // Draw orbiting objects
        let sample_rate = if state.sim.config.n_objects > 1000 { 10 } else { 1 };

        // Track closest object for click interaction
        let click_pos = if response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        };
        let mut closest_click: Option<(usize, f32)> = None;

        for (idx, pos_3d) in state.sim.objects.pos.iter().enumerate() {
            if idx % sample_rate != 0 {
                continue;
            }

            let pos = project(*pos_3d);
            let is_detected = state.detected_objects_flash.contains(&idx) && state.flash_timer > 0.0;
            let is_tracked = state.tracked_object_ids.contains(&idx);

            let obj_type = state.sim.objects.get_object_type(idx);
            let (color, radius) = if is_detected {
                if obj_type == crate::objects::ObjectType::Satellite {
                    (egui::Color32::from_rgb(255, 255, 100), 6.0)  // yellow = detected satellite
                } else {
                    (egui::Color32::from_rgb(100, 255, 100), 4.0)  // green = detected debris
                }
            } else if is_tracked {
                if obj_type == crate::objects::ObjectType::Satellite {
                    (egui::Color32::from_rgb(255, 200, 50), 4.0)   // amber = tracked satellite
                } else {
                    (egui::Color32::from_rgb(100, 150, 255), 3.0)  // blue = tracked debris
                }
            } else {
                if obj_type == crate::objects::ObjectType::Satellite {
                    (egui::Color32::from_rgb(100, 100, 60), 2.5)   // dim yellow = undetected satellite
                } else {
                    (egui::Color32::from_rgb(60, 60, 60), 1.5)     // dark gray = undetected debris
                }
            };

            painter.circle_filled(pos, radius, color);

            // Handle click-to-inspect
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

        // Handle click-to-inspect (update simulation state)
        if let Some((obj_idx, _)) = closest_click {
            if let Some(track_id) = state.find_track_for_object(obj_idx) {
                state.selected_track = Some(track_id);
                state.inspected_object = None;
            } else {
                state.selected_track = None;
                state.inspected_object = Some(obj_idx);
            }
        }

        // Draw orbital paths if enabled
        if state.show_orbits {
            let sample_orbits = 20.min(state.sim.config.n_objects / 10);
            for i in 0..sample_orbits {
                let idx = i * (state.sim.config.n_objects / sample_orbits.max(1));
                if idx >= state.sim.objects.radius.len() {
                    break;
                }

                let r = state.sim.objects.radius[idx];
                let incl = state.sim.objects.incl[idx];

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
                    egui::Stroke::new(0.5, egui::Color32::from_rgba_premultiplied(100, 100, 100, 50)),
                ));
            }
        }

        // Draw collision warnings
        for pair in state.collisions.iter().take(10) {
            for track_id in [pair.track_a, pair.track_b] {
                if let Some(track) = state.get_track(track_id) {
                    let pos = project(track.predicted_pos);
                    painter.circle_stroke(pos, 8.0, egui::Stroke::new(2.0, egui::Color32::RED));
                }
            }
        }

        // Draw selected track path
        if let Some(sel_id) = state.selected_track {
            if let Some(track) = state.get_track(sel_id) {
                let history_points: Vec<_> = track.history()
                    .iter()
                    .map(|obs| project(obs.position))
                    .collect();

                if history_points.len() >= 2 {
                    painter.add(egui::Shape::line(
                        history_points,
                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    ));
                }
            }
        }

        // Highlight inspected object (no track) with a ring
        if let Some(obj_idx) = state.inspected_object {
            let pos = project(state.sim.objects.get_position(obj_idx));
            painter.circle_stroke(pos, 8.0, egui::Stroke::new(2.0, egui::Color32::YELLOW));
        }
    });

    // Run metrics side panel
    if state.show_metrics {
        egui::SidePanel::right("run_metrics")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Run Metrics");
                ui.separator();

                if let Some(s) = &state.last_summary {
                    ui.label(format!("Confirmed: {}", s.tracks_confirmed));
                    ui.label(format!("Tentative: {}", s.tracks_tentative));
                    ui.label(format!("Lost: {}", s.tracks_lost));
                    ui.label(format!("Precision: {:.3}", state.cum_precision()));
                    ui.label(format!("Recall: {:.3}", state.cum_recall()));
                    ui.label(format!("F1: {:.3}", state.cum_f1()));
                }
            });
    }
}

fn render_benchmark_overlay(ctx: &egui::Context, state: &mut crate::gui::state::BenchmarkState) {
    // Centered modal overlay
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_size().y / 2.0 - 100.0);
            
            ui.group(|ui| {
                ui.set_min_width(400.0);
                ui.vertical_centered(|ui| {
                    ui.heading("Benchmark Running...");
                    ui.separator();
                    
                    let progress = state.progress.lock().unwrap();
                    ui.label(format!("Objects: {}", progress.current_object_count));
                    ui.label(format!("Progress: {}/{}", progress.current_iteration, progress.total_iterations));
                    
                    let completion = if progress.total_iterations > 0 {
                        progress.current_iteration as f32 / progress.total_iterations as f32
                    } else { 0.0 };
                    ui.add(egui::ProgressBar::new(completion).text(format!("{:.0}%", completion * 100.0)));
                    
                    ui.separator();
                    
                    if ui.button("Cancel").clicked() {
                        state.cancel();
                    }
                    
                    if state.completed {
                        ui.separator();
                        if let Some(path) = &state.results_path {
                            ui.colored_label(egui::Color32::from_rgb(100, 255, 100), 
                                format!("Results saved to: {}", path));
                        }
                    }
                });
            });
        });
    });
}