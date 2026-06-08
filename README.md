# LEO Observatory Network - 3D Space Object Tracking

A high-performance satellite tracking simulation featuring real-time 3D visualization of orbital mechanics, ground-based observatory networks, and collision detection.

## Features

- 🌍 **3D Globe Visualization**: Earth sphere with orbiting objects rendered in real-time
- 🔭 **Observatory Network**: 16 globally distributed ground stations with vision cones
- 📡 **Live Detection System**: Objects flash green when detected, with detailed logging
- ⚠️ **Collision Detection**: Real-time collision risk assessment with visual warnings
- 🛰️ **Object Cataloging**: CSV export of all detected objects with complete state vectors
- 🎮 **Interactive Camera**: Drag to rotate, scroll to zoom

## Quick Start

### Launch the 3D GUI
```bash
cargo run --release -- --gui
```

### Run Batch Simulation
```bash
cargo run --release -- --steps 100 --objects 10000
```

### Run with Custom Parameters
```bash
cargo run --release -- --gui --sensors 20 --objects 500
```

## GUI Controls

**Playback:**
- ▶ Play/⏸ Pause - Start/stop simulation
- ⏭ Step - Advance one time step
- ⟲ Reset - Restart simulation
- Speed Slider - 0.1x to 5.0x speed (logarithmic scale)

**Camera:**
- **Drag** - Rotate view
- **Scroll** - Zoom in/out
- View range: 8,000 - 30,000 km

**Display Options:**
- ☑ Earth - Show Earth sphere with latitude/longitude grid
- ☑ Observatories - Show ground station locations (orange markers)
- ☑ Orbital Paths - Show sample orbital trajectories

## Detection Log Format

Each detection entry shows:
```
✓ OBJ_000123 | T=420s | Collision=false
   Position: [7234, -1023, 4567] km
```

When collision risk is detected:
```
✓ OBJ_000456 | T=420s | Collision=true ⚠ COLLISION RISK
   Position: [7234, -1023, 4567] km
```

## Visual Legend

**Objects:**
- 🟢 Bright Green - Just detected (flash effect)
- 🔵 Blue - Confirmed tracked objects
- ⚪ Dim Gray - Undetected objects

**Observatories:**
- 🟠 Orange markers - Ground station locations

**Warnings:**
- 🔴 Red circles - Collision risk zones

**Selected Track:**
- 🟡 Yellow path - Historical trajectory

## Configuration

Default settings (can be customized in `src/config.rs`):
- Objects: 300 (GUI) / 100,000 (batch)
- Observatories: 16 globally distributed
- Time step: 20s (GUI) / 30s (batch)
- FOV: 60° half-angle per observatory
- Altitude range: 200-2000 km (LEO)

## Output Files

**detected_objects_catalog.csv** - Generated after simulation
```csv
Object_Name,First_Detection_Time_s,Last_Detection_Time_s,
Position_X_km,Position_Y_km,Position_Z_km,
Velocity_X_km_s,Velocity_Y_km_s,Velocity_Z_km_s,
Detection_Count,Tracking_Confidence
```

## Command Line Options

```bash
--gui                  # Launch graphical interface
--steps N             # Number of simulation steps
--objects N           # Number of orbiting objects
--sensors N           # Number of ground observatories
--seed N              # Random seed for reproducibility
--collision-km N      # Collision threshold distance
--bench               # Enable performance benchmarking
```

## Examples

**Small test with lots of observatories:**
```bash
cargo run --release -- --gui --objects 200 --sensors 24
```

**High-speed batch processing:**
```bash
cargo run --release -- --bench --steps 200 --objects 50000
```

**Reproducible simulation:**
```bash
cargo run --release -- --seed 12345 --steps 100
```

## Performance

- **GUI Mode**: 300 objects @ 20-60 FPS
- **Batch Mode**: 100,000 objects @ 40-85ms per step
- Uses Rayon for parallel computation
- Spatial indexing for O(1) proximity queries

## Architecture Highlights

- **Coordinate System**: 3D vectors with Earth's core at origin (0,0,0)
- **Object Naming**: Sequential OBJ_XXXXXX format
- **Vision Cones**: Each observatory has a 60° FOV cone
- **Detection**: Only objects within FOV and above horizon are cataloged
- **Orbital Mechanics**: Circular Keplerian orbits with varied inclinations

## Documentation

See `PROJECT_DOCUMENTATION.md` for complete technical documentation including:
- Detailed architecture
- Component specifications
- Orbital mechanics
- Extension points
- Performance characteristics

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

## License

See LICENSE file for details.

## Tips for Best Visualization

1. Start with **0.5x speed** to clearly observe detections
2. Use **Step mode** to examine individual detection events
3. Enable **Orbital Paths** to see trajectory diversity
4. Zoom to **12,000-15,000 km** for optimal viewing distance
5. Rotate view to see global observatory coverage
6. Watch the detection log for collision warnings
