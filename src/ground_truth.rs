use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub ospa: f64,  // OSPA distance metric
}

impl EvaluationMetrics {
    pub fn new() -> Self {
        Self {
            true_positives: 0,
            false_positives: 0,
            false_negatives: 0,
            ospa: 0.0,
        }
    }

    pub fn precision(&self) -> f64 {
        let total = self.true_positives + self.false_positives;
        if total == 0 {
            0.0
        } else {
            self.true_positives as f64 / total as f64
        }
    }

    pub fn recall(&self) -> f64 {
        let total = self.true_positives + self.false_negatives;
        if total == 0 {
            0.0
        } else {
            self.true_positives as f64 / total as f64
        }
    }

    pub fn f1(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

pub struct GroundTruthTable {
    entries: HashMap<usize, usize>,
}

impl GroundTruthTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn record_observations(
        &mut self,
        object_indices: &[usize],
        objects: &crate::objects::ObjectPool,
    ) {
        for (i, &obj_idx) in object_indices.iter().enumerate() {
            self.entries.insert(i, objects.get_id(obj_idx));
        }
    }

    pub fn get_object_id(&self, observation_index: usize) -> Option<usize> {
        self.entries.get(&observation_index).copied()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
