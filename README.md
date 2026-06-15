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
- **Realistic Detection**: Vision cone-based ground stations with measurement noise
- **Multiple Tracking Algorithms**: Nearest-neighbor and JPDA (Joint Probabilistic Data Association)
- **Advanced Metrics**: Precision, Recall, F1, and OSPA (Optimal Sub-Pattern Assignment)
- **Collision Analysis**: Linear closest-approach prediction with configurable thresholds
- **Real-time Visualization**: 3D GUI with track history and collision warnings
- **Benchmark Suite**: Density sweep analysis for scalability testing

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
- Enable with `--sgp4` flag

**Trade-offs:**
- Simple Keplerian: ~2-5ms for 100K objects
- SGP4: ~15-30ms for 100K objects (more realistic)

### 2. Observatory Network

Ground stations distributed globally using **Fibonacci sphere algorithm** for optimal coverage.


**Detection Characteristics:**
- **Vision Cone**: Observatories detect objects within a field-of-view cone (default: 60° half-angle)
- **Horizon Check**: Objects must be above horizon (positive dot product with zenith vector)
- **Detection Range**: Approximately 3,000 km maximum
- **Measurement Noise**: Gaussian noise added to position (0.5 km σ) and velocity (0.005 km/s σ)
- **Size-Based Detection Probability**: Uses exponential model `P = 1 - exp(-size/5.0)` where larger objects are easier to detect
- **Signal-to-Noise Ratio (SNR)**: Computed as `(size²/distance²) × 10⁶` for realism

**Object Naming Convention:**
- Observatories: `OBS_00`, `OBS_01`, ..., `OBS_XX`
- Objects: `OBJ_000001`, `OBJ_000002`, ..., `OBJ_XXXXXX`

### 3. Object Types & Characteristics

Objects are categorized as **Satellites** or **Debris** with realistic size distributions:

| Property | Satellites | Debris |
|----------|-----------|---------|
| **Ratio** | 15% of population | 85% of population |
| **Size Distribution** | Normal: μ=2.0m, σ=1.0m | Uniform: 0.01m - 2.0m |
| **RCS Model** | size² (square meters) | size² (square meters) |
| **Detection** | Higher probability | Size-dependent probability |

This models real LEO environments where debris significantly outnumbers active satellites.

### 4. Data Association & Tracking

#### Nearest-Neighbor Tracker (Default)
Fast baseline tracker using spatial gating:

- **Spatial Bucketing**: 50 km grid cells for O(1) proximity queries
- **Association Gate**: 17 km threshold (configurable)
- **Parallel Prediction**: Track propagation parallelized with Rayon
- **Sequential Association**: Maintains state consistency

**Track States:**
- **Tentative**: 1-2 observations, confidence < 0.5
- **Confirmed**: 3+ observations, confidence ≥ 0.9
- **Lost**: No updates for 3+ consecutive steps

**Track Update Logic:**
```rust
confidence = min(1.0, observation_count / 10.0)
predicted_pos = last_pos + last_vel × dt
```

#### JPDA Tracker (Advanced)

Enable with `--jpda` flag.

Joint Probabilistic Data Association implements soft probabilistic matching:

- **Gaussian Likelihood Gating**: Computes association probabilities for all observation-track pairs
- **Soft Updates**: Weighted combination of multiple observations within gate
- **Probability Threshold**: Requires max probability > 0.1 for association
- **Better Dense Environments**: Handles ambiguous observations more effectively

**Probability Computation:**
```
likelihood_i = exp(-0.5 × distance² / gate_threshold²)
probability_i = likelihood_i / Σ(likelihoods)
```

**Weighted Update:**
```
updated_pos = Σ(probability_i × observation_i.position)
updated_vel = Σ(probability_i × observation_i.velocity)
```

### 5. Evaluation Metrics

#### Classification Metrics
Standard metrics computed by comparing track associations to ground truth:

