# LEO Observatory Network - Space Object Tracking System

## Project Overview

This is a high-performance Low Earth Orbit (LEO) satellite tracking simulation system that models a global network of ground-based observatories detecting and cataloging space objects. The system uses a 3D vector coordinate system with Earth's core at the origin, simulating realistic orbital mechanics, sensor vision cones, and collision detection.

## Core Architecture

### Coordinate System

**Globe Model - 3D Vector System**
- Origin: Earth's core (0, 0, 0)
- Position vectors: **r** = [x, y, z] in kilometers from Earth's core
- Velocity vectors: **v** = [vx, vy, vz] in km/s
- Earth radius: 6,371 km
- LEO altitude range: 200-2,000 km above surface (6,571-8,371 km from core)

### Object Naming Convention

Objects are named using the format: **OBJ_XXXXXX**
- Sequential numbering: OBJ_000001, OBJ_000002, ..., OBJ_100000
- IDs are assigned at simulation start and remain constant
- Only detected objects are cataloged

### Observatory Network

**Ground Stations (Observatories)**
- Named as: **OBS_XX** (OBS_00, OBS_01, etc.)
- Positioned on Earth's surface using Fibonacci sphere distribution
- Each has a **vision cone** defined by field-of-view (FOV) half-angle
- Default FOV: 60° (π/3 radians) half-angle
- Detection criteria:
  1. Object must be above horizon (positive dot product with zenith)
  2. Object must be within FOV cone angle
  3. Object must be within detection range (~3,000 km)

### Detection and Cataloging

**Only detected objects are cataloged:**
- Simulates 100,000+ objects in orbit
- Only objects seen by observatories are recorded
- Catalog exports to CSV with complete state vectors
- Tracks position, velocity, detection timestamps, and confidence

## System Components

### 1. Object Pool (`src/objects.rs`)

Manages all orbiting objects in the simulation.

**Data Structure:**
```rust
pub struct ObjectPool {
    pub id: Vec<usize>,           // Sequential ID for OBJ naming
    pub radius: Vec<f64>,         // Orbital radius from Earth's core
    pub incl: Vec<f64>,           // Inclination (radians)
    pub theta0: Vec<f64>,         // Initial true anomaly
    pub period: Vec<f64>,         // Orbital period (seconds)
    pub pos: Vec<[f64; 3]>,       // Position vectors from core
    pub vel: Vec<[f64; 3]>,       // Velocity vectors
    sim_time: f64,                // Current simulation time
}
```

**Key Functions:**
- `new()`: Initialize objects with random orbital parameters
- `propagate(dt)`: Update all object positions using parallel computation
- `get_name(idx)`: Returns formatted name (OBJ_XXXXXX)
- Uses circular Keplerian orbits with simple propagation

**Orbital Mechanics:**
- Gravitational parameter μ = 398,600.4418 km³/s²
- Circular velocity: v = √(μ/r)
- Orbital period: T = 2π√(r³/μ)

### 2. Observatory Network (`src/sensor.rs`)

Ground-based observation stations with vision cones.

**GroundStation Structure:**
```rust
pub struct GroundStation {
    pub id: u32,
    pub name: String,              // "OBS_XX"
    pub position: [f64; 3],        // Location on Earth's surface
    pub zenith: [f64; 3],          // Upward-pointing normal
    pub fov_half_angle: f64,       // Vision cone half-angle
    // ... noise models and buffers
}
```

**Vision Cone Detection:**
1. Query spatial index for nearby objects
2. Calculate vector from observatory to object
3. Check horizon: dot(to_object, zenith) > 0
4. Check FOV: cos(angle) ≥ cos(fov_half_angle)
5. Add measurement noise to simulate real sensors

**Observatory Distribution:**
- Fibonacci sphere algorithm for even global coverage
- Configurable number of stations (default: 8)
- Positions calculated from latitude/longitude

### 3. Object Catalog (`src/catalog.rs`)

Records only detected objects with complete tracking information.

**CatalogEntry Structure:**
```rust
pub struct CatalogEntry {
    pub object_name: String,           // "OBJ_XXXXXX"
    pub first_detection_time: f64,     // Seconds
    pub last_detection_time: f64,      // Seconds
    pub position: [f64; 3],            // Last known position (km)
    pub velocity: [f64; 3],            // Last known velocity (km/s)
    pub detection_count: usize,        // Number of detections
    pub tracking_confidence: f32,      // 0.0 - 1.0
}
```

**CSV Export Format:**
```
Object_Name,First_Detection_Time_s,Last_Detection_Time_s,
Position_X_km,Position_Y_km,Position_Z_km,
Velocity_X_km_s,Velocity_Y_km_s,Velocity_Z_km_s,
Detection_Count,Tracking_Confidence
```

**Usage:**
- Updated every simulation step with confirmed tracks
- Exports to `detected_objects_catalog.csv` at simulation end
- Provides complete 6-DOF state vectors for trajectory prediction

