#[cfg(feature = "tier-spectra")]
use anyhow::Result;
#[cfg(feature = "tier-spectra")]
use spectra_core::RecordingSink;

#[cfg(feature = "tier-spectra")]
use crate::stack::counting_sink::CountingSink;
#[cfg(feature = "tier-spectra")]
use crate::stack::BenchRuntime;

#[cfg(feature = "tier-spectra")]
pub async fn boot(runtime: &mut BenchRuntime) -> Result<()> {
    let recording = RecordingSink::new();
    let counting = CountingSink::recording_only(recording.clone());
    counting.install();
    runtime.spectra_recording = Some(recording);
    Ok(())
}

#[cfg(feature = "tier-spectra")]
pub fn spectra_counts(recording: &RecordingSink) -> (u32, u32, u32) {
    let events = recording.events().len() as u32;
    let gauges = recording.gauges().len() as u32;
    let counters = recording.counters().len() as u32;
    (events, gauges, counters)
}

#[cfg(feature = "tier-spectra")]
pub fn clear_recording(recording: &RecordingSink) {
    recording.clear();
}

#[cfg(feature = "tier-spectra")]
pub fn retry_count_from_recording(recording: &RecordingSink) -> (u32, f64) {
    let mut count = 0u32;
    for ev in recording.events() {
        if ev.table != "valence_error_log" {
            continue;
        }
        let category = ev
            .fields
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if category == "retry" {
            count += 1;
        }
    }
    (count, 0.0)
}
