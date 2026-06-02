use std::path::Path;

#[test]
fn cuda_row_major_hashing_copies_validated_bytes_without_host_word_repacking() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/merkle_hash.rs");
    let source = std::fs::read_to_string(&source_path).expect("Merkle hash source should read");

    let arity2 = function_body(
        &source,
        "fn cuda_linear_hashes_row_major_arity2",
        "fn cuda_linear_hashes_row_major_arity4",
    );
    let arity4 = function_body(
        &source,
        "fn cuda_linear_hashes_row_major_arity4",
        "type CudaPoseidon2LinearRoundOp",
    );

    for body in [arity2, arity4] {
        assert!(
            body.contains("copy_row_major_bytes_to_device"),
            "row-major CUDA hashing should copy validated bytes directly"
        );
        assert!(
            !body.contains("row_major_words_from_bytes"),
            "row-major CUDA hashing should avoid host-side word repacking"
        );
    }
}

#[test]
fn cuda_witness_leaf_extension_serializes_device_words_without_extended_felt_vector() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/extend.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness extension source should read");

    let cuda_body = function_body(
        &source,
        "fn extend_witness_stage_row_major_bytes",
        "#[cfg(not(feature = \"cuda\"))]",
    );
    let validation_index = cuda_body
        .find("cuda_goldilocks_coset_extend_row_major_columns_output_bytes")
        .expect("CUDA witness leaf extension should validate domain shape");
    let setup_index = cuda_body
        .find("prepare_gpu_setup")
        .expect("CUDA witness leaf extension should initialize CUDA setup");
    let allocation_index = cuda_body
        .find("CudaDeviceBuffer::new")
        .expect("CUDA witness leaf extension should allocate device buffers");

    assert!(
        validation_index < setup_index,
        "CUDA witness leaf extension should validate domain shape before CUDA setup"
    );
    assert!(
        validation_index < allocation_index,
        "CUDA witness leaf extension should validate domain shape before device allocation"
    );
    assert!(
        cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns_device"),
        "CUDA witness leaf extension should write extended rows through device buffers"
    );
    assert!(
        !cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns("),
        "CUDA witness leaf extension should avoid the host-returning extension API"
    );
    assert!(
        !cuda_body.contains("Result<Vec<Felt>"),
        "CUDA witness leaf extension should not materialize extended Felt values"
    );
    assert!(
        !cuda_body.contains(".collect::<Result<Vec<_>, _>>()"),
        "CUDA witness leaf extension should serialize validated words directly"
    );
    assert!(
        cuda_body.contains("Felt::as_u64_slice"),
        "CUDA witness leaf extension should upload Felt words without staging a byte vector"
    );
    assert!(
        !cuda_body.contains("row_major_felt_bytes(values"),
        "CUDA witness leaf extension should avoid per-call Felt-to-byte staging"
    );
}

#[test]
fn cuda_fri_fixed_extension_uses_device_output_without_extended_word_vector() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/prove_fri_polynomial.rs");
    let source = std::fs::read_to_string(&source_path).expect("FRI polynomial source should read");

    let cuda_body = function_body(&source, "fn extend_row_major_columns", "fn fri_error");
    let validation_index = cuda_body
        .find("cuda_goldilocks_coset_extend_row_major_columns_output_bytes")
        .expect("CUDA FRI fixed extension should validate domain shape");
    let setup_index = cuda_body
        .find("prepare_gpu_setup")
        .expect("CUDA FRI fixed extension should initialize CUDA setup");
    let allocation_index = cuda_body
        .find("CudaDeviceBuffer::new")
        .expect("CUDA FRI fixed extension should allocate device buffers");

    assert!(
        validation_index < setup_index,
        "CUDA FRI fixed extension should validate domain shape before CUDA setup"
    );
    assert!(
        validation_index < allocation_index,
        "CUDA FRI fixed extension should validate domain shape before device allocation"
    );
    assert!(
        cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns_device"),
        "CUDA FRI fixed extension should write extended rows through device buffers"
    );
    assert!(
        !cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns("),
        "CUDA FRI fixed extension should avoid the host-returning extension API"
    );
    assert!(
        !cuda_body.contains("collect::<Result<Vec<_>, _>>()"),
        "CUDA FRI fixed extension should avoid a separate extended word vector"
    );
    assert!(
        cuda_body.contains("Felt::as_u64_slice"),
        "CUDA FRI fixed extension should upload Felt words without staging a byte vector"
    );
    assert!(
        !cuda_body.contains("row_major_felt_bytes(values"),
        "CUDA FRI fixed extension should avoid per-call Felt-to-byte staging"
    );
}