### 4. Track Manager (`src/tracker.rs`)

Associates observations across time to build object tracks.

**Track States:**
- **Tentative**: 1-2 observations, confidence < 0.5
- **Confirmed**: 3+ observations, confidence ≥ 0.9
- **Lost**: No updates for 3+ steps

**Data Association:**
- Spatial bucketing for efficient nearest-neighbor search
- Gating threshold: 5 km (configurable)
- Parallel prediction, sequential association
- Ground truth comparison for metrics

**Track Structure:**
```rust
pub struct Track {
    pub track_id: u64,
    pub observations: Vec<ObservationRecord>,
    pub predicted_pos: [f64; 3],
    pub predicted_vel: [f64; 3],
    pub last_updated_ms: u64,
    pub confidence: f32,
    pub status: TrackStatus,
    pub object_id: usize,              // Links to ObjectPool
}
```

### 5. Spatial Indexing (`src/spatial.rs`)

Accelerates proximity queries for observation and collision detection.

**Grid-Based Spatial Hash:**
- Cell size: 500 km
- 3D grid partitioning
- Parallel grid construction
- Neighbor queries check 27 cells (3³)

### 6. Collision Detection (`src/collision.rs`)

Identifies potential collisions between tracked objects.

**Linear Closest Approach:**
- Projects trajectories forward in time
- Calculates miss distance at closest approach
- Threshold: 10 km (configurable)
- Time horizon: 600 seconds (10 minutes)

**Output:**
```rust
pub struct CollisionPair {
    pub track_a: u64,
    pub track_b: u64,
    pub miss_distance_km: f64,
    pub time_to_closest_approach_s: f64,
}
```

### 7. Simulation Engine (`src/sim.rs`)

Orchestrates all components in each time step.

**Step Sequence:**
1. Propagate all objects (parallel)
2. Rebuild spatial index
3. All observatories observe (parallel)
4. Update ground truth table
5. Associate observations to tracks
6. Detect collision candidates
7. Update catalog with confirmed tracks
8. Export metrics

**Performance:**
- Rayon-based parallelism
- Efficient spatial queries
- Minimal allocations with buffer reuse

### 8. GUI Visualization (`src/gui.rs`)

Real-time visualization of the tracking system.

**Features:**
- **3D Projection Views:**
  - X-Y (Equatorial plane)
  - X-Z (Meridional plane)
  - Earth's core at origin marked
  
- **Track Visualization:**
  - Blue points: Confirmed tracks
  - Yellow points: Tentative tracks
  - Red points: Collision risks
  - Yellow lines: Selected track history

- **Control Panel:**
  - Play/Pause/Step simulation
  - Speed control (1x-10x)
  - Real-time statistics
  
- **Observatory Network Panel:**
  - Shows number of ground stations
  - Lists collision candidates
  - Miss distance and time-to-closest-approach

- **Object Inspector:**
  - Object name (OBJ_XXXXXX)
  - Complete state vector (position & velocity)
  - Detection history with timestamps
  - Observatory IDs that detected object
  - Tracking confidence
  - Catalog statistics

**Window Title:** "LEO Observatory Network — Space Object Tracking & Collision Detection"

## Configuration

### SimConfig (`src/config.rs`)

```rust
pub struct SimConfig {
    pub n_objects: usize,              // Default: 100,000
    pub n_sensors: usize,              // Default: 8 observatories
    pub dt: f64,                       // Time step: 10 seconds
    pub steps: usize,                  // Default: 100 steps
    pub seed: u64,                     // Random seed: 42
    pub fov_half_angle: f64,           // FOV: π/3 radians (60°)
    pub pos_noise_std: f64,            // Position noise: 0.5 km
    pub vel_noise_std: f64,            // Velocity noise: 0.005 km/s
    pub gate_threshold: f64,           // Association gate: 5 km
    pub collision_threshold_km: f64,   // Collision alert: 10 km
    pub collision_horizon_s: f64,      // Look-ahead: 600 seconds
}
```

### GUI Configuration

Reduced object count for real-time performance:
- Objects: 5,000 (vs 100,000 in batch mode)
- Unlimited steps
- Same sensor/collision parameters

## Usage

### Command Line Interface

**Basic Simulation:**
```bash
cargo run --release
```

**With Parameters:**
```bash
cargo run --release -- --steps 200 --objects 50000 --sensors 12
```

**Benchmark Mode:**
```bash
cargo run --release -- --bench --steps 100
```

**GUI Mode:**
```bash
cargo run --release -- --gui
```

**All Options:**
- `--steps N`: Number of simulation steps
- `--objects N`: Number of objects to simulate
- `--sensors N`: Number of ground observatories
- `--seed N`: Random seed for reproducibility
- `--collision-km N`: Collision threshold distance
- `--bench`: Enable performance benchmarking
- `--gui`: Launch graphical interface

### Output Files

**detected_objects_catalog.csv**
- Generated after simulation completes
- Contains only detected objects
- Full state vectors for each cataloged object
- Detection timestamps and confidence scores

