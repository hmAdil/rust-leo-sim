use crate::gui::state::{ConfigScreenState, DataSource};

/// Validate the configuration and return an error message if invalid
pub fn validate_config(state: &ConfigScreenState) -> Result<(), String> {
    let config = &state.config;
    
    // Range constraints
    if config.n_objects < 1 || config.n_objects > 100_000 {
        return Err(format!("n_objects must be between 1 and 100,000, got {}", config.n_objects));
    }
    
    if config.n_sensors < 1 || config.n_sensors > 100 {
        return Err(format!("n_sensors must be between 1 and 100, got {}", config.n_sensors));
    }
    
    if config.dt < 0.1 || config.dt > 1000.0 {
        return Err(format!("dt must be between 0.1 and 1000.0 seconds, got {}", config.dt));
    }
    
    if config.steps == 0 {
        return Err("steps must be at least 1".to_string());
    }
    
    // Logical constraints
    if config.n_sensors > config.n_objects {
        return Err(format!(
            "n_sensors ({}) cannot exceed n_objects ({})",
            config.n_sensors, config.n_objects
        ));
    }
    
    if config.collision_threshold_km > 1000.0 {
        return Err(format!(
            "collision_threshold_km cannot exceed 1000.0 km, got {}",
            config.collision_threshold_km
        ));
    }
    
    // Data source constraints
    if state.data_source == DataSource::ImportCSV && state.csv_path.is_none() {
        return Err("Please select a CSV file for import".to_string());
    }
    
    Ok(())
}