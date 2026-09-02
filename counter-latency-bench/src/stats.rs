use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub min: f64,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
    pub count: usize,
}

impl Stats {
    pub fn empty() -> Self {
        Self {
            min: 0.0,
            p50: 0.0,
            p95: 0.0,
            max: 0.0,
            count: 0,
        }
    }

    pub fn summarize(mut samples: Vec<f64>) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let count = samples.len();
        let min = samples[0];
        let max = samples[count - 1];
        let p50 = percentile(&samples, 0.50);
        let p95 = percentile(&samples, 0.95);
        Self {
            min,
            p50,
            p95,
            max,
            count,
        }
    }

    pub fn median(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 * p).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[idx]
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count == 0 {
            write!(f, "min=— p50=— p95=— max=— (n=0)")
        } else {
            write!(
                f,
                "min={:.1} p50={:.1} p95={:.1} max={:.1} (n={})",
                self.min, self.p50, self.p95, self.max, self.count
            )
        }
    }
}

pub struct MetricReport {
    pub labels: Vec<(String, Stats)>,
}

impl MetricReport {
    pub fn new() -> Self {
        Self { labels: Vec::new() }
    }

    pub fn push(&mut self, label: impl Into<String>, samples: Vec<f64>) {
        self.labels.push((label.into(), Stats::summarize(samples)));
    }

    pub fn print_summary(&self, header: &str) {
        println!("[counter-latency-bench] {header}");
        for (label, stats) in &self.labels {
            println!("[counter-latency-bench]   {label}: {stats}");
        }
    }
}

impl Default for MetricReport {
    fn default() -> Self {
        Self::new()
    }
}