#[test]
fn cuda_merkle_root_folds_on_device_without_host_level_loop() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/merkle_hash.rs");
    let source = std::fs::read_to_string(&source_path).expect("Merkle hash source should read");

    let cuda_body = function_body(
        &source,
        "fn root_from_digest_level_on_cuda",
        "fn digest_level_as_state_words",
    );

    assert!(
        cuda_body.contains("cuda_poseidon2_width8_merkle_root_device"),
        "arity-2 CUDA root folding should use the native root operation"
    );
    assert!(
        cuda_body.contains("cuda_poseidon2_width16_merkle_root_device"),
        "arity-4 CUDA root folding should use the native root operation"
    );
    assert!(
        !cuda_body.contains("while state_count > 1"),
        "CUDA root folding should not loop over Merkle levels on the host"
    );
    assert!(
        !cuda_body.contains("CudaDeviceBuffer::new"),
        "CUDA root folding should not allocate a device buffer per Merkle level"
    );
}

#[test]
fn witness_merkle_tree_uses_device_parent_level_pipeline() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/tree.rs");
    let source = std::fs::read_to_string(&source_path).expect("witness tree source should read");

    let commit_body = function_body(
        &source,
        "fn commit_witness_stage_leaves",
        "fn open_witness_stage_commitment",
    );

    assert!(
        commit_body.contains("parent_levels_from_digest_level"),
        "witness Merkle tree construction should reuse the CUDA parent-level pipeline"
    );
    assert!(
        !commit_body.contains("parent_hashes(&level"),
        "witness Merkle tree construction should avoid re-uploading each parent level"
    );

    let merkle_source_path = crate_root.join("src/merkle_hash.rs");
    let merkle_source =
        std::fs::read_to_string(&merkle_source_path).expect("Merkle hash source should read");
    let cuda_body = function_body(
        &merkle_source,
        "fn parent_levels_from_device_buffer",
        "pub(crate) fn root_from_digest_level",
    );
    assert!(
        !cuda_body.contains("from_u64_words"),
        "CUDA parent-level pipeline should not upload each Merkle level"
    );
}

#[test]
fn all_units_witness_reuses_shared_inputs_and_borrows_trace_bundle_bytes() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_execution.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness execution source should read");

    let single_unit_inner = function_body(
        &source,
        "fn run_prove_witness_commitments_with_trace_backend_inner",
        "pub fn run_prove_witness_commitments_for_all_units",
    );
    assert!(
        !single_unit_inner.contains("read_witness_input"),
        "witness execution should preload input data before entering the per-unit body"
    );
    assert!(
        !single_unit_inner.contains("load_public_inputs"),
        "witness execution should preload public inputs before entering the per-unit body"
    );
    assert!(
        single_unit_inner.contains("layout.request(&shared_inputs.input[..])"),
        "witness execution should borrow shared input bytes for each unit"
    );
    assert!(
        !single_unit_inner.contains("request_borrowed"),
        "witness trace layout should use one request constructor for owned and borrowed inputs"
    );
    assert!(
        !single_unit_inner.contains("shared_inputs.input.clone()"),
        "witness execution should avoid cloning shared input bytes for each unit"
    );

    let all_units_body = function_body(
        &source,
        "pub fn run_prove_witness_commitments_for_all_units",
        "pub fn run_prove_witness_commitments_for_all_units_with_trace_bundle",
    );
    assert!(
        all_units_body.contains("load_witness_shared_inputs(plan)"),
        "all-units witness execution should load shared inputs once"
    );

    let trace_bundle_body = function_body(
        &source,
        "pub fn run_prove_witness_commitments_for_all_units_with_trace_bundle",
        "fn validate_trace_bundle_unit_set",
    );
    assert!(
        trace_bundle_body.contains("run_prove_witness_commitments_with_trace_bytes_inner"),
        "all-units trace bundle execution should parse precomputed trace bytes directly"
    );
    assert!(
        !trace_bundle_body.contains("TraceBytesBackend"),
        "all-units trace bundle execution should avoid copying through the witness backend buffer"
    );
}

