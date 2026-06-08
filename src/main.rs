mod bench;
mod catalog;
mod collision;
mod config;
mod ground_truth;
mod gui;
mod objects;
mod passive;
mod sensor;
mod sim;
mod spatial;
mod tracker;

use crate::bench::BenchmarkHarness;
use crate::config::SimConfig;
use crate::sim::Simulation;
use serde::Serialize;
use std::env;

#[derive(Serialize)]
struct SimulationReport {
    total_steps: usize,
    total_sim_time_s: f64,
    total_wall_time_s: f64,
    throughput_steps_per_sec: f64,
    mean_precision: f64,
    mean_recall: f64,
    mean_f1: f64,
    benchmark: crate::bench::BenchmarkReport,
}

struct RunMode {
    bench: bool,
    gui: bool,
}

fn parse_args() -> (SimConfig, RunMode) {
    let args: Vec<String> = env::args().collect();
    let mut config = SimConfig::default();
    let mut mode = RunMode {
        bench: false,
        gui: false,
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
    } else if mode.bench {
        run_benchmark(config);
    } else {
        run_simulation(config);
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
        benchmark: bench.report(),
    };
    println!("{}", serde_json::to_string(&report).unwrap());
}
