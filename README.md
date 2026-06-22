# LEO Observatory Network — Space Situational Awareness Simulation

A high-performance Rust-based space surveillance simulation modeling Low Earth Orbit (LEO) satellite tracking through a global network of ground-based observatories. Designed as a classical baseline for Space Situational Awareness (SSA) research and Graph Neural Network (GNN) data association benchmarking, inspired by ISRO's NETRA program.

![Language: Rust](https://img.shields.io/badge/language-Rust-orange.svg)
![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)

---

## Table of Contents

- [Overview](#overview)
- [Core Features](#core-features)
- [Architecture](#architecture)
- [Installation & Usage](#installation--usage)
- [Configuration](#configuration)
- [Output & Metrics](#output--metrics)
- [Performance](#performance)
- [Research Applications](#research-applications)
- [Extending the System](#extending-the-system)

---

## Overview

This simulation models the complete SSA pipeline: orbital propagation → ground-based detection → data association → track maintenance → collision detection → catalog generation. The system handles 100,000+ space objects with realistic sensor characteristics, producing quantitative evaluation metrics suitable for algorithm benchmarking and mission planning.

### Key Capabilities

- **Massive Scale**: Simulate 100,000+ orbital objects with parallel propagation
- **Real Satellite Data**: Track actual satellites using CelesTrak TLE data (131+ satellites: ISS, GPS, Weather)
- **Realistic Detection**: Vision cone-based ground stations with measurement noise
- **Multiple Tracking Algorithms**: Nearest-neighbor and JPDA (Joint Probabilistic Data Association)
- **Advanced Metrics**: Precision, Recall, F1, and OSPA (Optimal Sub-Pattern Assignment)
- **Collision Analysis**: Linear closest-approach prediction with configurable thresholds
- **Interactive Visualization**: 3D GUI with track history and collision warnings
- **Real-Time Live Tracking**: Demonstration mode using actual CelesTrak satellite data
- **Benchmark Suite**: Density sweep analysis for scalability testing
- **Data Export**: CSV export of satellite catalog and TLE data

---

## Core Features

### 1. Orbital Propagation

#### Simple Keplerian (Default)
- Circular orbit approximation for fast computation
- Earth-centered 3D coordinate system (origin at Earth's core)
- Gravitational parameter μ = 398,600.4418 km³/s²
- Parallelized with Rayon for high performance

**Equations:**
```
Orbital velocity: v = √(μ/r)
Orbital period:   T = 2π√(r³/μ)
Position:         r(t) = [r·cos(θ)·cos(i), r·sin(θ), r·cos(θ)·sin(i)]
```

#### SGP4 Propagator (Optional)
- Industry-standard propagation using SGP4/SDP4 models
- Accounts for orbital perturbations (drag, J2 effects)
- Pre-computed constants cached for performance
- Enable via GUI or --sgp4 flag

---

## Architecture

### Module Structure

```
src/
├── main.rs           - Entry point, launches unified GUI
├── config.rs         - Configuration structures (SimConfig, PropagatorType, TrackerType)
├── objects.rs        - Object pool, propagation, orbital parameters
├── sensor.rs         - Ground stations, observations
├── spatial.rs        - Spatial indexing (grid-based)
├── tracker.rs        - Nearest-neighbor tracker
├── jpda.rs           - JPDA tracker
├── ground_truth.rs   - Ground truth table, metrics
├── hungarian.rs      - Hungarian algorithm, OSPA
├── collision.rs      - Collision detection
├── catalog.rs        - Object catalog, CSV export
├── passive.rs        - Passive propagation utilities
├── sim.rs            - Simulation orchestration
├── gui/              - Unified GUI module
│   ├── mod.rs        - Module exports and run() launcher
│   ├── app.rs        - UnifiedApp with AppState routing
│   ├── state.rs      - AppState, ConfigScreenState, BenchmarkState
│   ├── config_screen.rs - Configuration UI and validation
│   ├── simulation_screen.rs - Simulation state and controls
│   ├── benchmark.rs  - Density sweep benchmark
│   └── file_io.rs    - CSV import/export
└── bench.rs          - Benchmark harness
```

### Unified GUI Architecture

The application now uses a two-screen architecture:

1. **ConfigScreen**: Configuration interface with presets, parameter controls, and CSV import
2. **SimulationScreen**: 3D visualization with playback controls, metrics, and export

**State Management:**
- `AppState::Config` - Shows configuration screen
- `AppState::Simulation` - Runs simulation with 3D visualization
- `AppState::Benchmark` - Background density sweep with progress overlay

---

## Installation and Usage

### Prerequisites

- Rust 1.70+ (install from https://rustup.rs)
- Cargo (included with Rust)

### Build

```bash
cd rust-leo-sim
cargo build --release
```

### Launch Application

**Unified GUI (No CLI arguments required):**
```bash
cargo run --release
# or double-click the built executable
```

The application now launches directly into the ConfigScreen. Configure your simulation parameters and click "Launch Simulation" to start.

### Configuration Screen

The ConfigScreen provides:

- **Preset Buttons**: Default, Stress Test, High Fidelity, Benchmark
- **Parameter Controls**: All SimConfig fields with sliders and input fields
- **Data Source Selection**: Generate synthetic objects or import CSV
- **CSV Import**: Browse and validate satellite catalog files
- **Validation**: Real-time configuration validation with error messages

### Simulation Screen

The SimulationScreen provides:

- **Top Bar**: Config button, Play/Pause, Step, Reset, Speed control
- **3D View**: Earth, observatories, orbiting objects with camera controls
- **Detection Log**: Live detection feed with collision warnings
- **Object Inspector**: Detailed track and object information
- **Run Metrics**: Precision, Recall, F1, OSPA, performance statistics
- **Export**: CSV catalog and JSON metrics export

---

## Configuration

### SimConfig Structure

All configuration is now done through the GUI. The `SimConfig` struct contains:

- `n_objects`: Number of simulated objects (1-100,000)
- `n_sensors`: Number of ground observatories (1-100)
- `dt`: Time step in seconds (0.1-1000.0)
- `steps`: Number of simulation steps (1-10,000)
- `seed`: Random seed for reproducibility
- `fov_half_angle`: Sensor field of view (radians)
- `pos_noise_std`: Position measurement noise (km)
- `vel_noise_std`: Velocity measurement noise (km/s)
- `gate_threshold`: Association gate threshold (km)
- `collision_threshold_km`: Collision warning threshold (km)
- `collision_horizon_s`: Time horizon for collision prediction (seconds)
- `propagator`: Simple Keplerian or SGP4
- `tracker_type`: Nearest Neighbor or JPDA
- `stress_test`: Clustered object distribution

---

## Output and Metrics

### Console Output (JSON)

Per-step summary with optional verbose output.

### Catalog Export (CSV)

File: `detected_objects_catalog.csv`

Columns:
- Object_Name, Object_Type, Size_m
- First_Detection_Time_s, Last_Detection_Time_s
- Position_X_km, Position_Y_km, Position_Z_km
- Velocity_X_km_s, Velocity_Y_km_s, Velocity_Z_km_s
- Detection_Count, Tracking_Confidence

### Metrics Export (JSON)

File: `run_metrics_snapshot.json`

Contains cumulative precision, recall, F1, OSPA, and benchmark statistics.

---

## Performance

### Scalability (100K Objects)

- **Propagation**: 2-5 ms (parallel)
- **Spatial indexing**: 5-10 ms
- **Observation**: 20-40 ms (parallel)
- **Tracking**: 10-30 ms
- **Total**: 40-85 ms per step

---

## Research Applications

- Space Situational Awareness simulation
- Observatory network design and coverage analysis
- Tracking algorithm benchmarking (baseline vs GNN)
- Collision assessment and conjunction analysis
- Real-time operations prototyping

---

## Dependencies

```toml
[dependencies]
rayon = "1"              # Parallel computation
rand = "0.8"             # Random number generation
rand_distr = "0.4"       # Statistical distributions
serde = "1"              # Serialization
serde_json = "1"         # JSON output
eframe = "0.29"          # GUI framework (egui)
egui_plot = "0.29"       # Plotting widgets
sgp4 = "2.3"             # SGP4/SDP4 propagator
rfd = "0.14"             # File dialog for CSV import

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

---

## Limitations and Assumptions

- Circular orbits only in simple mode
- No atmospheric effects in simple propagation
- Simplified Gaussian noise model for sensors
- Linear closest approach for collision detection

---

## License

MIT License - See LICENSE file for details

---

## Contact

For questions, issues, or collaboration opportunities, please open an issue on GitHub.