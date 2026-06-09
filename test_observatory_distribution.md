# Observatory Distribution Test

Run the GUI to verify observatory distribution:

```bash
cargo run --release -- --gui
```

## Expected Result

You should now see **24 observatories** evenly distributed across the entire 3D globe:

### Distribution Pattern
- **Sunflower seed arrangement** - optimal sphere packing
- Coverage in **both hemispheres** (north and south)
- **360° longitude coverage** (all around the globe)
- **-90° to +90° latitude coverage** (from south pole to north pole)

### Visual Indicators
- 🟠 Orange circles with yellow outlines
- Spread across visible and hidden sides of Earth
- Rotate the view to confirm global distribution
- No clustering in any hemisphere

## Quick Verification

1. **Launch GUI**: `cargo run --release -- --gui`
2. **Rotate the globe**: Drag to rotate, you should see observatories everywhere
3. **Check detection log**: Multiple observatories (OBS_00 through OBS_23) should detect objects
4. **Watch detections**: Objects detected by different combinations of observatories

## Distribution Details

With 24 sensors:
- Approximately 12 in northern hemisphere
- Approximately 12 in southern hemisphere  
- Golden ratio spiral ensures even spacing
- No polar clustering (offset by 0.5)

## Test Command

To test with even more sensors:
```bash
cargo run --release -- --gui --sensors 32
```

This will give you 32 observatories for even denser global coverage!
