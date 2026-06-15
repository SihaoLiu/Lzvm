use std::collections::BTreeMap;
use std::panic::Location;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const COPY_SITE_STATS_ENV: &str = "LZVM_CUDA_COPY_SITE_STATS";
const ENABLED_UNKNOWN: u8 = 0;
const ENABLED_FALSE: u8 = 1;
const ENABLED_TRUE: u8 = 2;

static COPY_SITE_STATS_ENABLED: AtomicU8 = AtomicU8::new(ENABLED_UNKNOWN);
static COPY_SITE_STATS: OnceLock<Mutex<BTreeMap<CudaCopySiteKey, CudaCopySiteStat>>> =
    OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CudaCopySiteKey {
    label: &'static str,
    file: &'static str,
    line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CudaCopySiteStat {
    pub label: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub calls: usize,
    pub bytes: usize,
    pub max_bytes: usize,
    pub wait_ns: usize,
    pub max_wait_ns: usize,
}

fn copy_site_stats() -> &'static Mutex<BTreeMap<CudaCopySiteKey, CudaCopySiteStat>> {
    COPY_SITE_STATS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn env_flag_enabled(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

fn copy_site_stats_enabled() -> bool {
    match COPY_SITE_STATS_ENABLED.load(Ordering::Acquire) {
        ENABLED_FALSE => false,
        ENABLED_TRUE => true,
        _ => {
            let enabled = std::env::var(COPY_SITE_STATS_ENV)
                .as_deref()
                .is_ok_and(env_flag_enabled);
            COPY_SITE_STATS_ENABLED.store(
                if enabled { ENABLED_TRUE } else { ENABLED_FALSE },
                Ordering::Release,
            );
            enabled
        }
    }
}

fn duration_ns(duration: Duration) -> usize {
    usize::try_from(duration.as_nanos()).unwrap_or(usize::MAX)
}

#[track_caller]
fn record_h2d_copy_site(label: &'static str, bytes: usize, wait_ns: usize) {
    let location = Location::caller();
    let key = CudaCopySiteKey {
        label,
        file: location.file(),
        line: location.line(),
    };
    let mut stats = copy_site_stats()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let entry = stats.entry(key).or_insert_with(|| CudaCopySiteStat {
        label,
        file: location.file(),
        line: location.line(),
        calls: 0,
        bytes: 0,
        max_bytes: 0,
        wait_ns: 0,
        max_wait_ns: 0,
    });
    entry.calls = entry.calls.saturating_add(1);
    entry.bytes = entry.bytes.saturating_add(bytes);
    entry.max_bytes = entry.max_bytes.max(bytes);
    entry.wait_ns = entry.wait_ns.saturating_add(wait_ns);
    entry.max_wait_ns = entry.max_wait_ns.max(wait_ns);
}

#[track_caller]
pub(crate) fn record_h2d_copy_site_timing<T>(
    label: &'static str,
    bytes: usize,
    run: impl FnOnce() -> T,
) -> T {
    if !copy_site_stats_enabled() {
        return run();
    }
    let started = Instant::now();
    let result = run();
    record_h2d_copy_site(label, bytes, duration_ns(started.elapsed()));
    result
}

pub fn cuda_copy_site_stats_snapshot() -> Vec<CudaCopySiteStat> {
    let mut stats = copy_site_stats()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .values()
        .cloned()
        .collect::<Vec<_>>();
    stats.sort_by(|lhs, rhs| {
        rhs.bytes
            .cmp(&lhs.bytes)
            .then_with(|| rhs.calls.cmp(&lhs.calls))
            .then_with(|| lhs.label.cmp(rhs.label))
            .then_with(|| lhs.file.cmp(rhs.file))
            .then_with(|| lhs.line.cmp(&rhs.line))
    });
    stats
}

pub fn cuda_copy_site_stats_clear() {
    COPY_SITE_STATS_ENABLED.store(ENABLED_UNKNOWN, Ordering::Release);
    if let Some(stats) = COPY_SITE_STATS.get() {
        stats
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}
