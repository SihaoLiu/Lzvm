use lzvm_artifacts::trace_bundle::TraceBundleSource;
use lzvm_prover::{witness_layout::derive_witness_trace_layout, ProveSchedule};

pub(crate) fn validate_trace_input_shapes<B: TraceBundleSource + ?Sized>(
    trace_bytes_len: Option<u64>,
    trace_bundle: Option<&B>,
    aggregate: bool,
    schedule: &ProveSchedule,
) -> Result<(), String> {
    if let Some(byte_len) = trace_bytes_len {
        validate_trace_unit_byte_len("trace bytes", 0, byte_len, schedule)?;
    }
    if let Some(bundle) = trace_bundle {
        validate_trace_bundle_shape(bundle, aggregate, schedule)?;
    }
    Ok(())
}

fn validate_trace_bundle_shape<B: TraceBundleSource + ?Sized>(
    bundle: &B,
    aggregate: bool,
    schedule: &ProveSchedule,
) -> Result<(), String> {
    if !aggregate {
        let Some(trace_bytes) = bundle.trace_bytes_for_unit(0) else {
            return Err("trace bundle is missing unit 0".to_owned());
        };
        let byte_len = u64::try_from(trace_bytes.len())
            .map_err(|_| "trace bundle unit 0 byte length overflow".to_owned())?;
        return validate_trace_unit_byte_len("trace bundle", 0, byte_len, schedule);
    }

    let mut seen = vec![false; schedule.units.len()];
    for unit_index_u32 in bundle.unit_indices() {
        let unit_index = usize::try_from(unit_index_u32)
            .map_err(|_| format!("trace bundle unit index is too large: {unit_index_u32}"))?;
        let trace_bytes = bundle
            .trace_bytes_for_unit(unit_index_u32)
            .ok_or_else(|| format!("trace bundle is missing unit {unit_index}"))?;
        let byte_len = u64::try_from(trace_bytes.len())
            .map_err(|_| format!("trace bundle unit {unit_index} byte length overflow"))?;
        validate_trace_unit_byte_len("trace bundle", unit_index, byte_len, schedule)?;
        let Some(present) = seen.get_mut(unit_index) else {
            return Err(format!("trace bundle has unexpected unit {unit_index}"));
        };
        *present = true;
    }
    for (unit_index, present) in seen.into_iter().enumerate() {
        if !present {
            return Err(format!("trace bundle is missing unit {unit_index}"));
        }
    }
    Ok(())
}

fn validate_trace_unit_byte_len(
    role: &str,
    unit_index: usize,
    byte_len: u64,
    schedule: &ProveSchedule,
) -> Result<(), String> {
    let expected = expected_trace_unit_byte_len(unit_index, schedule)?;
    if byte_len != expected {
        return Err(format!(
            "{role} unit {unit_index} byte length mismatch: expected {expected}, found {byte_len}"
        ));
    }
    Ok(())
}

fn expected_trace_unit_byte_len(
    unit_index: usize,
    schedule: &ProveSchedule,
) -> Result<u64, String> {
    let unit = schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("trace bundle has unexpected unit {unit_index}"))?;
    let layout = derive_witness_trace_layout(unit)
        .map_err(|error| format!("trace unit {unit_index} layout failed: {error}"))?;
    let elements = layout
        .row_count()
        .checked_mul(layout.column_count())
        .ok_or_else(|| format!("trace unit {unit_index} byte length overflow"))?;
    let bytes = elements
        .checked_mul(8)
        .ok_or_else(|| format!("trace unit {unit_index} byte length overflow"))?;
    u64::try_from(bytes).map_err(|_| format!("trace unit {unit_index} byte length overflow"))
}
