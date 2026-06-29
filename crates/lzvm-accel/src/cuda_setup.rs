use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};

use super::{AccelError, ROOTS_OF_UNITY};

unsafe extern "C" {
    fn lzvm_cuda_current_device(out: *mut i32) -> i32;
    #[cfg(test)]
    fn lzvm_cuda_setup_root_limit(out: *mut u32) -> i32;
    fn lzvm_cuda_setup_init(roots: *const u64, root_count: usize, max_bits_ext: usize) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CudaSetupCacheEntry {
    device: i32,
    max_bits_ext: usize,
}

static CUDA_SETUP_CACHE: Mutex<Vec<CudaSetupCacheEntry>> = Mutex::new(Vec::new());
static CUDA_SETUP_STATS: CudaSetupStatsCounters = CudaSetupStatsCounters::new();

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CudaSetupStats {
    pub setup_init_calls: usize,
    pub setup_init_wait_ns: usize,
    pub setup_init_max_wait_ns: usize,
    pub setup_cache_hits: usize,
    pub setup_cache_hit_wait_ns: usize,
    pub setup_cache_hit_max_wait_ns: usize,
    pub setup_native_init_calls: usize,
    pub setup_native_init_wait_ns: usize,
    pub setup_native_init_max_wait_ns: usize,
    pub current_device_calls: usize,
    pub current_device_wait_ns: usize,
    pub current_device_max_wait_ns: usize,
    pub memory_info_calls: usize,
    pub memory_info_wait_ns: usize,
    pub memory_info_max_wait_ns: usize,
}

struct CudaSetupStatsCounters {
    setup_init_calls: AtomicUsize,
    setup_init_wait_ns: AtomicUsize,
    setup_init_max_wait_ns: AtomicUsize,
    setup_cache_hits: AtomicUsize,
    setup_cache_hit_wait_ns: AtomicUsize,
    setup_cache_hit_max_wait_ns: AtomicUsize,
    setup_native_init_calls: AtomicUsize,
    setup_native_init_wait_ns: AtomicUsize,
    setup_native_init_max_wait_ns: AtomicUsize,
    current_device_calls: AtomicUsize,
    current_device_wait_ns: AtomicUsize,
    current_device_max_wait_ns: AtomicUsize,
    memory_info_calls: AtomicUsize,
    memory_info_wait_ns: AtomicUsize,
    memory_info_max_wait_ns: AtomicUsize,
}

impl CudaSetupStatsCounters {
    const fn new() -> Self {
        Self {
            setup_init_calls: AtomicUsize::new(0),
            setup_init_wait_ns: AtomicUsize::new(0),
            setup_init_max_wait_ns: AtomicUsize::new(0),
            setup_cache_hits: AtomicUsize::new(0),
            setup_cache_hit_wait_ns: AtomicUsize::new(0),
            setup_cache_hit_max_wait_ns: AtomicUsize::new(0),
            setup_native_init_calls: AtomicUsize::new(0),
            setup_native_init_wait_ns: AtomicUsize::new(0),
            setup_native_init_max_wait_ns: AtomicUsize::new(0),
            current_device_calls: AtomicUsize::new(0),
            current_device_wait_ns: AtomicUsize::new(0),
            current_device_max_wait_ns: AtomicUsize::new(0),
            memory_info_calls: AtomicUsize::new(0),
            memory_info_wait_ns: AtomicUsize::new(0),
            memory_info_max_wait_ns: AtomicUsize::new(0),
        }
    }

    fn snapshot(&self) -> CudaSetupStats {
        CudaSetupStats {
            setup_init_calls: self.setup_init_calls.load(Ordering::Relaxed),
            setup_init_wait_ns: self.setup_init_wait_ns.load(Ordering::Relaxed),
            setup_init_max_wait_ns: self.setup_init_max_wait_ns.load(Ordering::Relaxed),
            setup_cache_hits: self.setup_cache_hits.load(Ordering::Relaxed),
            setup_cache_hit_wait_ns: self.setup_cache_hit_wait_ns.load(Ordering::Relaxed),
            setup_cache_hit_max_wait_ns: self.setup_cache_hit_max_wait_ns.load(Ordering::Relaxed),
            setup_native_init_calls: self.setup_native_init_calls.load(Ordering::Relaxed),
            setup_native_init_wait_ns: self.setup_native_init_wait_ns.load(Ordering::Relaxed),
            setup_native_init_max_wait_ns: self
                .setup_native_init_max_wait_ns
                .load(Ordering::Relaxed),
            current_device_calls: self.current_device_calls.load(Ordering::Relaxed),
            current_device_wait_ns: self.current_device_wait_ns.load(Ordering::Relaxed),
            current_device_max_wait_ns: self.current_device_max_wait_ns.load(Ordering::Relaxed),
            memory_info_calls: self.memory_info_calls.load(Ordering::Relaxed),
            memory_info_wait_ns: self.memory_info_wait_ns.load(Ordering::Relaxed),
            memory_info_max_wait_ns: self.memory_info_max_wait_ns.load(Ordering::Relaxed),
        }
    }
}

pub fn cuda_setup_init(max_bits_ext: usize) -> Result<(), AccelError> {
    let setup_started = Instant::now();
    let result = cuda_setup_init_inner(max_bits_ext, setup_started);
    record_wait_stats(
        &CUDA_SETUP_STATS.setup_init_calls,
        &CUDA_SETUP_STATS.setup_init_wait_ns,
        &CUDA_SETUP_STATS.setup_init_max_wait_ns,
        setup_started.elapsed(),
    );
    result
}

fn cuda_setup_init_inner(max_bits_ext: usize, setup_started: Instant) -> Result<(), AccelError> {
    validate_cuda_setup_domain(max_bits_ext)?;
    let device = current_cuda_device()?;

    let mut cache = CUDA_SETUP_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if cuda_setup_cache_covers(&cache, device, max_bits_ext) {
        record_wait_stats(
            &CUDA_SETUP_STATS.setup_cache_hits,
            &CUDA_SETUP_STATS.setup_cache_hit_wait_ns,
            &CUDA_SETUP_STATS.setup_cache_hit_max_wait_ns,
            setup_started.elapsed(),
        );
        return Ok(());
    }

    let native_started = Instant::now();
    let code = unsafe {
        lzvm_cuda_setup_init(ROOTS_OF_UNITY.as_ptr(), ROOTS_OF_UNITY.len(), max_bits_ext)
    };
    record_wait_stats(
        &CUDA_SETUP_STATS.setup_native_init_calls,
        &CUDA_SETUP_STATS.setup_native_init_wait_ns,
        &CUDA_SETUP_STATS.setup_native_init_max_wait_ns,
        native_started.elapsed(),
    );
    if code == 0 {
        record_cuda_setup_cache(&mut cache, device, max_bits_ext);
        Ok(())
    } else {
        Err(AccelError::Cuda { code })
    }
}

pub fn cuda_setup_stats() -> CudaSetupStats {
    CUDA_SETUP_STATS.snapshot()
}

pub(crate) fn record_cuda_memory_info_duration(duration: Duration) {
    record_wait_stats(
        &CUDA_SETUP_STATS.memory_info_calls,
        &CUDA_SETUP_STATS.memory_info_wait_ns,
        &CUDA_SETUP_STATS.memory_info_max_wait_ns,
        duration,
    );
}

pub(crate) fn record_cuda_current_device_duration(duration: Duration) {
    record_wait_stats(
        &CUDA_SETUP_STATS.current_device_calls,
        &CUDA_SETUP_STATS.current_device_wait_ns,
        &CUDA_SETUP_STATS.current_device_max_wait_ns,
        duration,
    );
}

fn current_cuda_device() -> Result<i32, AccelError> {
    let started = Instant::now();
    let mut device = 0_i32;
    let code = unsafe { lzvm_cuda_current_device(&mut device) };
    record_cuda_current_device_duration(started.elapsed());
    if code == 0 {
        Ok(device)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(test)]
fn cuda_setup_root_limit() -> Result<u32, AccelError> {
    let mut root_limit = 0_u32;
    let code = unsafe { lzvm_cuda_setup_root_limit(&mut root_limit) };
    if code == 0 {
        Ok(root_limit)
    } else {
        Err(AccelError::Cuda { code })
    }
}

fn validate_cuda_setup_domain(max_bits_ext: usize) -> Result<(), AccelError> {
    if ROOTS_OF_UNITY.get(max_bits_ext).is_none() {
        Err(AccelError::InvalidDomain {
            bits: max_bits_ext,
            len: ROOTS_OF_UNITY.len(),
        })
    } else {
        Ok(())
    }
}

fn cuda_setup_cache_covers(
    cache: &[CudaSetupCacheEntry],
    device: i32,
    max_bits_ext: usize,
) -> bool {
    cache
        .iter()
        .any(|entry| entry.device == device && entry.max_bits_ext >= max_bits_ext)
}

fn record_cuda_setup_cache(cache: &mut Vec<CudaSetupCacheEntry>, device: i32, max_bits_ext: usize) {
    if let Some(entry) = cache.iter_mut().find(|entry| entry.device == device) {
        entry.max_bits_ext = entry.max_bits_ext.max(max_bits_ext);
    } else {
        cache.push(CudaSetupCacheEntry {
            device,
            max_bits_ext,
        });
    }
}

fn record_wait_stats(
    call_count: &AtomicUsize,
    wait_ns: &AtomicUsize,
    max_wait_ns: &AtomicUsize,
    duration: Duration,
) {
    let elapsed_ns = duration_ns(duration);
    saturating_add_atomic(call_count, 1);
    saturating_add_atomic(wait_ns, elapsed_ns);
    max_atomic(max_wait_ns, elapsed_ns);
}

fn duration_ns(duration: Duration) -> usize {
    duration.as_nanos().min(usize::MAX as u128) as usize
}

fn saturating_add_atomic(value: &AtomicUsize, increment: usize) {
    let _ = value.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_add(increment))
    });
}

