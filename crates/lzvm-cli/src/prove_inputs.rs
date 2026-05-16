use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::key_directory::read_key_directory_catalog;
use lzvm_artifacts::program_image::{
    read_program_image_commitment_cache_file, ProgramImageCommitmentCache, ProgramImageGpuMode,
};
use lzvm_prover::{derive_prove_execution_plan, ProveExecutionInputArtifacts};

use crate::prove_plan::{
    format_hash, parse_run_args, write_run_plan_summary, ParseError, ParsedRunArgs,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_run_args(args, 4, 5) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    let catalog = match read_key_directory_catalog(&parsed.positionals[0]) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    };

    let inputs = parsed_inputs(&parsed);
    let plan = match derive_prove_execution_plan(&catalog, parsed.request, inputs) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    };

    let cache_summary = match parsed.program_image_cache.as_ref() {
        Some(path) => match read_program_image_cache_summary(path, plan.guest_image_info.digest) {
            Ok(summary) => Some(summary),
            Err(error) => {
                let _ = writeln!(stderr, "prove inputs failed: {error}");
                return 1;
            }
        },
        None => None,
    };

    write_run_plan_summary(stdout, &plan.run_plan);
    let _ = writeln!(
        stdout,
        "witness_library={}",
        plan.inputs.witness_library.display()
    );
    let _ = writeln!(
        stdout,
        "witness_library_bytes={}",
        plan.witness_library_info.byte_len
    );
    let _ = writeln!(
        stdout,
        "witness_library_machine={}",
        plan.witness_library_info.machine
    );
    let _ = writeln!(
        stdout,
        "witness_library_digest={}",
        format_hash(&plan.witness_library_info.digest)
    );
    let _ = writeln!(stdout, "guest_image={}", plan.inputs.guest_image.display());
    let _ = writeln!(
        stdout,
        "guest_image_bytes={}",
        plan.guest_image_info.byte_len
    );
    let _ = writeln!(
        stdout,
        "guest_image_machine={}",
        plan.guest_image_info.machine
    );
    let _ = writeln!(stdout, "guest_image_entry={}", plan.guest_image_info.entry);
    let _ = writeln!(
        stdout,
        "guest_image_digest={}",
        format_hash(&plan.guest_image_info.digest)
    );
    let _ = writeln!(
        stdout,
        "public_inputs={}",
        plan.inputs
            .public_inputs
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    if let Some(summary) = cache_summary {
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
    0
}

fn parsed_inputs(parsed: &ParsedRunArgs) -> ProveExecutionInputArtifacts {
    ProveExecutionInputArtifacts {
        witness_library: parsed.positionals[2].clone(),
        guest_image: parsed.positionals[3].clone(),
        public_inputs: parsed.positionals.get(4).cloned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramImageCacheSummary {
    pub path: PathBuf,
    pub cache: ProgramImageCommitmentCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramImageCacheSummaryError {
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

pub fn read_program_image_cache_summary(
    path: impl AsRef<std::path::Path>,
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

fn format_program_image_gpu_mode(mode: ProgramImageGpuMode) -> &'static str {
    match mode {
        ProgramImageGpuMode::Cpu => "cpu",
        ProgramImageGpuMode::Cuda => "cuda",
    }
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]"
    );
    2
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