#[test]
fn cli_single_unit_precomputed_trace_bytes_parse_without_backend_copy() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let source = std::fs::read_to_string(&source_path).expect("prove witness source should read");

    let single_unit_body = function_body(
        &source,
        "let output = if let Some(path) = &plan.inputs.witness_library",
        "let commitments = output.commitments();",
    );

    assert!(
        single_unit_body.contains("run_prove_witness_commitments_with_trace_bytes"),
        "single-unit precomputed trace execution should parse trace bytes directly"
    );
    assert!(
        !single_unit_body.contains("TraceBytesBackend"),
        "single-unit precomputed trace execution should avoid copying through the witness backend buffer"
    );
}

#[test]
fn cli_prove_witness_parses_trace_bundle_without_unit_trace_copies() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let source = std::fs::read_to_string(&source_path).expect("prove witness source should read");

    let trace_bundle_body = function_body(
        &source,
        "let trace_bundle = match",
        "let challenge_values_segment = match",
    );

    assert!(
        trace_bundle_body.contains("parse_trace_bundle_ref"),
        "prove witness should parse trace bundles as borrowed section views"
    );
    assert!(
        !trace_bundle_body.contains("read_trace_bundle_file"),
        "prove witness should not clone trace bundle unit bytes while reading"
    );
}

#[test]
fn fri_opening_from_transcript_values_borrows_large_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/prove_fri_opening.rs");
    let source = std::fs::read_to_string(&source_path).expect("FRI opening source should read");

    let body = function_body(
        &source,
        "pub fn build_pcs_fri_opening_segment_from_transcript_values",
        "pub fn build_pcs_fri_opening_segment_from_trace",
    );

    assert!(
        !body.contains("challenges.clone()"),
        "FRI opening construction should borrow transcript challenges"
    );
    assert!(
        !body.contains("polynomial.clone()"),
        "FRI opening construction should borrow transcript polynomial values"
    );
    assert!(
        body.contains("build_pcs_fri_opening_segment_from_value_refs"),
        "FRI opening construction should use the borrowed opening builder"
    );

    let helper_body = function_body(
        &source,
        "fn build_pcs_fri_opening_segment_from_value_refs",
        "pub fn build_pcs_fri_transcript_values_from_trace",
    );
    for copy_operation in [
        "challenges.clone()",
        "polynomial.clone()",
        "challenges.to_vec()",
        "polynomial.to_vec()",
        "clone_from",
    ] {
        assert!(
            !helper_body.contains(copy_operation),
            "borrowed FRI opening builder should not copy transcript vectors with {copy_operation}"
        );
    }
}

#[test]
fn fri_opening_from_trace_borrows_challenges() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/prove_fri_opening.rs");
    let source = std::fs::read_to_string(&source_path).expect("FRI opening source should read");

    let body = function_body(
        &source,
        "pub fn build_pcs_fri_opening_segment_from_trace",
        "pub fn build_pcs_fri_opening_segment_from_trace_segments",
    );

    assert!(
        body.contains("build_pcs_fri_opening_segment_from_value_refs"),
        "trace FRI opening construction should use the borrowed opening builder"
    );
    assert!(
        body.contains("PcsFriOpeningTraceValue"),
        "trace FRI opening construction should keep local values in a borrowed challenge holder"
    );
    assert!(
        body.contains("challenges: input.challenges"),
        "trace FRI opening construction should store challenge slices directly"
    );
    for copy_operation in [
        "challenges.clone()",
        "challenges.to_vec()",
        "challenges.to_owned()",
        "Vec::from(input.challenges",
        "Vec::from(value.challenges",
        "input.challenges.iter().copied().collect",
        "input.challenges.iter().cloned().collect",
        "clone_from",
    ] {
        assert!(
            !body.contains(copy_operation),
            "trace FRI opening construction should borrow challenges instead of copying with {copy_operation}"
        );
    }
}

