#![cfg(feature = "cuda")]

use std::sync::Mutex;

use lzvm_accel::{
    cuda_copy_site_stats_clear, cuda_copy_site_stats_snapshot, CudaCopyDirection, CudaDeviceBuffer,
};

const COPY_SITE_ENV: &str = "LZVM_CUDA_COPY_SITE_STATS";

static COPY_SITE_ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_enabled() -> Self {
        let previous = std::env::var_os(COPY_SITE_ENV);
        std::env::set_var(COPY_SITE_ENV, "1");
        Self { previous }
    }

    fn unset() -> Self {
        let previous = std::env::var_os(COPY_SITE_ENV);
        std::env::remove_var(COPY_SITE_ENV);
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(COPY_SITE_ENV, value),
            None => std::env::remove_var(COPY_SITE_ENV),
        }
        cuda_copy_site_stats_clear();
    }
}

fn upload_test_bytes(buffer: &mut CudaDeviceBuffer) {
    buffer
        .copy_from(&[7_u8; 64])
        .expect("test H2D upload should succeed");
}

fn download_test_bytes(buffer: &CudaDeviceBuffer, output: &mut [u8]) {
    buffer
        .copy_to(output)
        .expect("test D2H download should succeed");
}

#[test]
fn cuda_copy_site_stats_records_enabled_h2d_upload_callers() {
    let _guard = COPY_SITE_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env_guard = EnvGuard::set_enabled();
    cuda_copy_site_stats_clear();

    let mut buffer = CudaDeviceBuffer::new(64).expect("device buffer should allocate");
    upload_test_bytes(&mut buffer);
    upload_test_bytes(&mut buffer);

    let stats = cuda_copy_site_stats_snapshot();
    assert_eq!(stats.len(), 1, "same call site should aggregate: {stats:?}");
    let stat = &stats[0];
    assert_eq!(stat.direction, CudaCopyDirection::H2d);
    assert_eq!(stat.label, "copy_from");
    assert_eq!(stat.calls, 2);
    assert_eq!(stat.bytes, 128);
    assert_eq!(stat.max_bytes, 64);
    assert!(
        stat.wait_ns > 0,
        "copy-site stat should record host API wait time: {stat:?}"
    );
    assert!(
        stat.max_wait_ns > 0,
        "copy-site stat should record max host API wait time: {stat:?}"
    );
    assert!(
        stat.wait_ns >= stat.max_wait_ns,
        "total wait should dominate max wait: {stat:?}"
    );
    assert!(stat.line > 0);
    assert!(
        stat.file.ends_with("cuda_copy_site_stats.rs"),
        "caller file should identify the Rust upload site: {stat:?}"
    );
}

#[test]
fn cuda_copy_site_stats_records_enabled_d2h_download_callers() {
    let _guard = COPY_SITE_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env_guard = EnvGuard::set_enabled();
    cuda_copy_site_stats_clear();

    let mut buffer = CudaDeviceBuffer::new(64).expect("device buffer should allocate");
    upload_test_bytes(&mut buffer);
    cuda_copy_site_stats_clear();

    let mut output = vec![0_u8; 64];
    download_test_bytes(&buffer, &mut output);
    download_test_bytes(&buffer, &mut output);

    let stats = cuda_copy_site_stats_snapshot();
    assert_eq!(stats.len(), 1, "same call site should aggregate: {stats:?}");
    let stat = &stats[0];
    assert_eq!(stat.direction, CudaCopyDirection::D2h);
    assert_eq!(stat.label, "copy_to");
    assert_eq!(stat.calls, 2);
    assert_eq!(stat.bytes, 128);
    assert_eq!(stat.max_bytes, 64);
    assert!(
        stat.wait_ns > 0,
        "copy-site stat should record host API wait time: {stat:?}"
    );
    assert!(
        stat.max_wait_ns > 0,
        "copy-site stat should record max host API wait time: {stat:?}"
    );
    assert!(
        stat.wait_ns >= stat.max_wait_ns,
        "total wait should dominate max wait: {stat:?}"
    );
    assert!(stat.line > 0);
    assert!(
        stat.file.ends_with("cuda_copy_site_stats.rs"),
        "caller file should identify the Rust download site: {stat:?}"
    );
}

#[test]
fn cuda_copy_site_stats_default_disabled() {
    let _guard = COPY_SITE_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env_guard = EnvGuard::unset();
    cuda_copy_site_stats_clear();

    let mut buffer = CudaDeviceBuffer::new(64).expect("device buffer should allocate");
    upload_test_bytes(&mut buffer);

    assert!(
        cuda_copy_site_stats_snapshot().is_empty(),
        "copy-site stats should not record unless explicitly enabled"
    );
}
