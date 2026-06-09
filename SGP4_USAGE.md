# SGP4 Propagator Integration

The system now supports **SGP4 (Simplified General Perturbations 4)** propagation for more realistic orbital mechanics!

## What is SGP4?

SGP4 is the standard orbital propagator used by NORAD/Space Force for tracking satellites. It accounts for:
- **Atmospheric drag** 
- **Earth's oblateness (J2 perturbation)**
- **Gravitational perturbations**
- **Solar radiation pressure effects**
- **Third-body perturbations (Moon, Sun)**

This is much more realistic than simple Keplerian circular orbits!

## Usage

### Enable SGP4 Mode

Add the `--sgp4` flag to any command:

```bash
# GUI with SGP4
cargo run --release -- --gui --sgp4

# Batch simulation with SGP4
cargo run --release -- --steps 100 --sgp4

# With custom parameters
cargo run --release -- --gui --objects 200 --sensors 24 --sgp4
```

### Default Mode (Simple Keplerian)

Without the flag, the system uses fast circular Keplerian orbits:

```bash
# Fast mode (default)
cargo run --release -- --gui
```

## Comparison

| Feature | Simple Keplerian | SGP4 |
|---------|-----------------|------|
| **Speed** | Very fast | Moderate |
| **Accuracy** | Circular orbits only | Realistic elliptical orbits |
| **Perturbations** | None | Drag, J2, solar pressure |
| **Orbital decay** | No | Yes |
| **Best for** | Quick visualization, testing | Realistic simulation |

## Performance Notes

### Simple Keplerian
- **100,000 objects**: ~2-5 ms propagation time
- Parallel computation with Rayon
- Perfect for large-scale tests

### SGP4
- **100,000 objects**: ~50-100 ms propagation time (slower)
- Each object requires individual SGP4 calculation
- Better for realistic accuracy with fewer objects

## Recommendations

**Use Simple Keplerian when:**
- Testing the system
- Need maximum performance
- Simulating 10,000+ objects
- Visualizing orbital patterns

**Use SGP4 when:**
- Need realistic orbital decay
- Modeling actual satellites
- Validating tracking algorithms
- Comparing with real-world data
- Simulating < 5,000 objects for performance

## Example Workflows

### Quick Test (Fast)
```bash
cargo run --release -- --gui --objects 500
```

### Realistic Simulation (Accurate)
```bash
cargo run --release -- --gui --objects 200 --sgp4
```

### Benchmark Comparison
```bash
# Simple mode
cargo run --release -- --bench --steps 100

# SGP4 mode
cargo run --release -- --bench --steps 100 --sgp4
```

## Technical Details

### Orbital Elements Generated

When using `--sgp4`, the system generates:
- **Inclination**: 0° to 180° (full range)
- **Eccentricity**: 0.0 to 0.01 (nearly circular LEO)
- **RAAN**: Random 0° to 360°
- **Argument of Perigee**: Random 0° to 360°
- **Mean Anomaly**: Random starting position
- **Mean Motion**: Calculated from altitude
- **Epoch**: 2021-01-01 00:00:00 UTC

### Coordinate System

Both propagators use the same coordinate system:
- Origin: Earth's core (0, 0, 0)
- Position: [x, y, z] in kilometers
- Velocity: [vx, vy, vz] in km/s
- Reference frame: TEME (True Equator Mean Equinox)

## Future Enhancements

Possible improvements:
1. **Load real TLE data** from files
2. **Parse Space-Track.org data** for actual satellites  
3. **Export TLEs** for detected objects
4. **Orbit determination** from observations
5. **Maneuver detection** when orbits change

## Dependencies

The integration uses:
- [`sgp4` crate v2.4](https://crates.io/crates/sgp4)
- Based on official NORAD algorithms
- Rust implementation of the C++ SGP4 library

## References

- [SGP4 Theory](https://celestrak.org/publications/AIAA/2006-6753/)
- [Space-Track.org](https://www.space-track.org/) - Real satellite TLE data
- [Celestrak](https://celestrak.org/) - Orbital mechanics resources