#[test]
fn all_units_transcript_proof_borrows_auxiliary_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/proof_artifact.rs");
    let source = std::fs::read_to_string(&source_path).expect("proof artifact source should read");
    let body = function_body(
        &source,
        "fn build_witness_transcript_proof_artifact_for_all_units",
        "struct AllUnitsTranscriptProofInputs",
    );

    for copy_operation in [
        "let mut auxiliary_inputs",
        "ProveWitnessAuxiliaryInputs {",
        "output.auxiliary_inputs().clone()",
        "values.packed_values.clone()",
        "values.packed_values.to_owned()",
        "Vec::from(values.packed_values",
        "proof_inputs.proof_values.to_vec()",
        "proof_inputs.proof_values.to_owned()",
        "Vec::from(proof_inputs.proof_values",
        "proof_inputs.group_values.to_vec()",
        "proof_inputs.group_values.to_owned()",
        "Vec::from(proof_inputs.group_values",
        "values.values.clone()",
        "values.values.to_owned()",
        "Vec::from(values.values",
        "extend_from_slice",
        "clone_from",
    ] {
        assert!(
            !body.contains(copy_operation),
            "all-units transcript proof should borrow auxiliary values instead of copying with {copy_operation}"
        );
    }
    assert!(
        body.contains("ProveWitnessAuxiliaryInputSlices"),
        "all-units transcript proof should assemble borrowed auxiliary slices"
    );
    assert!(
        body.contains("build_pcs_fri_transcript_values_from_trace_segment_refs"),
        "all-units transcript proof should use the borrowed transcript builder"
    );

    let fri_source_path = crate_root.join("src/prove_fri_opening.rs");
    let fri_source =
        std::fs::read_to_string(&fri_source_path).expect("FRI opening source should read");
    let helper_body = function_body(
        &fri_source,
        "pub(crate) fn build_pcs_fri_transcript_values_from_trace_segment_refs",
        "pub fn build_pcs_fri_opening_segment_from_transcript_values",
    );
    assert!(
        helper_body.contains("build_pcs_fri_transcript_values_from_trace_refs"),
        "borrowed trace segment builder should keep borrowed auxiliary slices through FRI transcript construction"
    );
    assert!(
        !helper_body.contains("ProveWitnessAuxiliaryInputs"),
        "borrowed trace segment builder should not rebuild owned auxiliary inputs"
    );
}

#[test]
fn guest_pc_trace_writes_use_direct_trace_builder_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");

    let scalar_body = function_body(&source, "fn write_column", "fn write_optional_column");
    assert!(
        scalar_body.contains("write_trusted_resolved_scalar_value"),
        "guest PC scalar trace writes should avoid slice-based builder dispatch"
    );
    assert!(
        !scalar_body.contains("write_resolved_column_values(row, column.resolved(), &[value])"),
        "guest PC scalar trace writes should not construct a one-value slice"
    );

    let pair_body = function_body(
        &source,
        "fn write_wide_column",
        "fn write_optional_wide_column",
    );
    assert!(
        pair_body.contains("write_trusted_resolved_pair_values"),
        "guest PC pair trace writes should avoid slice-based builder dispatch"
    );
    assert!(
        !pair_body.contains("write_resolved_column_values(row, column.resolved(), &values)"),
        "guest PC pair trace writes should not route through the slice builder API"
    );
}

#[test]
fn zisk_main_report_writes_stream_trace_values_without_row_value_vector() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");

    let write_body = function_body(
        &source,
        "fn write_zisk_main_report_columns",
        "fn write_zisk_main_row_columns",
    );
    assert!(
        !write_body.contains("let values = validate_and_apply_zisk_main_report("),
        "Zisk Main report writes should stream rows instead of collecting trace values first"
    );

    let validate_body = function_body(
        &source,
        "fn validate_and_apply_zisk_main_report",
        "fn lower_stateful_zisk_main_report_rows",
    );
    for allocation in [
        "Result<Vec<ZiskMainReportTraceValues>",
        "let mut values = Vec::with_capacity",
        "values.push(",
        "Ok(values)",
    ] {
        assert!(
            !validate_body.contains(allocation),
            "Zisk Main report validation should not allocate a trace-value vector with {allocation}"
        );
    }
}

