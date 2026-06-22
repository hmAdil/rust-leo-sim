use crate::config::{PropagatorType, SimConfig, TrackerType};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

/// Data source for object generation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DataSource {
    #[default]
    Generate,
    ImportCSV,
}

/// State for the configuration screen
pub struct ConfigScreenState {
    pub config: SimConfig,
    pub data_source: DataSource,
    pub csv_path: Option<String>,
    pub csv_object_count: Option<usize>,
    pub validation_error: Option<String>,
    pub is_benchmark_mode: bool,
    pub launch_requested: bool,
}

impl Default for ConfigScreenState {
    fn default() -> Self {
        Self {
            config: SimConfig::default(),
            data_source: DataSource::default(),
            csv_path: None,
            csv_object_count: None,
            validation_error: None,
            is_benchmark_mode: false,
            launch_requested: false,
        }
    }
}

impl ConfigScreenState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// State for benchmark running
pub struct BenchmarkState {
    pub progress: Arc<Mutex<BenchmarkProgress>>,
    pub cancel_signal: Arc<AtomicBool>,
    pub thread_handle: Option<JoinHandle<()>>,
    pub completed: bool,
    pub results_path: Option<String>,
}

/// Progress tracking for density sweep benchmark
#[derive(Debug, Clone)]
pub struct BenchmarkProgress {
    pub current_object_count: usize,
    pub total_iterations: usize,
    pub current_iteration: usize,
    pub mean_precision: f64,
    pub mean_recall: f64,
    pub mean_f1: f64,
    pub mean_step_time_ms: f64,
}

impl Default for BenchmarkProgress {
    fn default() -> Self {
        Self {
            current_object_count: 0,
            total_iterations: 6,
            current_iteration: 0,
            mean_precision: 0.0,
            mean_recall: 0.0,
            mean_f1: 0.0,
            mean_step_time_ms: 0.0,
        }
    }
}

impl BenchmarkState {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(BenchmarkProgress::default())),
            cancel_signal: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            completed: false,
            results_path: None,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_signal.load(Ordering::SeqCst)
    }

    pub fn cancel(&self) {
        self.cancel_signal.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancel(&self) {
        self.cancel_signal.store(false, Ordering::SeqCst);
    }
}

/// Application state - either showing config or running simulation
pub enum AppState {
    Config(ConfigScreenState),
    Simulation(Box<crate::gui::simulation_screen::SimulationScreenState>),
    Benchmark(BenchmarkState),
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Config(ConfigScreenState::new())
    }
}

/// Preset scenarios for quick configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Default,
    StressTest,
    HighFidelity,
    Benchmark,
}

impl Preset {
    pub fn apply(&self, config: &mut SimConfig) {
        match self {
            Preset::Default => {
                *config = SimConfig::default();
            }
            Preset::StressTest => {
                config.stress_test = true;
                config.n_objects = 5000;
            }
            Preset::HighFidelity => {
                config.propagator = PropagatorType::Sgp4;
                config.tracker_type = TrackerType::Jpda;
                config.pos_noise_std = 0.1;
                config.n_objects = 1000;
            }
            Preset::Benchmark => {
                config.n_objects = 1000;
                config.steps = 50;
            }
        }
    }
}