**Console Output (JSON)**
```json
{
  "step": 42,
  "sim_time_s": 420.0,
  "objects_propagated": 100000,
  "observations_total": 1523,
  "tracks_confirmed": 845,
  "tracks_tentative": 234,
  "cataloged_objects": 845,
  "collision_candidates": 3
}
```

## Performance Characteristics

### Scalability

**100,000 Objects:**
- Propagation: ~2-5 ms (parallel)
- Spatial indexing: ~5-10 ms
- Observation: ~20-40 ms (8 sensors)
- Tracking: ~10-30 ms
- **Total per step: ~40-85 ms**

### Parallelization

Uses Rayon for:
- Object propagation (data parallel)
- Spatial index construction
- Observatory observations
- Track prediction

Sequential sections:
- Data association (state consistency)
- Catalog updates

### Memory Usage

Approximate for 100,000 objects:
- ObjectPool: ~15 MB
- Tracks: ~1-2 MB (depends on detection rate)
- Spatial index: ~5-10 MB
- Total: ~20-30 MB

## Limitations and Assumptions

### Orbital Mechanics
- Circular orbits only (no eccentricity)
- Simple Keplerian propagation
- No perturbations (drag, J2, solar pressure)
- No orbit determination/refinement

### Sensor Model
- Simplified noise model (Gaussian)
- No atmospheric effects
- No occlusion modeling
- Infinite detection range within FOV

### Tracking
- Nearest-neighbor association only
- No multi-hypothesis tracking
- Simple linear prediction
- No orbit fitting from observations

### Collision Detection
- Linear closest approach only
- No covariance propagation
- No probability of collision calculation

## Extension Points

### Easy Modifications

1. **Change Orbital Regime:**
   - Modify `radius_dist` in `objects.rs`
   - Example: GEO (35,786 km), MEO (2,000-35,786 km)

2. **Adjust Observatory Network:**
   - Change `n_sensors` parameter
   - Modify distribution in `create_sensors()`

3. **Tune Detection Parameters:**
   - FOV angle: `fov_half_angle`
   - Noise levels: `pos_noise_std`, `vel_noise_std`

4. **Export Additional Data:**
   - Add fields to `CatalogEntry`
   - Modify CSV export format

### Advanced Extensions

1. **Realistic Propagators:**
   - Integrate SGP4/SDP4
   - Add perturbation models
   - Use high-precision ephemeris

2. **Advanced Tracking:**
   - Kalman filtering
   - Multi-hypothesis tracking (MHT)
   - Joint probabilistic data association (JPDA)
   - Orbit determination from observations

3. **Improved Collision Analysis:**
   - Covariance propagation
   - Probability of collision (Pc)
   - Conjunction data messages (CDMs)

4. **Sensor Realism:**
   - Atmospheric refraction
   - Occlusion by terrain/Earth
   - Sky brightness constraints
   - Weather effects

## Code Quality

### Performance Optimizations
- Zero-copy buffer reuse
- Parallel iterators with Rayon
- Spatial indexing for O(1) queries
- SIMD-friendly data layout

### Safety
- No unsafe code
- Bounds checking
- Type-safe state management

### Testing
- Unit tests for core algorithms
- Benchmark harness for performance
- Configurable seeds for reproducibility

## Dependencies

```toml
[dependencies]
rayon = "1"              # Parallel computation
rand = "0.8"             # Random number generation
rand_distr = "0.4"       # Statistical distributions
serde = "1"              # Serialization
serde_json = "1"         # JSON output
eframe = "0.29"          # GUI framework
egui_plot = "0.29"       # Plotting widgets
```

## Building and Optimization

### Release Build
```bash
cargo build --release
```

### Profile Settings
```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for best performance
```

### Benchmarking
```bash
cargo run --release -- --bench --steps 100 --objects 100000
```

## Scientific Applications

### Space Situational Awareness (SSA)
- Catalog maintenance
- Debris tracking
- Conjunction assessment

### Observatory Network Design
- Coverage analysis
- Optimal placement studies
- Sensor requirement definition

### Algorithm Development
- Track association methods
- Orbit determination algorithms
- Collision prediction techniques

### Mission Planning
- Launch window analysis
- Deorbit strategy simulation
- Conjunction avoidance maneuvers

## Summary

This system provides a complete pipeline from raw observations to cataloged objects with collision warnings. The 3D vector model with Earth's core as origin accurately represents orbital mechanics, while the observatory network with vision cones realistically simulates ground-based space surveillance.

Key strengths:
- **Scalable**: Handles 100,000+ objects efficiently
- **Realistic**: Vision cone detection, measurement noise
- **Complete**: Full pipeline from detection to catalog
- **Fast**: Parallel computation, spatial indexing
- **Extensible**: Modular design for easy enhancement

The catalog output provides complete 6-DOF state vectors (position + velocity) with timestamps, enabling external tools to predict future positions and analyze orbital characteristics of detected objects.