#[test]
fn zisk_main_memory_access_validation_avoids_temporary_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");

    let matching_body = function_body(
        &source,
        "fn matching_memory_access",
        "fn validate_zisk_main_memory_accesses",
    );
    assert!(
        !matching_body.contains("collect"),
        "Zisk Main memory access matching should scan without collecting matches"
    );
    assert!(
        !matching_body.contains("Vec<"),
        "Zisk Main memory access matching should avoid temporary vectors"
    );

    let validate_body = function_body(
        &source,
        "fn validate_zisk_main_memory_accesses",
        "fn validate_zisk_main_precompile_memory_accesses",
    );
    for allocation in [
        "Vec::",
        "Vec<",
        "collect",
        "vec![",
        "expected.push",
        "expected.extend",
    ] {
        assert!(
            !validate_body.contains(allocation),
            "Zisk Main memory access validation should avoid temporary vectors with {allocation}"
        );
    }
}

#[test]
fn raw_fixed_material_uses_raw_row_major_bytes() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/fixed_material.rs");
    let source = std::fs::read_to_string(&source_path).expect("fixed material source should read");

    let load_body = function_body(
        &source,
        "fn load_fixed_columns_material_inner",
        "fn fixed_columns_to_row_major_values",
    );
    assert!(
        load_body.contains("raw_fixed_bytes_to_row_major_values(&path, &raw_bytes)?"),
        "raw fixed-column material should reuse row-major raw bytes for Felt material"
    );
    assert!(
        load_body.contains("raw_layout_columns_match_physical_order(&raw_layout)"),
        "raw fixed-column material should guard raw-byte reuse by column order"
    );
    assert!(
        source.contains("fn raw_fixed_bytes_to_row_major_values"),
        "fixed material should provide a raw-byte row-major conversion path"
    );
}

#[test]
fn cuda_regular_constraints_borrow_felt_words_without_value_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/regular_constraints/cuda.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("regular constraints CUDA source should read");

    let body = function_body(
        &source,
        "pub(crate) fn try_evaluate_regular_constraints_cuda_base",
        "fn cuda_base_source_supported",
    );

    assert!(
        body.contains("Felt::as_u64_slice"),
        "CUDA regular constraints should borrow Felt words for CUDA inputs"
    );
    assert!(
        !body.contains(".map(|value| value.to_u64())"),
        "CUDA regular constraints should avoid per-call Felt-to-u64 value vectors"
    );
    assert!(
        body.contains("fixed_values_device_buffer"),
        "CUDA regular constraints should accept a proven row-major fixed device buffer"
    );
}

#[test]
fn regular_constraint_fixed_device_buffer_stays_out_of_cpu_inputs() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/regular_constraints.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("regular constraints source should read");

    let input_body = function_body(
        &source,
        "pub struct RegularConstraintInputs",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
    );

    assert!(
        !source.contains("RegularFixedValuesDeviceBuffer"),
        "regular constraint inputs should not use a Send/Sync wrapper for CUDA fixed values"
    );
    assert!(
        !input_body.contains("CudaDeviceBuffer"),
        "regular constraint inputs should not carry CUDA device buffers into CPU workers"
    );
}

#[test]
fn witness_regular_constraints_pass_only_row_major_fixed_device_buffer() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_execution.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness execution source should read");

    let body = function_body(
        &source,
        "fn validate_witness_regular_constraints",
        "fn map_regular_constraint_eval_error",
    );

    assert!(
        body.contains("material.row_major_device_buffer()"),
        "witness regular constraints should use the fixed material row-major device-buffer guard"
    );
    assert!(
        body.contains("evaluate_regular_constraints_first_violations_with_cuda_fixed_values"),
        "witness regular constraints should pass the guarded buffer only to the CUDA acceleration path"
    );
}

fn function_body<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let body = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing function start: {start}"))
        .1;
    body.split_once(end)
        .unwrap_or_else(|| panic!("missing function end: {end}"))
        .0
}
