use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use lzvm_artifacts::constant_tree::ConstantTreeFileSummary;
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_prover::ProveSchedule;

use super::timing::TimingRecorder;

pub(super) struct ConstantTreeMaterialValidationJob {
    handle: JoinHandle<Result<Vec<Option<ConstantTreeFileSummary>>, String>>,
    started: Instant,
}

const EAGER_CONSTANT_MATERIAL_VALIDATION_ENV: &str = "LZVM_EAGER_CONSTANT_MATERIAL_VALIDATION";

pub(super) fn eager_constant_material_validation_enabled_from_env(
    public_inputs_present: bool,
    contribution_only: bool,
) -> bool {
    let value = std::env::var(EAGER_CONSTANT_MATERIAL_VALIDATION_ENV).ok();
    eager_constant_material_validation_enabled(
        public_inputs_present,
        contribution_only,
        value.as_deref(),
    )
}

pub(super) fn eager_constant_material_validation_enabled(
    public_inputs_present: bool,
    contribution_only: bool,
    value: Option<&str>,
) -> bool {
    public_inputs_present
        && !contribution_only
        && value.is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub(super) fn start_constant_tree_material_validation(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    enabled: bool,
) -> Option<ConstantTreeMaterialValidationJob> {
    if !enabled {
        return None;
    }
    let catalog = catalog.clone();
    let schedule = schedule.clone();
    Some(ConstantTreeMaterialValidationJob {
        started: Instant::now(),
        handle: thread::Builder::new()
            .name("lzvm-ct-val".to_owned())
            .spawn(move || {
                lzvm_prover::validate_constant_opening_materials(&catalog, &schedule)
                    .map_err(|error| error.to_string())
            })
            .expect("constant-tree material validation thread should spawn"),
    })
}

pub(super) fn join_constant_tree_material_validation(
    job: &mut Option<ConstantTreeMaterialValidationJob>,
    timings: &mut TimingRecorder,
) -> Result<Option<Vec<Option<ConstantTreeFileSummary>>>, String> {
    let Some(job) = job.take() else {
        return Ok(None);
    };
    let started = job.started;
    let join_started = Instant::now();
    let summaries = job
        .handle
        .join()
        .map_err(|_| "constant-tree material validation thread panicked".to_owned())?;
    let summaries = summaries?;
    let join_wait = join_started.elapsed();
    record_constant_material_validation_timing(timings, started.elapsed(), join_wait, &summaries);
    Ok(Some(summaries))
}

pub(super) fn record_constant_material_validation_timing(
    timings: &mut TimingRecorder,
    elapsed: Duration,
    join_wait: Duration,
    summaries: &[Option<ConstantTreeFileSummary>],
) {
    let mut byte_count = 0u64;
    let mut unit_count = 0usize;
    for summary in summaries.iter().flatten() {
        unit_count += 1;
        byte_count = byte_count.saturating_add(summary.byte_count);
    }
    timings.record("constant_material_validation_elapsed", elapsed);
    timings.record("constant_material_validation_join_wait", join_wait);
    timings.record_count("constant_material_validation_units", unit_count);
    timings.record_count(
        "constant_material_validation_bytes",
        usize::try_from(byte_count).unwrap_or(usize::MAX),
    );
}
