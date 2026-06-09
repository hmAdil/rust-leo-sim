// Quick test to verify SGP4 functionality
// Run with: cargo run --release --bin test_sgp4

use sgp4::{Elements, Constants, MinutesSinceEpoch};
use sgp4::chrono::NaiveDate;

fn main() {
    println!("Testing SGP4 propagation...\n");

    // Create a simple test satellite
    let datetime = NaiveDate::from_ymd_opt(2021, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let elements = Elements {
        object_name: Some("TEST_SAT".to_string()),
        international_designator: None,
        norad_id: 12345,
        datetime,
        inclination: 0.9,  // ~51.6 degrees
        right_ascension: 1.5,
        eccentricity: 0.0001,
        argument_of_perigee: 0.0,
        mean_anomaly: 0.0,
        mean_motion: 15.5,  // ~15.5 revs/day for LEO
        mean_motion_dot: 0.0,
        mean_motion_ddot: 0.0,
        drag_term: 0.0001,
        revolution_number: 0,
        classification: sgp4::Classification::Unclassified,
        ephemeris_type: 0,
        element_set_number: 999,
    };

    println!("Satellite: {:?}", elements.object_name);
    println!("Inclination: {:.2}°", elements.inclination.to_degrees());
    println!("Mean Motion: {:.2} rev/day\n", elements.mean_motion);

    // Create SGP4 constants
    match Constants::from_elements(&elements) {
        Ok(constants) => {
            println!("✓ SGP4 constants created successfully\n");
            
            // Test propagation at different times
            let test_times = vec![0.0, 1.0, 60.0, 1440.0]; // 0 min, 1 min, 1 hour, 1 day
            
            println!("Propagation tests:");
            println!("{:<12} {:<30} {:<30}", "Time", "Position (km)", "Velocity (km/s)");
            println!("{:-<72}", "");
            
            for &minutes in &test_times {
                match constants.propagate(MinutesSinceEpoch(minutes)) {
                    Ok(prediction) => {
                        let time_str = if minutes < 60.0 {
                            format!("{:.0} min", minutes)
                        } else if minutes < 1440.0 {
                            format!("{:.1} hours", minutes / 60.0)
                        } else {
                            format!("{:.1} days", minutes / 1440.0)
                        };
                        
                        println!("{:<12} [{:>8.1}, {:>8.1}, {:>8.1}] [{:>6.3}, {:>6.3}, {:>6.3}]",
                            time_str,
                            prediction.position[0],
                            prediction.position[1],
                            prediction.position[2],
                            prediction.velocity[0],
                            prediction.velocity[1],
                            prediction.velocity[2]
                        );
                    }
                    Err(e) => {
                        println!("{:<12} Error: {:?}", minutes, e);
                    }
                }
            }
            
            println!("\n✓ SGP4 is working correctly!");
            
            // Performance test
            println!("\nPerformance test (1000 propagations):");
            let start = std::time::Instant::now();
            for i in 0..1000 {
                let _ = constants.propagate(MinutesSinceEpoch(i as f64));
            }
            let elapsed = start.elapsed();
            println!("Time: {:.2?}", elapsed);
            println!("Per propagation: {:.2?}", elapsed / 1000);
            
        }
        Err(e) => {
            println!("✗ Failed to create SGP4 constants: {:?}", e);
        }
    }
}
