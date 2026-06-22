mod app;
mod benchmark;
mod config_screen;
mod file_io;
mod simulation_screen;
mod state;

pub use app::UnifiedApp;

/// Launch the unified GUI application
pub fn run() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_title("LEO Observatory Network - Space Object Tracking"),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "leo_sim_gui",
        options,
        Box::new(|_| Ok(Box::new(UnifiedApp::new()))),
    );
}