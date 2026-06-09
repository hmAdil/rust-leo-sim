# LEO Observatory Network - 3D Space Object Tracking

A high-performance satellite tracking simulation featuring real-time 3D visualization of orbital mechanics, ground-based observatory networks, and collision detection with **SGP4 support**.

![Language: Rust](https://img.shields.io/badge/language-Rust-orange.svg)
![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- 🌍 **3D Globe Visualization**: Earth sphere with orbiting objects rendered in real-time
- 🔭 **Observatory Network**: 24 globally distributed ground stations with vision cones
- 📡 **Live Detection System**: Objects flash green when detected, with detailed logging
- ⚠️ **Collision Detection**: Real-time collision risk assessment with visual warnings
- 🛰️ **Object Cataloging**: CSV export of all detected objects with complete state vectors
- 🎮 **Interactive Camera**: Drag to rotate, scroll to zoom
- 🚀 **SGP4 Propagator**: Industry-standard realistic orbital mechanics (optional)

## Quick Start

### Launch 3D GUI (Recommended)
```bash
cargo run --release -- --gui
```

### With SGP4 Realistic Orbits
```bash
cargo run --release -- --gui --sgp4
```

### Run Batch Simulation
```bash
cargo run --release -- --steps 100 --objects 10000
```

## Installation

```bash
git clone <repository-url>
cd leo_sim
cargo build --release
```

## GUI Controls

**Playback:**
- ▶ Play/⏸ Pause - Start/stop simulation
- ⏭ Step - Advance one time step
- ⟲ Reset - Restart simulation
- Speed Slider - 0.1x to 5.0x speed (logarithmic scale)

**Camera:**
- **Drag** - Rotate view around Earth
- **Scroll** - Zoom in/out (8,000 - 30,000 km range)

**Display Options:**
- ☑ Earth - Show Earth sphere with latitude/longitude grid
- ☑ Observatories - Show ground station locations (orange markers)
- ☑ Orbital Paths - Show sample orbital trajectories

## Detection Log Format

Each detection entry shows which observatories detected the object:

```
✓ OBJ_000123 | T=420s | Collision=false
   Detected by: OBS_03, OBS_08, OBS_15
   Pos: [7234, -1023, 4567] km
```

When collision risk is detected:
```
✓ OBJ_000456 | T=420s | Collision=true ⚠ COLLISION
   Detected by: OBS_01, OBS_12
   Pos: [7234, -1023, 4567] km
```

## Visual Legend

**Objects:**
- 🟢 Bright Green - Just detected (flash effect)
- 🔵 Blue - Confirmed tracked objects
- ⚪ Dim Gray - Undetected objects

**Observatories:**
- 🟠 Orange markers with yellow outline - Ground stations

**Warnings:**
- 🔴 Red circles - Collision risk zones

**Selected Track:**
- 🟡 Yellow path - Historical trajectory

## Propagation Modes

### Simple Keplerian (Default - Fast)
```bash
cargo run --release -- --gui --objects 1000
```
- Circular orbits only
- Very fast (2-5ms for 100k objects)
- Perfect for visualization and testing

### SGP4 Realistic (Accurate)
```bash
cargo run --release -- --gui --objects 200 --sgp4
```
- Industry-standard NORAD propagator
- Atmospheric drag and perturbations
- Orbital decay over time
- Elliptical orbits
- Best for < 5,000 objects

## Command Line Options

```bash
--gui                  # Launch graphical interface
--steps N             # Number of simulation steps
--objects N           # Number of orbiting objects
--sensors N           # Number of ground observatories
--seed N              # Random seed for reproducibility
--collision-km N      # Collision threshold distance
--sgp4                # Use SGP4 propagator (realistic orbits)
--bench               # Enable performance benchmarking
```

## Example Commands

**Small test with lots of observatories:**
```bash
cargo run --release -- --gui --objects 200 --sensors 32
```

**Realistic SGP4 simulation:**
```bash
cargo run --release -- --gui --objects 300 --sgp4 --sensors 24
```

**High-speed batch processing:**
```bash
cargo run --release -- --bench --steps 200 --objects 50000
```

**Reproducible simulation:**
```bash
cargo run --release -- --seed 12345 --steps 100
```

## Output Files

**detected_objects_catalog.csv** - Generated after simulation
```csv
Object_Name,First_Detection_Time_s,Last_Detection_Time_s,
Position_X_km,Position_Y_km,Position_Z_km,
Velocity_X_km_s,Velocity_Y_km_s,Velocity_Z_km_s,
Detection_Count,Tracking_Confidence
```

## Performance

- **GUI Mode (Keplerian)**: 1,000 objects @ 60 FPS
- **GUI Mode (SGP4)**: 300 objects @ 60 FPS
- **Batch Mode (Keplerian)**: 100,000 objects @ 2-5ms per step
- **Batch Mode (SGP4)**: 10,000 objects @ 50-100ms per step
- Uses Rayon for parallel computation
- Spatial indexing for O(1) proximity queries

## Architecture Highlights

- **Coordinate System**: 3D vectors with Earth's core at origin (0,0,0)
- **Object Naming**: Sequential OBJ_XXXXXX format
- **Vision Cones**: Each observatory has a 60° FOV cone
- **Detection**: Only objects within FOV and above horizon are cataloged
- **Observatory Distribution**: Sunflower seed arrangement for optimal global coverage
- **Orbital Mechanics**: 
  - Simple: Circular Keplerian orbits
  - SGP4: Realistic perturbations, drag, and decay

## Documentation

- `PROJECT_DOCUMENTATION.md` - Complete technical documentation
- `SGP4_USAGE.md` - Detailed SGP4 propagator guide

## Building

**Debug build:**
```bash
cargo build
```

**Optimized release build:**
```bash
cargo build --release
```

**Check code quality:**
```bash
cargo clippy
```

## Dependencies

- `rayon` - Parallel computation
- `rand` - Random number generation  
- `eframe` - GUI framework
- `egui_plot` - Plotting (batch mode)
- `serde` - Serialization
- `sgp4` - Realistic orbital propagator

## Tips for Best Visualization

1. Start with **0.5x speed** to clearly observe detections
2. Use **Step mode** to examine individual detection events
3. Enable **Orbital Paths** to see trajectory diversity
4. Zoom to **12,000-15,000 km** for optimal viewing distance
5. Rotate view to see global observatory coverage
6. Watch the detection log for collision warnings
7. Try **--sgp4 mode** to see realistic orbital decay

## System Requirements

- Rust 1.70+
- Windows/Linux/macOS
- 4GB RAM minimum (8GB recommended for large simulations)
- GPU with OpenGL 3.3+ for GUI

## License

See LICENSE file for details.

## Contributing

Contributions welcome! Areas for enhancement:
- Load real TLE data from Space-Track.org
- Implement orbit determination from observations
- Add more propagator options (numerical integration)
- Enhanced collision probability calculations
- Real-time data feeds

---

**Ready to track satellites!** 🛰️🌍📡
