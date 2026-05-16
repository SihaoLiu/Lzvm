use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache, ProgramImageGpuMode,
};

use crate::prove_plan::format_hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProgramImageCacheSummary {
    pub path: PathBuf,
    pub cache: ProgramImageCommitmentCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgramImageCacheSummaryError {
    Cache {
        path: PathBuf,
        source: lzvm_artifacts::program_image::ProgramImageCommitmentCacheError,
    },
    GuestImageDigestMismatch {
        path: PathBuf,
    },
}

impl std::fmt::Display for ProgramImageCacheSummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cache { path, source } => write!(
                f,
                "program image cache failed at {}: {source}",
                path.display()
            ),
            Self::GuestImageDigestMismatch { path } => write!(
                f,
                "program image cache guest image digest mismatch at {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProgramImageCacheSummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cache { source, .. } => Some(source),
            Self::GuestImageDigestMismatch { .. } => None,
        }
    }
}

pub(crate) fn read_requested_program_image_cache_summary(
    path: Option<&Path>,
    guest_image_digest: [u8; 32],
) -> Result<Option<ProgramImageCacheSummary>, ProgramImageCacheSummaryError> {
    path.map(|path| read_program_image_cache_summary(path, guest_image_digest))
        .transpose()
}

pub(crate) fn read_program_image_cache_summary(
    path: impl AsRef<Path>,
    guest_image_digest: [u8; 32],
) -> Result<ProgramImageCacheSummary, ProgramImageCacheSummaryError> {
    let path = path.as_ref().to_path_buf();
    let cache = read_program_image_commitment_cache_file(&path).map_err(|source| {
        ProgramImageCacheSummaryError::Cache {
            path: path.clone(),
            source,
        }
    })?;
    if cache.source_image_digest != guest_image_digest {
        return Err(ProgramImageCacheSummaryError::GuestImageDigestMismatch { path });
    }
    Ok(ProgramImageCacheSummary { path, cache })
}

pub(crate) fn write_program_image_cache_summary(
    stdout: &mut dyn Write,
    summary: &ProgramImageCacheSummary,
) {
    let _ = writeln!(stdout, "program_image_cache={}", summary.path.display());
    let _ = writeln!(
        stdout,
        "program_image_cache_program_digest={}",
        format_hash(&summary.cache.program_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_source_image_digest={}",
        format_hash(&summary.cache.source_image_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_trace_rows={}",
        summary.cache.trace_row_count
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_trace_columns={}",
        summary.cache.trace_column_count
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_blowup_factor={}",
        summary.cache.blowup_factor
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_arity={}",
        summary.cache.merkle_tree_arity
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_gpu_mode={}",
        format_program_image_gpu_mode(summary.cache.gpu_mode)
    );
}

fn format_program_image_gpu_mode(mode: ProgramImageGpuMode) -> &'static str {
    match mode {
        ProgramImageGpuMode::Cpu => "cpu",
        ProgramImageGpuMode::Cuda => "cuda",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use lzvm_artifacts::program_image::{
        encode_program_image_commitment_cache, ProgramImageCommitmentCache, ProgramImageGpuMode,
    };

    use super::read_program_image_cache_summary;

    fn sample_cache(source_image_digest: [u8; 32]) -> ProgramImageCommitmentCache {
        ProgramImageCommitmentCache {
            program_digest: [0x11; 32],
            source_image_digest,
            constraint_system_digest: [0x22; 32],
            tree_root: [3, 4, 5, 6],
            trace_row_count: 1024,
            trace_column_count: 17,
            blowup_factor: 8,
            merkle_tree_arity: 4,
            gpu_mode: ProgramImageGpuMode::Cuda,
        }
    }

    #[test]
    fn reads_matching_program_image_cache_summary() {
        let dir = std::env::temp_dir().join(format!(
            "lzvm-cli-program-image-cache-summary-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture directory should be created");
        let path = dir.join("cache.bin");
        let cache = sample_cache([0x44; 32]);
        fs::write(
            &path,
            encode_program_image_commitment_cache(&cache).expect("cache should encode"),
        )
        .expect("cache should write");

        let summary =
            read_program_image_cache_summary(&path, [0x44; 32]).expect("cache should validate");

        fs::remove_dir_all(&dir).expect("fixture directory should be removed");

        assert_eq!(summary.path, PathBuf::from(&path));
        assert_eq!(summary.cache.program_digest, [0x11; 32]);
        assert_eq!(summary.cache.source_image_digest, [0x44; 32]);
        assert_eq!(summary.cache.trace_row_count, 1024);
        assert_eq!(summary.cache.trace_column_count, 17);
        assert_eq!(summary.cache.blowup_factor, 8);
        assert_eq!(summary.cache.merkle_tree_arity, 4);
        assert_eq!(summary.cache.gpu_mode, ProgramImageGpuMode::Cuda);
    }

    #[test]
    fn rejects_mismatched_program_image_cache_source_digest() {
        let dir = std::env::temp_dir().join(format!(
            "lzvm-cli-program-image-cache-summary-bad-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture directory should be created");
        let path = dir.join("cache.bin");
        let cache = sample_cache([0x44; 32]);
        fs::write(
            &path,
            encode_program_image_commitment_cache(&cache).expect("cache should encode"),
        )
        .expect("cache should write");

        let result = read_program_image_cache_summary(&path, [0x55; 32]);

        fs::remove_dir_all(&dir).expect("fixture directory should be removed");

        assert!(result.is_err());
    }
}
