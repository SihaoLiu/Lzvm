use std::sync::Mutex;

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

pub fn cuda_setup_init(max_bits_ext: usize) -> Result<(), AccelError> {
    validate_cuda_setup_domain(max_bits_ext)?;
    let device = current_cuda_device()?;

    let mut cache = CUDA_SETUP_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if cuda_setup_cache_covers(&cache, device, max_bits_ext) {
        return Ok(());
    }

    let code = unsafe {
        lzvm_cuda_setup_init(ROOTS_OF_UNITY.as_ptr(), ROOTS_OF_UNITY.len(), max_bits_ext)
    };
    if code == 0 {
        record_cuda_setup_cache(&mut cache, device, max_bits_ext);
        Ok(())
    } else {
        Err(AccelError::Cuda { code })
    }
}

fn current_cuda_device() -> Result<i32, AccelError> {
    let mut device = 0_i32;
    let code = unsafe { lzvm_cuda_current_device(&mut device) };
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
