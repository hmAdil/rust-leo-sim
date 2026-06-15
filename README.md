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