fn max_atomic(value: &AtomicUsize, candidate: usize) {
    let _ = value.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        (candidate > current).then_some(candidate)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuda_setup_cache_is_keyed_by_device() {
        let mut cache = Vec::<CudaSetupCacheEntry>::new();

        record_cuda_setup_cache(&mut cache, 0, 4);

        assert!(cuda_setup_cache_covers(&cache, 0, 3));
        assert!(!cuda_setup_cache_covers(&cache, 1, 3));

        record_cuda_setup_cache(&mut cache, 1, 2);

        assert!(cuda_setup_cache_covers(&cache, 1, 2));
        assert!(!cuda_setup_cache_covers(&cache, 1, 3));
    }

    #[test]
    fn cuda_setup_stats_helpers_saturate_and_track_max() {
        let calls = AtomicUsize::new(usize::MAX);
        let wait_ns = AtomicUsize::new(usize::MAX - 3);
        let max_wait_ns = AtomicUsize::new(9);

        record_wait_stats(&calls, &wait_ns, &max_wait_ns, Duration::from_nanos(7));

        assert_eq!(calls.load(Ordering::Relaxed), usize::MAX);
        assert_eq!(wait_ns.load(Ordering::Relaxed), usize::MAX);
        assert_eq!(max_wait_ns.load(Ordering::Relaxed), 9);

        record_wait_stats(
            &AtomicUsize::new(0),
            &AtomicUsize::new(0),
            &max_wait_ns,
            Duration::from_nanos(11),
        );

        assert_eq!(max_wait_ns.load(Ordering::Relaxed), 11);
        assert_eq!(duration_ns(Duration::from_secs(u64::MAX)), usize::MAX);
    }

    #[test]
    fn cuda_setup_cache_skips_smaller_native_reinitialization() {
        cuda_setup_init(4).expect("larger setup should initialize");
        assert_eq!(
            cuda_setup_root_limit().expect("root limit should be readable"),
            4
        );

        cuda_setup_init(3).expect("smaller setup should reuse initialized constants");

        assert_eq!(
            cuda_setup_root_limit().expect("root limit should be readable"),
            4
        );
    }
}
