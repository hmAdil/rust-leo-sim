use crate::config::SimConfig;
use crate::gui::state::{BenchmarkProgress, BenchmarkState};
use crate::sim::Simulation;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Result of a single density sweep iteration
#[derive(Debug, Serialize)]
pub struct DensitySweepResult {
    pub n_objects: usize,
    pub mean_precision: f64,
    pub mean_recall: f64,
    pub mean_f1: f64,
    pub mean_step_time_ms: f64,
}

/// Launch the density sweep benchmark in a background thread
pub fn launch_density_sweep(config: &SimConfig) -> BenchmarkState {
    let progress = Arc::new(Mutex::new(BenchmarkProgress::default()));
    let cancel_signal = Arc::new(AtomicBool::new(false));
    
    let progress_clone = progress.clone();
    let cancel_clone = cancel_signal.clone();
    let config_clone = config.clone();
    
    let handle = thread::spawn(move || {
        run_density_sweep(config_clone, progress_clone, cancel_clone);
    });
    
    BenchmarkState {
        progress,
        cancel_signal,
        thread_handle: Some(handle),
        completed: false,
        results_path: None,
    }
}

/// Run density sweep in background thread
fn run_density_sweep(
    _config: SimConfig,
    progress: Arc<Mutex<BenchmarkProgress>>,
    cancel_signal: Arc<AtomicBool>,
) {
    let object_counts = [100, 500, 1000, 2000, 5000, 10000];
    let steps = 50;
    let seed = 42;
    
    let mut results: Vec<DensitySweepResult> = Vec::new();
    
    for (i, &n_objects) in object_counts.iter().enumerate() {
        // Check for cancellation
        if cancel_signal.load(Ordering::SeqCst) {
            return;
        }
        
        {
            let mut p = progress.lock().unwrap();
            p.current_object_count = n_objects;
            p.current_iteration = i + 1;
        }
        
        let config = SimConfig {
            n_objects,
            steps,
            seed,
            ..SimConfig::default()
        };
        
        let mut sim = Simulation::new(config);
        let mut all_metrics = Vec::with_capacity(steps);
        let mut step_times: Vec<u128> = Vec::with_capacity(steps);
        
        while !sim.finished && !cancel_signal.load(Ordering::SeqCst) {
            let result = sim.step_once();
            all_metrics.push(result.metrics);
            step_times.push(result.summary.wall_time_ms);
        }
        
        let n = all_metrics.len().max(1) as f64;
        let mean_precision = all_metrics.iter().map(|m| m.precision()).sum::<f64>() / n;
        let mean_recall = all_metrics.iter().map(|m| m.recall()).sum::<f64>() / n;
        let mean_f1 = all_metrics.iter().map(|m| m.f1()).sum::<f64>() / n;
        let mean_step_time = step_times.iter().sum::<u128>() as f64 / n;
        
        results.push(DensitySweepResult {
            n_objects,
            mean_precision,
            mean_recall,
            mean_f1,
            mean_step_time_ms: mean_step_time,
        });
        
        // Update progress with latest metrics
        {
            let mut p = progress.lock().unwrap();
            p.mean_precision = mean_precision;
            p.mean_recall = mean_recall;
            p.mean_f1 = mean_f1;
            p.mean_step_time_ms = mean_step_time;
        }
    }
    
    // Save results to file
    if !cancel_signal.load(Ordering::SeqCst) {
        let json = serde_json::to_string_pretty(&results).unwrap();
        if let Err(e) = File::create("density_sweep_results.json").and_then(|mut f| {
            f.write_all(json.as_bytes())
        }) {
            eprintln!("Failed to write density_sweep_results.json: {}", e);
        }
    }
}