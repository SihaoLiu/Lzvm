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

    #[cfg(feature = "cuda")]
    pub(super) fn is_enabled(&self) -> bool {
        self.enabled
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

pub(super) fn write_timing_summary_with_allocator(
    stdout: &mut dyn Write,
    timings: &mut TimingRecorder,
) {
    record_cuda_allocator_timing(timings);
    write_timing_summary(stdout, timings);
}

#[cfg(feature = "cuda")]
fn record_cuda_allocator_timing(timings: &mut TimingRecorder) {
    if !timings.is_enabled() {
        return;
    }
    let Ok(stats) = lzvm_prover::cuda_allocator_stats() else {
        return;
    };
    timings.record_count("cuda_allocator_malloc_calls", stats.cuda_malloc_calls);
    timings.record_count("cuda_allocator_malloc_bytes", stats.cuda_malloc_bytes);
    timings.record_count("cuda_allocator_malloc_wait_ns", stats.cuda_malloc_wait_ns);
    timings.record_count(
        "cuda_allocator_malloc_max_wait_ns",
        stats.cuda_malloc_max_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_host_register_calls",
        stats.cuda_host_register_calls,
    );
    timings.record_count(
        "cuda_allocator_host_register_bytes",
        stats.cuda_host_register_bytes,
    );
    timings.record_count(
        "cuda_allocator_host_register_wait_ns",
        stats.cuda_host_register_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_host_register_max_wait_ns",
        stats.cuda_host_register_max_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_host_unregister_calls",
        stats.cuda_host_unregister_calls,
    );
    timings.record_count(
        "cuda_allocator_host_unregister_wait_ns",
        stats.cuda_host_unregister_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_host_unregister_max_wait_ns",
        stats.cuda_host_unregister_max_wait_ns,
    );
    timings.record_count("cuda_allocator_copy_h2d_calls", stats.cuda_copy_h2d_calls);
    timings.record_count("cuda_allocator_copy_h2d_bytes", stats.cuda_copy_h2d_bytes);
    timings.record_count(
        "cuda_allocator_copy_h2d_wait_ns",
        stats.cuda_copy_h2d_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_copy_h2d_max_wait_ns",
        stats.cuda_copy_h2d_max_wait_ns,
    );
    timings.record_count("cuda_allocator_copy_d2h_calls", stats.cuda_copy_d2h_calls);
    timings.record_count("cuda_allocator_copy_d2h_bytes", stats.cuda_copy_d2h_bytes);
    timings.record_count(
        "cuda_allocator_copy_d2h_wait_ns",
        stats.cuda_copy_d2h_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_copy_d2h_max_wait_ns",
        stats.cuda_copy_d2h_max_wait_ns,
    );
    timings.record_count("cuda_allocator_copy_d2d_calls", stats.cuda_copy_d2d_calls);
    timings.record_count("cuda_allocator_copy_d2d_bytes", stats.cuda_copy_d2d_bytes);
    timings.record_count(
        "cuda_allocator_copy_d2d_wait_ns",
        stats.cuda_copy_d2d_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_copy_d2d_max_wait_ns",
        stats.cuda_copy_d2d_max_wait_ns,
    );
    timings.record_count("cuda_allocator_cached_blocks", stats.cached_blocks);
    timings.record_count("cuda_allocator_cached_bytes", stats.cached_bytes);
    timings.record_count(
        "cuda_allocator_event_query_calls",
        stats.cuda_event_query_calls,
    );
    timings.record_count(
        "cuda_allocator_event_query_ready",
        stats.cuda_event_query_ready_count,
    );
    timings.record_count(
        "cuda_allocator_event_query_not_ready",
        stats.cuda_event_query_not_ready_count,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_calls",
        stats.cuda_event_synchronize_calls,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_bytes",
        stats.cuda_event_synchronize_bytes,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_max_bytes",
        stats.cuda_event_synchronize_max_bytes,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_wait_ns",
        stats.cuda_event_synchronize_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_max_wait_ns",
        stats.cuda_event_synchronize_max_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_hot_bytes",
        stats.cuda_event_synchronize_hot_bytes,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_hot_count",
        stats.cuda_event_synchronize_hot_count,
    );
    timings.record_count(
        "cuda_allocator_event_synchronize_hot_wait_ns",
        stats.cuda_event_synchronize_hot_wait_ns,
    );
    timings.record_count(
        "cuda_allocator_cached_reuse_count",
        stats.cached_reuse_count,
    );
    timings.record_count(
        "cuda_allocator_pending_reuse_count",
        stats.pending_reuse_count,
    );
    timings.record_count(
        "cuda_allocator_no_wait_bypass_count",
        stats.no_wait_bypass_count,
    );
    timings.record_count(
        "cuda_allocator_no_wait_bypass_bytes",
        stats.no_wait_bypass_bytes,
    );
}

#[cfg(not(feature = "cuda"))]
fn record_cuda_allocator_timing(_timings: &mut TimingRecorder) {}

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
