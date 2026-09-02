#[cfg(feature = "tier-spectra")]
use std::sync::Arc;

#[cfg(feature = "tier-spectra")]
use spectra_core::{RecordingSink, SpectraSink};

/// Forwards to an inner sink while mirroring into a [`RecordingSink`] for per-iteration counts.
#[cfg(feature = "tier-spectra")]
#[derive(Clone)]
pub struct CountingSink {
    pub recording: RecordingSink,
    inner: Arc<dyn SpectraSink>,
}

#[cfg(feature = "tier-spectra")]
impl CountingSink {
    pub fn new(recording: RecordingSink, inner: Arc<dyn SpectraSink>) -> Self {
        Self { recording, inner }
    }

    pub fn recording_only(recording: RecordingSink) -> Self {
        Self {
            recording,
            inner: Arc::new(NoopSink),
        }
    }

    pub fn install(self) {
        spectra_core::set_sink(Arc::new(self));
    }
}

#[cfg(feature = "tier-spectra")]
impl SpectraSink for CountingSink {
    fn record_counter(&self, name: &str, labels: &[(&str, &str)], delta: i64) {
        self.recording.record_counter(name, labels, delta);
        self.inner.record_counter(name, labels, delta);
    }

    fn record_gauge(&self, name: &str, labels: &[(&str, &str)], value: f64) {
        self.recording.record_gauge(name, labels, value);
        self.inner.record_gauge(name, labels, value);
    }

    fn log_event(&self, table: &str, fields: &serde_json::Value) {
        self.recording.log_event(table, fields);
        self.inner.log_event(table, fields);
    }
}

#[cfg(feature = "tier-spectra")]
struct NoopSink;

#[cfg(feature = "tier-spectra")]
impl SpectraSink for NoopSink {
    fn record_counter(&self, _name: &str, _labels: &[(&str, &str)], _delta: i64) {}
    fn record_gauge(&self, _name: &str, _labels: &[(&str, &str)], _value: f64) {}
    fn log_event(&self, _table: &str, _fields: &serde_json::Value) {}
}
