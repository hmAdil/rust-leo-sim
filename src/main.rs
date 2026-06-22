mod bench;
mod catalog;
pub mod celestrak;  // Public for export tool
mod collision;
mod config;
mod ground_truth;
mod gui;
mod hungarian;
mod jpda;
mod objects;
mod passive;
mod sensor;
mod sim;
mod spatial;
mod tracker;

fn main() {
    gui::run();
}