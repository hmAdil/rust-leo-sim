use crate::objects::ObjectPool;
use rayon::prelude::*;
use std::collections::HashMap;

const GRID_CELL_SIZE: f64 = 500.0;

pub struct SpatialIndex {
    grid: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialIndex {
    pub fn new() -> Self {
        Self {
            grid: HashMap::new(),
        }
    }

    pub fn rebuild(&mut self, objects: &ObjectPool) {
        self.grid.clear();
        let cells: Vec<((i32, i32, i32), usize)> = objects
            .pos
            .par_iter()
            .enumerate()
            .map(|(idx, pos)| (Self::position_to_cell(pos), idx))
            .collect();

        for (cell, idx) in cells {
            self.grid.entry(cell).or_default().push(idx);
        }
    }

    fn position_to_cell(pos: &[f64; 3]) -> (i32, i32, i32) {
        (
            (pos[0] / GRID_CELL_SIZE).floor() as i32,
            (pos[1] / GRID_CELL_SIZE).floor() as i32,
            (pos[2] / GRID_CELL_SIZE).floor() as i32,
        )
    }

    pub fn query_nearby(&self, sensor_pos: &[f64; 3], _fov_half_angle: f64) -> Vec<usize> {
        let max_dist = 3000.0;
        let cells_to_search = (max_dist / GRID_CELL_SIZE).ceil() as i32;
        let sensor_cell = Self::position_to_cell(sensor_pos);
        let mut candidates = Vec::new();

        for dx in -cells_to_search..=cells_to_search {
            for dy in -cells_to_search..=cells_to_search {
                for dz in -cells_to_search..=cells_to_search {
                    let cell = (
                        sensor_cell.0 + dx,
                        sensor_cell.1 + dy,
                        sensor_cell.2 + dz,
                    );
                    if let Some(indices) = self.grid.get(&cell) {
                        candidates.extend(indices);
                    }
                }
            }
        }

        candidates
    }
}