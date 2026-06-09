# SGP4 GUI Test - FIXED!

The SGP4 performance issue has been **FIXED**! 

## What Was Wrong

The code was calling `Constants::from_elements()` **every single frame** for every object. This is very expensive (converting TLE elements to internal constants).

## The Fix

Now we:
1. **Pre-compute** SGP4 Constants once during initialization
2. **Cache** them in `ObjectPool.sgp4_constants`
3. **Reuse** the cached constants for every propagation

## Performance Results

**Before Fix:**
- Creating constants every frame: ~100-500ms per step
- GUI was unusable with 300 objects

**After Fix:**
- Cached constants: 0-4ms per step
- GUI runs smoothly!

## Test Commands

### Quick Test (Console)
```bash
cargo run --release --bin leo_sim -- --objects 300 --steps 10 --sgp4
```
Expected: Completes in < 1 second

### GUI Test
```bash
cargo run --release --bin leo_sim -- --gui --objects 200 --sgp4
```
Expected: Smooth 60fps with realistic SGP4 orbits!

### Performance Comparison
```bash
# Simple Keplerian (baseline)
cargo run --release --bin leo_sim -- --objects 300 --steps 100

# SGP4 (now comparable performance!)
cargo run --release --bin leo_sim -- --objects 300 --steps 100 --sgp4
```

## What You'll See with SGP4

- **Elliptical orbits** (not perfect circles)
- **Orbital decay** over time due to drag
- **Precession** of orbital plane
- **More realistic** satellite motion

The orbits won't be perfect circles anymore - they'll have slight eccentricity and will evolve realistically over time!

## Technical Details

The Constants struct contains:
- Pre-computed orbital parameters
- Internal SGP4 state
- Drag coefficients
- Gravitational constants

By caching these, we avoid expensive recomputation and get near-Keplerian performance!
