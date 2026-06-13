# LEO Observatory Network – Space Object Tracking Simulation

A Rust-based Space Situational Awareness (SSA) simulation designed as a classical baseline for future Graph Neural Network (GNN) data-association research inspired by ISRO's NETRA program.

The simulator models thousands of orbiting objects, a global network of ground observatories, multi-sensor tracking, collision detection, and catalog generation while providing quantitative evaluation metrics and benchmarking tools.

---

![Language: Rust](https://img.shields.io/badge/language-Rust-orange.svg)
![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

### Orbital Propagation

* Simple Keplerian orbit propagation
* Optional SGP4 propagation (`--sgp4`)
* Earth-centered 3D coordinate system
* Parallelized object propagation using Rayon

### Observatory Network

* Globally distributed ground stations
* Vision-cone based detection
* Sensor noise simulation
* Multi-sensor observation generation

### Data Association

#### Nearest-Neighbor Tracker (Baseline)

* Spatially gated association
* Fast and scalable
* Default tracking mode

#### JPDA Tracker

Enable with:

```bash
cargo run --release -- --jpda
```

Features:

* Joint Probabilistic Data Association
* Gaussian likelihood-based association
* Soft probabilistic track updates
* Improved handling of dense observation environments

### Evaluation Metrics

The simulator reports:

* Precision
* Recall
* F1 Score
* OSPA (Optimal Sub-Pattern Assignment)

OSPA is computed using a custom Hungarian Algorithm implementation and provides a stronger measure of tracking quality than classification metrics alone.

### Collision Detection

* Closest-approach prediction
* Miss-distance estimation
* Configurable collision threshold
* Collision candidate reporting

### Stress-Test Scenario

Enable with:

```bash
cargo run --release -- --stress-test
```

Creates dense orbital environments using:

* 5 orbital shells

  * 550 km
  * 600 km
  * 650 km
  * 700 km
  * 750 km

Within each shell:

* 70% clustered in hotspot regions
* 30% uniformly distributed

This scenario is intended to expose classical association failure modes and create challenging datasets for future GNN-based approaches.

### Density Benchmark Suite

Enable with:

```bash
cargo run --release -- --density-sweep
```

Runs simulations at:

* 100 objects
* 500 objects
* 1,000 objects
* 2,000 objects
* 5,000 objects
* 10,000 objects

Outputs:

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

Results are:

* Printed to stdout
* Exported to `density_sweep_results.json`

---

## Command Line Options

| Flag               | Description                  |
| ------------------ | ---------------------------- |
| `--gui`            | Launch GUI                   |
| `--bench`          | Run benchmark mode           |
| `--steps N`        | Simulation steps             |
| `--objects N`      | Number of objects            |
| `--sensors N`      | Number of observatories      |
| `--seed N`         | Random seed                  |
| `--collision-km N` | Collision threshold          |
| `--sgp4`           | Enable SGP4 propagation      |
| `--jpda`           | Use JPDA tracker             |
| `--stress-test`    | Dense orbital-shell scenario |
| `--density-sweep`  | Run density benchmark suite  |

---

## Project Structure

```text
src/
├── objects.rs
├── sensor.rs
├── spatial.rs
├── tracker.rs
├── jpda.rs
├── collision.rs
├── catalog.rs
├── ground_truth.rs
├── hungarian.rs
├── sim.rs
├── gui.rs
├── bench.rs
├── config.rs
└── main.rs
```

---

## Research Motivation

This simulator serves as a classical SSA baseline for evaluating future machine-learning approaches to data association.

The newly added:

* JPDA tracker
* OSPA evaluation
* Density benchmark suite
* Orbital stress-test mode

provide quantitative evidence of how traditional tracking methods degrade as orbital density increases and establish a comparison point for future GNN-based association systems.