- **Precision**: TP / (TP + FP) — fraction of confirmed tracks that are correct
- **Recall**: TP / (TP + FN) — fraction of true objects successfully tracked
- **F1 Score**: 2 × (
**Detection Characteristics:**
- Vision Cone: Observatories detect objects within FOV cone (default 60 degree half-angle)
- Horizon Check: Objects must be above horizon
- Detection Range: ~3000 km maximum
- Measurement Noise: Gaussian noise on position (0.5 km) and velocity (0.005 km/s)
- Size-Based Detection: Exponential probability model favoring larger objects
- SNR Computation: Based on object size and distance

**Object Naming:**
- Observatories: OBS_00, OBS_01, etc.
- Objects: OBJ_000001, OBJ_000002, etc.

### 3. Object Types and Characteristics

Objects categorized as Satellites or Debris with realistic size distributions:

**Satellites (15% of population):**
- Size: Normal distribution, mean=2.0m, std=1.0m
- Higher detection probability

**Debris (85% of population):**
- Size: Uniform distribution, 0.01m to 2.0m
- Detection probability depends on size

**RCS Model:** size squared (square meters)

### 4. Data Association and Tracking

#### Nearest-Neighbor Tracker (Default)

Fast baseline using spatial gating:
- Spatial Bucketing: 50 km grid cells
- Association Gate: 17 km threshold
- Parallel prediction, sequential association

**Track States:**
- Tentative: 1-2 observations, confidence < 0.5
- Confirmed: 3+ observations, confidence >= 0.9
- Lost: No updates for 3+ steps

#### JPDA Tracker (Advanced)

Enable with --jpda flag.

Joint Probabilistic Data Association with soft probabilistic matching:
- Gaussian likelihood gating
- Weighted combination of observations
- Better handling of dense environments

### 5. Evaluation Metrics

**Classification Metrics:**
- Precision: TP / (TP + FP)
- Recall: TP / (TP + FN)
- F1 Score: Harmonic mean of precision and recall

**OSPA (Optimal Sub-Pattern Assignment):**
- Hungarian algorithm implementation for optimal matching
- Measures both localization and cardinality errors
- Cutoff: 100 km, order parameter: 2.0
- Capped at 200 tracks for O(n3) complexity management

### 6. Collision Detection

Linear closest-approach prediction:
- Projects trajectories forward in time
- Computes miss distance at closest approach
- Default threshold: 10 km
- Time horizon: 600 seconds (10 minutes)

**Output:**
```rust
CollisionPair {
    track_a: u64,
    track_b: u64,
    miss_distance_km: f64,
    time_to_closest_approach_s: f64,
}
```

### 7. Stress-Test Scenario

Enable with --stress-test flag.

Creates dense orbital environments for algorithm testing:

**5 Orbital Shells:**
- 550 km altitude (6921 km from core)
- 600 km altitude (6971 km from core)
- 650 km altitude (7021 km from core)
- 700 km altitude (7071 km from core)
- 750 km altitude (7121 km from core)

**Distribution:**
- 70% clustered in 3 hotspots per shell (25 km std dev)
- 30% uniformly distributed
- Exposes classical tracker failure modes

### 8. Density Benchmark Suite

Enable with --density-sweep flag.

Runs simulations at multiple object counts:
- 100, 500, 1000, 2000, 5000, 10000 objects
- 50 steps per configuration
- Exports JSON results to density_sweep_results.json

**Output Format:**
```json
[
  {
    "n_objects": 1000,
    "mean_precision": 0.96,
    "mean_recall": 0.81,
    "mean_f1": 0.88,
    "mean_step_time_ms": 4.24,
    "mean_tracks_confirmed": 251.58
  }
]
```

### 9. Real-Time GUI

Enable with --gui flag.

**Features:**
- 3D projection views (X-Y equatorial, X-Z meridional)
- Color-coded tracks (blue=confirmed, yellow=tentative, red=collision risk)
- Track history visualization
- Play/pause/step controls
- Speed control (1x-10x)
- Real-time statistics panel
- Observatory network display
- Object inspector with complete state vectors

---

## Architecture

### Coordinate System

**3D Vector System with Earth's Core at Origin:**
- Position vectors: r = [x, y, z] in km from Earth's core
- Velocity vectors: v = [vx, vy, vz] in km/s
- Earth radius: 6371 km
- LEO altitude: 200-2000 km above surface (6571-8371 km from core)

### Module Structure

```
src/
├── main.rs           - CLI parsing, run modes
├── config.rs         - Configuration structures
├── objects.rs        - Object pool, propagation
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
├── gui.rs            - Real-time visualization
└── bench.rs          - Benchmark harness
```

### Simulation Loop

1. **Propagate**: Update all object positions (parallel)
2. **Index**: Rebuild spatial grid
3. **Observe**: All observatories detect objects (parallel)
4. **Associate**: Match observations to tracks
5. **Update**: Refresh track states and confidence
6. **Collide**: Check for collision candidates
7. **Catalog**: Update detected object catalog
8. **Export**: Output metrics and summaries

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

### Run Modes

**Basic Simulation:**
```bash
cargo run --release
```

**GUI Mode:**
```bash
cargo run --release -- --gui
```

**Benchmark Mode:**
```bash
cargo run --release -- --bench --steps 100
```

**Density Sweep:**
```bash
cargo run --release -- --density-sweep
```

**Stress Test with JPDA:**
```bash
cargo run --release -- --stress-test --jpda --steps 200
```

**SGP4 Propagation:**
```bash
cargo run --release -- --sgp4 --objects 10000 --steps 50
```

### Command-Line Options

| Flag | Description | Default |
|------|-------------|---------|
| --gui | Launch real-time GUI | Off |
| --bench | Enable benchmark mode | Off |
| --steps N | Number of simulation steps | 100 |
| --objects N | Number of objects | 100000 |
| --sensors N | Number of observatories | 20 |
| --seed N | Random seed | 42 |
| --collision-km N | Collision threshold (km) | 10.0 |
| --sgp4 | Use SGP4 propagator | Simple Keplerian |
| --jpda | Use JPDA tracker | Nearest-neighbor |
| --stress-test | Dense orbital shells | Uniform distribution |
| --density-sweep | Run density benchmark | Off |

---

## Configuration

### SimConfig Structure

```rust
pub struct SimConfig {
    pub n_objects: usize,              // 100000
    pub n_sensors: usize,              // 20
    pub dt: f64,                       // 30.0 seconds
    pub steps: usize,                  // 100
    pub seed: u64,                     // 42
    pub fov_half_angle: f64,           // PI/3 (60 degrees)
    pub pos_noise_std: f64,            // 0.5 km
    pub vel_noise_std: f64,            // 0.005 km/s
    pub gate_threshold: f64,           // 17.0 km
    pub collision_threshold_km: f64,   // 10.0 km
    pub collision_horizon_s: f64,      // 600.0 seconds
    pub propagator: PropagatorType,    // SimpleKeplerian | Sgp4
    pub tracker_type: TrackerType,     // NearestNeighbor | Jpda
    pub stress_test: bool,             // false
    pub satellite_ratio: f64,          // 0.15 (15% satellites)
    pub satellite_size_mean: f64,      // 2.0 meters
    pub satellite_size_std: f64,       // 1.0 meters
    pub debris_size_min: f64,          // 0.01 meters
    pub debris_size_max: f64,          // 2.0 meters
}
```

### GUI Configuration

Reduced object count for real-time performance:
- Objects: 300 (vs 100000 in batch mode)
- Sensors: 24
- Time step: 20 seconds
- Unlimited steps

---

## Output and Metrics

### Console Output (JSON)

Per-step summary:
```json
{
  "step": 42,
  "sim_time_s": 1260.0,
  "wall_time_ms": 67,
  "objects_propagated": 100000,
  "observations_total": 1523,
  "tracks_confirmed": 845,
  "tracks_tentative": 234,
  "tracks_lost": 12,
  "association_tp": 820,
  "association_fp": 25,
  "association_fn": 18,
  "collision_candidates": 3,
  "cataloged_objects": 845,
  "ospa": 8.42
}
```

### Catalog Export (CSV)

File: detected_objects_catalog.csv

Columns:
- Object_Name: OBJ_XXXXXX format
- First_Detection_Time_s: Initial detection timestamp
- Last_Detection_Time_s: Most recent detection
- Position_X_km, Position_Y_km, Position_Z_km: Last known position
- Velocity_X_km_s, Velocity_Y_km_s, Velocity_Z_km_s: Last known velocity
- Detection_Count: Number of detections
- Tracking_Confidence: 0.0 to 1.0

**Important:** Catalog contains only detected objects, not all simulated objects.

### Benchmark Report

```json
{
  "total_steps": 100,
  "total_sim_time_s": 3000.0,
  "total_wall_time_s": 8.234,
  "throughput_steps_per_sec": 12.14,
  "mean_precision": 0.967,
  "mean_recall": 0.843,
  "mean_f1": 0.901,
  "mean_ospa": 12.56,
  "benchmark": {
    "mean_step_time_ms": 82.34,
    "median_step_time_ms": 79.12,
    "p95_step_time_ms": 95.67,
    "p99_step_time_ms": 103.21
  }
}
```

---

## Performance

### Scalability (100K Objects)

**Timing Breakdown:**
- Propagation: 2-5 ms (parallel)
- Spatial indexing: 5-10 ms
- Observation: 20-40 ms (8-20 sensors, parallel)
- Tracking: 10-30 ms
- **Total: 40-85 ms per step**

### Parallelization Strategy

**Rayon-based parallelism:**
- Object propagation (data parallel)
- Spatial index construction
- Observatory observations
- Track prediction

**Sequential sections:**
- Data association (state consistency)
- Catalog updates

### Memory Usage (100K Objects)

- ObjectPool: ~15 MB
- Tracks: 1-2 MB
- Spatial index: 5-10 MB
- **Total: ~20-30 MB**

### Performance Comparison

| Objects | Propagator | Time/Step | Memory |
|---------|-----------|-----------|--------|
| 1K | Keplerian | ~2 ms | <5 MB |
| 10K | Keplerian | ~8 ms | ~10 MB |
| 100K | Keplerian | ~65 ms | ~25 MB |
| 100K | SGP4 | ~180 ms | ~30 MB |

---

## Research Applications

### Space Situational Awareness

- **Catalog Maintenance**: Simulate catalog update workflows
- **Debris Tracking**: Model detection and tracking of small debris
- **Conjunction Assessment**: Collision candidate identification

### Observatory Network Design

- **Coverage Analysis**: Test different sensor configurations
- **Placement Optimization**: Evaluate global distribution strategies
- **Sensor Requirements**: Define FOV and sensitivity needs

### Algorithm Development

- **Baseline Performance**: Establish classical tracker metrics
- **GNN Benchmarking**: Provide comparison data for neural approaches
- **Association Testing**: Stress-test data association algorithms
- **Orbit Determination**: Use observations for orbit fitting research

### Mission Planning

- **Launch Window Analysis**: Simulate congested orbital regimes
- **Deorbit Strategy**: Model debris removal scenarios
- **Conjunction Avoidance**: Evaluate collision risk over time

---

## Extending the System

### Easy Modifications

**Change Orbital Regime:**
```rust
// In objects.rs, modify radius_dist
let radius_dist = Uniform::new(42164.0, 42164.0); // GEO
```

**Adjust Observatory Network:**
```rust
// Change number of sensors
cargo run --release -- --sensors 50
```

**Tune Detection Parameters:**
```rust
// Modify SimConfig defaults in config.rs
fov_half_angle: PI / 4.0,  // 45 degrees
pos_noise_std: 0.1,        // 100 meters
```

**Export Additional Data:**
```rust
// Add fields to CatalogEntry in catalog.rs
pub struct CatalogEntry {
    // ... existing fields
    pub orbit_type: String,
    pub last_maneuver_time: Option<f64>,
}
```

### Advanced Extensions

**1. Realistic Propagators**
- Integrate J2 perturbations
- Add atmospheric drag models
- Use GMAT or Orekit for high-fidelity propagation

**2. Advanced Tracking**
- Implement Kalman filtering
- Add multi-hypothesis tracking (MHT)
- Orbit determination from observations

**3. Improved Collision Analysis**
- Covariance propagation
- Probability of collision (Pc) computation
- Conjunction Data Message (CDM) generation

**4. Sensor Realism**
- Atmospheric refraction
- Terrain occlusion
- Sky brightness constraints
- Weather effects
- Sensor tasking optimization

**5. Multi-Sensor Fusion**
- Combine radar and optical observations
- Cross-sensor track correlation
- Distributed tracking architectures

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

[profile.release]
opt-level = 3            # Maximum optimization
lto = true               # Link-time optimization
codegen-units = 1        # Single codegen unit
```

---

## Limitations and Assumptions

### Orbital Mechanics
- Circular orbits only (no eccentricity in Keplerian mode)
- No perturbations in simple mode (drag, J2, solar pressure)
- No orbit determination from observations

### Sensor Model
- Simplified Gaussian noise model
- No atmospheric effects in simple mode
- No occlusion modeling
- Infinite detection range within FOV cone

### Tracking
- No multi-hypothesis tracking
- Simple linear prediction
- No orbit fitting from observation sequences

### Collision Detection
- Linear closest approach only
- No covariance propagation
- No probability of collision calculation

---

## Contributing

Contributions welcome! Areas of interest:
- Realistic orbit determination algorithms
- Multi-hypothesis tracking implementations
- Advanced sensor models
- GPU acceleration for massive simulations
- Machine learning integration

---

## License

MIT License - See LICENSE file for details

---

## Citation

If you use this simulator in your research, please cite:

```bibtex
@software{leo_sim_2024,
  title = {LEO Observatory Network: Space Situational Awareness Simulation},
  author = {Your Name},
  year = {2024},
  url = {https://github.com/yourusername/rust-leo-sim}
}
```

---

## Acknowledgments

- Inspired by ISRO's NETRA (Network for space object Tracking and Analysis) program
- SGP4 propagator from the sgp4 Rust crate
- Hungarian algorithm implementation for OSPA metric computation

---

## Contact

For questions, issues, or collaboration opportunities, please open an issue on GitHub.
