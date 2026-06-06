use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct TimingEntry {
    pub(super) name: String,
    pub(super) duration: Duration,
}

#[derive(Clone)]
pub(super) struct TimingCountEntry {
    pub(super) name: String,
    pub(super) value: usize,
}

pub(super) struct TimingRecorder {
    enabled: bool,
    started: Instant,
    last_mark: Instant,
    entries: Vec<TimingEntry>,
    count_entries: Vec<TimingCountEntry>,
}

impl TimingRecorder {
    pub(super) fn new(enabled: bool) -> Self {
        let now = Instant::now();
        Self {
            enabled,
            started: now,
            last_mark: now,
            entries: Vec::new(),
            count_entries: Vec::new(),
        }
    }

    pub(super) fn mark(&mut self, name: &'static str) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        self.entries.push(TimingEntry {
            name: name.to_owned(),
            duration: now.duration_since(self.last_mark),
        });
        self.last_mark = now;
    }

    pub(super) fn record(&mut self, name: &'static str, duration: Duration) {
        if !self.enabled {
            return;
        }
        self.record_dynamic(name.to_owned(), duration);
    }

    pub(super) fn record_dynamic(&mut self, name: String, duration: Duration) {
        if !self.enabled {
            return;
        }
        self.entries.push(TimingEntry { name, duration });
    }

    pub(super) fn record_count(&mut self, name: &'static str, value: usize) {
        if !self.enabled {
            return;
        }
        self.record_count_dynamic(name.to_owned(), value);
    }

    pub(super) fn record_count_dynamic(&mut self, name: String, value: usize) {
        if !self.enabled {
            return;
        }
        self.count_entries.push(TimingCountEntry { name, value });
    }

    fn total(&self) -> Duration {
        self.last_mark.duration_since(self.started)
    }
}

pub(super) fn write_timing_summary(stdout: &mut dyn Write, timings: &TimingRecorder) {
    if !timings.enabled {
        return;
    }
    write_timing_entries(
        stdout,
        &timings.entries,
        &timings.count_entries,
        timings.total(),
    );
}

pub(super) fn write_timing_entries(
    stdout: &mut dyn Write,
    entries: &[TimingEntry],
    count_entries: &[TimingCountEntry],
    total: Duration,
) {
    let _ = writeln!(stdout, "prover_gpu_mode={}", prover_gpu_mode());
    for entry in entries {
        let _ = writeln!(
            stdout,
            "timing_{}_ms={}",
            entry.name.as_str(),
            entry.duration.as_millis()
        );
    }
    for entry in count_entries {
        let _ = writeln!(stdout, "timing_{}={}", entry.name.as_str(), entry.value);
    }
    let _ = writeln!(stdout, "timing_total_ms={}", total.as_millis());
}

pub(super) fn prover_gpu_mode() -> &'static str {
    if lzvm_prover::gpu_setup_available() {
        "cuda"
    } else {
        "unavailable"
    }
}
