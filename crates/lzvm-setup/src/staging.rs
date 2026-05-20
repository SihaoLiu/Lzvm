use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::SetupError;

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn staging_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "fixed-columns".into());
    let id = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    name.push(format!(".staging.{}.{}", std::process::id(), id));
    path.with_file_name(name)
}

pub(crate) fn write_staging_bytes(
    path: &Path,
    bytes: &[u8],
    role: &'static str,
) -> Result<PathBuf, SetupError> {
    let parent = path.parent().ok_or_else(|| SetupError::MissingParent {
        path: path.to_path_buf(),
    })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let staging_path = staging_path_for(path);
    std::fs::write(&staging_path, bytes).map_err(|error| SetupError::Io {
        role,
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    Ok(staging_path)
}

pub(crate) fn publish_staging_bytes(
    staging_path: &Path,
    output_path: &Path,
    role: &'static str,
) -> Result<u64, SetupError> {
    let bytes_written = std::fs::metadata(staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.to_path_buf(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(staging_path, output_path).map_err(|error| SetupError::Io {
        role,
        path: output_path.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(bytes_written)
}
