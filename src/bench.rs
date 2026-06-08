
pub struct BenchmarkHarness {
    step_times: Vec<u128>,
}

impl BenchmarkHarness {
    pub fn new() -> Self {
        Self {
            step_times: Vec::new(),
        }
    }

    pub fn record_step(&mut self, duration_ms: u128) {
        self.step_times.push(duration_ms);
    }

    pub fn report(&self) -> BenchmarkReport {
        if self.step_times.is_empty() {
            return BenchmarkReport::default();
        }

        let n = self.step_times.len();
        let sum: u128 = self.step_times.iter().sum();
        let mean = sum / n as u128;

        let mut sorted = self.step_times.clone();
        sorted.sort();
        let p95 = sorted[(n * 95) / 100];
        let p99 = sorted[(n * 99) / 100];

        BenchmarkReport {
            total_steps: n,
            mean_step_time_ms: mean,
            p95_step_time_ms: p95,
            p99_step_time_ms: p99,
        }
    }
}

#[derive(Debug, Default, serde::Serialize)]
pub struct BenchmarkReport {
    pub total_steps: usize,
    pub mean_step_time_ms: u128,
    pub p95_step_time_ms: u128,
    pub p99_step_time_ms: u128,
}
