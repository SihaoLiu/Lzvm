use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct TimingEntry {
    pub(super) name: String,
    pub(super) duration: Duration,
}

pub(super) struct TimingRecorder {
    enabled: bool,
    started: Instant,
    last_mark: Instant,
    entries: Vec<TimingEntry>,
}

impl TimingRecorder {
    pub(super) fn new(enabled: bool) -> Self {
        let now = Instant::now();
        Self {
            enabled,
            started: now,
            last_mark: now,
            entries: Vec::new(),
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

    fn total(&self) -> Duration {
        self.last_mark.duration_since(self.started)
    }
}

pub(super) fn write_timing_summary(stdout: &mut dyn Write, timings: &TimingRecorder) {
    if !timings.enabled {
        return;
    }
    write_timing_entries(stdout, &timings.entries, timings.total());
}

pub(super) fn write_timing_entries(
    stdout: &mut dyn Write,
    entries: &[TimingEntry],
    total: Duration,
) {
    for entry in entries {
        let _ = writeln!(
            stdout,
            "timing_{}_ms={}",
            entry.name.as_str(),
            entry.duration.as_millis()
        );
    }
    let _ = writeln!(stdout, "timing_total_ms={}", total.as_millis());
}
