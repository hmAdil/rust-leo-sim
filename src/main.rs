mod bench;
mod catalog;
pub mod celestrak;  // Public for export tool
mod collision;
mod config;
mod ground_truth;
mod gui;
mod hungarian;
mod jpda;
mod objects;
mod passive;
mod sensor;
mod sim;
mod spatial;
mod tracker;

use crate::bench::BenchmarkHarness;
use crate::config::{SimConfig, TrackerType};
use crate::sim::Simulation;
use serde::Serialize;
use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Serialize)]
struct SimulationReport {
    total_steps: usize,
    total_sim_time_s: f64,
    total_wall_time_s: f64,
    throughput_steps_per_sec: f64,
    mean_precision: f64,
    mean_recall: f64,
    mean_f1: f64,
    mean_ospa: f64,
    benchmark: crate::bench::BenchmarkReport,
}

#[derive(Serialize)]
struct DensitySweepResult {
    n_objects: usize,
    mean_precision: f64,
    mean_recall: f64,
    mean_f1: f64,
    mean_step_time_ms: f64,
    mean_tracks_confirmed: f64,
}

struct RunMode {
    bench: bool,
    gui: bool,
    density_sweep: bool,
    realtime_gui: bool,
}

fn parse_args() -> (SimConfig, RunMode) {
    let args: Vec<String> = env::args().collect();
    let mut config = SimConfig::default();
    let mut mode = RunMode {
        bench: false,
        gui: false,
        density_sweep: false,
        realtime_gui: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--steps" => {
                i += 1;
                if i < args.len() {
                    config.steps = args[i].parse().unwrap();
                }
            }
            "--bench" => mode.bench = true,
            "--gui" => mode.gui = true,
            "--realtime-gui" => mode.realtime_gui = true,
            "--objects" => {
                i += 1;
                if i < args.len() {
                    config.n_objects = args[i].parse().unwrap();
                }
            }
            "--sensors" => {
                i += 1;
                if i < args.len() {
                    config.n_sensors = args[i].parse().unwrap();
                }
            }
            "--seed" => {
                i += 1;
                if i < args.len() {
                    config.seed = args[i].parse().unwrap();
                }
            }
            "--collision-km" => {
                i += 1;
                if i < args.len() {
                    config.collision_threshold_km = args[i].parse().unwrap();
                }
            }
            "--sgp4" => {
                config.propagator = crate::config::PropagatorType::Sgp4;
            }
            "--jpda" => {
                config.tracker_type = TrackerType::Jpda;
            }
            "--stress-test" => {
                config.stress_test = true;
            }
            "--density-sweep" => {
                mode.density_sweep = true;
            }
            "--celestrak-group" => {
                i += 1;
                if i < args.len() {
                    config.celestrak_group = Some(args[i].clone());
                    // Force SGP4 when using real data
                    config.propagator = crate::config::PropagatorType::Sgp4;
                }
            }
            "--list-celestrak-groups" => {
                println!("Available CelesTrak satellite groups:");
                println!("{}", "-".repeat(60));
                for (group, description) in celestrak::available_groups() {
                    println!("  {:20} - {}", group, description);
                }
                println!("\nUsage: --celestrak-group <group>");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    (config, mode)
}

fn main() {
    let (config, mode) = parse_args();
    if mode.gui {
        gui::run(config);
    } else if mode.realtime_gui {
        gui::run_realtime(config);
    } else if mode.density_sweep {
        run_density_sweep();
    } else if mode.bench {
        run_benchmark(config);
    } else {
        run_simulation(config);
    }
}

fn run_density_sweep() {
    let object_counts = [100, 500, 1000, 2000, 5000, 10000];
    let steps = 50;
    let seed = 42;
    
    let mut results: Vec<DensitySweepResult> = Vec::new();
    
    for &n_objects in &object_counts {
        eprintln!("Running density sweep for {} objects...", n_objects);
        
        let config = SimConfig {
            n_objects,
            steps,
            seed,
            ..SimConfig::default()
        };
        
        let mut sim = Simulation::new(config);
        let mut all_metrics = Vec::with_capacity(steps);
        let mut step_times: Vec<u128> = Vec::with_capacity(steps);
        let mut tracks_confirmed: Vec<usize> = Vec::with_capacity(steps);
        
        while !sim.finished {
            let result = sim.step_once();
            all_metrics.push(result.metrics);
            step_times.push(result.summary.wall_time_ms);
            tracks_confirmed.push(result.summary.tracks_confirmed);
        }
        
        let n = all_metrics.len().max(1) as f64;
        let mean_precision = all_metrics.iter().map(|m| m.precision()).sum::<f64>() / n;
        let mean_recall = all_metrics.iter().map(|m| m.recall()).sum::<f64>() / n;
        let mean_f1 = all_metrics.iter().map(|m| m.f1()).sum::<f64>() / n;
        let mean_step_time = step_times.iter().sum::<u128>() as f64 / n;
        let mean_tracks = tracks_confirmed.iter().sum::<usize>() as f64 / n;
        
        results.push(DensitySweepResult {
            n_objects,
            mean_precision,
            mean_recall,
            mean_f1,
            mean_step_time_ms: mean_step_time,
            mean_tracks_confirmed: mean_tracks,
        });
    }
    
    // Output JSON array to stdout
    println!("{}", serde_json::to_string(&results).unwrap());
    
    // Export to file
    let json = serde_json::to_string_pretty(&results).unwrap();
    if let Err(e) = File::create("density_sweep_results.json").and_then(|mut f| {
        f.write_all(json.as_bytes())
    }) {
        eprintln!("Failed to write density_sweep_results.json: {}", e);
    } else {
        eprintln!("Results exported to density_sweep_results.json");
    }
}

fn run_simulation(config: SimConfig) {
    let mut sim = Simulation::new(config.clone());
    let mut all_metrics = Vec::with_capacity(config.steps);

    while !sim.finished {
        let result = sim.step_once();
        println!("{}", serde_json::to_string(&result.summary).unwrap());
        all_metrics.push(result.metrics);
    }

    // Export catalog to CSV
    if let Err(e) = sim.catalog.export_to_csv("detected_objects_catalog.csv") {
        eprintln!("Failed to export catalog: {}", e);
    } else {
        println!("Catalog exported to detected_objects_catalog.csv");
    }

    let n = all_metrics.len().max(1) as f64;
    let report = SimulationReport {
        total_steps: config.steps,
        total_sim_time_s: config.steps as f64 * config.dt,
        total_wall_time_s: 0.0,
        throughput_steps_per_sec: 0.0,
        mean_precision: all_metrics.iter().map(|m| m.precision()).sum::<f64>() / n,
        mean_recall: all_metrics.iter().map(|m| m.recall()).sum::<f64>() / n,
        mean_f1: all_metrics.iter().map(|m| m.f1()).sum::<f64>() / n,
        mean_ospa: all_metrics.iter().map(|m| m.ospa).sum::<f64>() / n,
        benchmark: crate::bench::BenchmarkReport::default(),
    };
    println!("{}", serde_json::to_string(&report).unwrap());
}

fn run_benchmark(config: SimConfig) {
    let mut sim = Simulation::new(config.clone());
    let mut bench = BenchmarkHarness::new();
    let mut all_metrics = Vec::with_capacity(config.steps);
    let sim_start = std::time::Instant::now();

    while !sim.finished {
        let result = sim.step_once();
        bench.record_step(result.summary.wall_time_ms);
        all_metrics.push(result.metrics);
        println!("{}", serde_json::to_string(&result.summary).unwrap());
    }

    // Export catalog to CSV
    if let Err(e) = sim.catalog.export_to_csv("detected_objects_catalog.csv") {
        eprintln!("Failed to export catalog: {}", e);
    } else {
        println!("Catalog exported to detected_objects_catalog.csv");
    }

    let total_wall_time = sim_start.elapsed();
    let n = all_metrics.len().max(1) as f64;
    let report = SimulationReport {
        total_steps: config.steps,
        total_sim_time_s: config.steps as f64 * config.dt,
        total_wall_time_s: total_wall_time.as_secs_f64(),
        throughput_steps_per_sec: config.steps as f64 / total_wall_time.as_secs_f64(),
        mean_precision: all_metrics.iter().map(|m| m.precision()).sum::<f64>() / n,
        mean_recall: all_metrics.iter().map(|m| m.recall()).sum::<f64>() / n,
        mean_f1: all_metrics.iter().map(|m| m.f1()).sum::<f64>() / n,
        mean_ospa: all_metrics.iter().map(|m| m.ospa).sum::<f64>() / n,
        benchmark: bench.report(),
    };
    println!("{}", serde_json::to_string(&report).unwrap());
}