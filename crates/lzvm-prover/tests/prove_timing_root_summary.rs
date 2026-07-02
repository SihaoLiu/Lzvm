use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;

    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(ch);
            }
        } else if ch == ',' {
            fields.push(std::mem::take(&mut field));
        } else if ch == '"' && field.is_empty() {
            quoted = true;
        } else {
            field.push(ch);
        }
    }

    fields.push(field);
    fields
}

fn run_prove_timing_root_summary(input_lines: &[&str]) -> String {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = input_lines.join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn parse_summary_values(stdout: &str) -> BTreeMap<String, String> {
    let mut lines = stdout.lines();
    let header = parse_csv_line(lines.next().expect("summary should include a header"));
    let row = parse_csv_line(lines.next().expect("summary should include a data row"));
    assert!(
        lines.next().is_none(),
        "summary should include exactly one data row: stdout={stdout}"
    );
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let mut values = BTreeMap::new();
    for (name, value) in header.into_iter().zip(row) {
        assert!(
            values.insert(name.clone(), value).is_none(),
            "summary header should not repeat {name}: stdout={stdout}"
        );
    }
    values
}

fn prove_timing_root_summary_values(input_lines: &[&str]) -> BTreeMap<String, String> {
    let stdout = run_prove_timing_root_summary(input_lines);
    parse_summary_values(&stdout)
}

fn prove_timing_root_summary_value(input_lines: &[&str], name: &str) -> String {
    let stdout = run_prove_timing_root_summary(input_lines);
    parse_summary_values(&stdout)
        .remove(name)
        .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"))
}

fn expect_summary_value<'a>(values: &'a BTreeMap<String, String>, name: &str) -> &'a str {
    values.get(name).map(String::as_str).unwrap_or_else(|| {
        let keys = values.keys().collect::<Vec<_>>();
        panic!("summary should expose {name}: keys={keys:?}")
    })
}

#[cfg(unix)]
#[test]
fn prove_timing_root_summary_script_is_directly_executable() {
    use std::os::unix::fs::PermissionsExt;

    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let mode = std::fs::metadata(&script_path)
        .expect("prove timing root summary metadata should read")
        .permissions()
        .mode();
    assert_ne!(
        mode & 0o111,
        0,
        "prove timing root summary should be executable as a profiling helper"
    );

    let output = Command::new(&script_path)
        .arg("--self-test")
        .output()
        .expect("prove timing root summary should run directly through its shebang");
    assert!(
        output.status.success(),
        "prove timing root summary direct self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn prove_timing_root_summary_rejects_conflicting_duplicate_timing_fields() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join(format!(
        "../../temp/prove-timing-duplicate-field-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("duplicate timing fixture dir should be created");
    let log_path = dir.join("duplicate.log");
    let input = [
        "timing_total_ms=1000",
        "timing_total_ms=2000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");
    std::fs::write(&log_path, input).expect("duplicate timing fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "conflicting duplicate timing fields should be rejected"
    );
    assert!(
        stderr.contains("duplicate timing field: timing_total_ms"),
        "duplicate timing rejection should name the repeated field: stderr={stderr}"
    );
}

#[test]
fn prove_timing_root_summary_accepts_identical_duplicate_timing_fields() {
    let input = [
        "timing_total_ms=1000",
        "timing_total_ms=1000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ];
    let values = prove_timing_root_summary_values(&input);

    assert_eq!(
        expect_summary_value(&values, "total_ms"),
        "1000",
        "summary should preserve the duplicated timing value once"
    );
}

#[test]
fn prove_timing_root_summary_rejects_malformed_timing_fields() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join(format!(
        "../../temp/prove-timing-malformed-field-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("malformed timing fixture dir should be created");

    for (name, first_line, expected) in [
        (
            "invalid",
            "timing_total_ms=not-a-number",
            "invalid timing field: timing_total_ms",
        ),
        (
            "negative",
            "timing_total_ms=-1",
            "negative timing field: timing_total_ms",
        ),
    ] {
        let log_path = dir.join(format!("{name}.log"));
        let input = [
            first_line,
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n");
        std::fs::write(&log_path, input).expect("malformed timing fixture should be written");

        let output = Command::new("python3")
            .arg(&script_path)
            .arg(&log_path)
            .output()
            .expect("prove timing root summary should run");
        assert!(
            !output.status.success(),
            "malformed timing field {name} should be rejected"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(expected),
            "malformed timing rejection should name the bad field: stderr={stderr}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prove_timing_root_summary_reports_stream_chunk_process_time() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-stream-chunk-process");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stream chunk fixture dir should be created");
    let log_path = dir.join("stream-chunk-process.log");
    let input = [
        "timing_total_ms=9717",
        "input_bytes=643026",
        "timing_guest_trace_stream_elapsed_ms=9000",
        "timing_guest_trace_stream_ms=8800",
        "timing_guest_trace_parallel_lower_workers=8",
        "timing_guest_trace_parallel_lower_stream_segments=4",
        "timing_guest_trace_parallel_lower_stream_chunks=512",
        "timing_guest_trace_parallel_lower_stream_fallbacks=1",
        "timing_guest_trace_parallel_lower_stream_retained_reports=7",
        "timing_guest_trace_owned_streaming_lower_segments=3",
        "timing_guest_trace_parallel_lower_stream_chunk_process_ms=123",
        "timing_guest_trace_parallel_lower_job_receive_wait_ms=456",
        "timing_guest_trace_parallel_lower_result_send_wait_ms=789",
        "timing_guest_trace_report_chunk_reports=2097152",
        "timing_guest_trace_report_apply_ms=111",
        "timing_guest_trace_unit_summary_ms=12",
        "timing_guest_trace_parallel_lower_stream_chunk_dispatch_wait_ms=299",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=8701",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");
    std::fs::write(&log_path, input).expect("stream chunk fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should finish");

    assert!(
        output.status.success(),
        "prove timing root summary should parse stream chunk input: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let values =
        parse_summary_values(&String::from_utf8(output.stdout).expect("stdout should be utf-8"));
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_chunk_process_ms"),
        "123",
        "stream chunk process timing should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_segments"),
        "4",
        "stream segment count should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_chunks"),
        "512",
        "stream chunk count should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_chunks_per_segment"),
        "128.000",
        "stream chunks per segment should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_reports_per_chunk"),
        "4096.000",
        "stream reports per chunk should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_shape_hint"),
        "many_chunks_per_segment",
        "stream shape hint should identify segment-internal chunk serialization pressure"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_fallbacks"),
        "1",
        "stream fallback count should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_stream_retained_reports"),
        "7",
        "stream retained report count should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "owned_streaming_lower_segments"),
        "3",
        "owned streaming lower segment count should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_job_receive_wait_ms"),
        "456",
        "worker job receive wait timing should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "parallel_lower_result_send_wait_ms"),
        "789",
        "worker result send wait timing should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "trace_report_apply_ms"),
        "111",
        "report apply timing should be surfaced in root summary"
    );
    assert_eq!(
        expect_summary_value(&values, "trace_unit_summary_ms"),
        "12",
        "unit summary timing should be surfaced in root summary"
    );
}

#[test]
fn prove_timing_root_summary_reports_device_source_timings() {
    let input = [
        "timing_total_ms=1000",
        "input_bytes=1024",
        "timing_guest_device_source_build_ms=31",
        "timing_guest_device_source_descriptor_upload_ms=29",
        "timing_guest_device_source_trace_expand_ms=2",
        "timing_guest_device_source_descriptor_upload_bytes=880",
        "timing_guest_device_source_descriptor_upload_rows=10",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ];
    let values = prove_timing_root_summary_values(&input);

    assert_eq!(
        expect_summary_value(&values, "device_source_build_ms"),
        "31"
    );
    assert_eq!(expect_summary_value(&values, "descriptor_upload_ms"), "29");
    assert_eq!(
        expect_summary_value(&values, "device_source_trace_expand_ms"),
        "2"
    );
    assert_eq!(
        expect_summary_value(&values, "descriptor_upload_bytes"),
        "880"
    );
    assert_eq!(
        expect_summary_value(&values, "descriptor_bytes_per_row"),
        "88.000"
    );
}

#[test]
fn prove_timing_root_summary_reports_opening_external_source_timings() {
    let input = [
        "timing_total_ms=1000",
        "input_bytes=1024",
        "timing_finish_witness_opening_ms=123",
        "timing_finish_witness_opening_external_source_count=2",
        "timing_finish_witness_external_source_ms=45",
        "timing_finish_witness_external_source_descriptor_upload_ms=37",
        "timing_finish_witness_external_source_descriptor_upload_bytes=880",
        "timing_finish_witness_external_source_descriptor_upload_words=110",
        "timing_finish_witness_external_source_descriptor_upload_rows=10",
        "timing_finish_witness_external_source_trace_expand_ms=8",
        "timing_guest_descriptor_buffer_retention_rejected=3",
        "timing_guest_descriptor_buffer_retention_limit_bytes=1400",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ];
    let values = prove_timing_root_summary_values(&input);

    assert_eq!(
        expect_summary_value(&values, "opening_external_source_ms"),
        "45"
    );
    assert_eq!(
        expect_summary_value(&values, "opening_external_source_descriptor_upload_ms"),
        "37"
    );
    assert_eq!(
        expect_summary_value(&values, "opening_external_source_descriptor_upload_bytes"),
        "880"
    );
    assert_eq!(
        expect_summary_value(&values, "opening_external_source_descriptor_upload_words"),
        "110"
    );
    assert_eq!(
        expect_summary_value(&values, "opening_external_source_descriptor_upload_rows"),
        "10"
    );
    assert_eq!(
        expect_summary_value(&values, "opening_external_source_trace_expand_ms"),
        "8"
    );
    assert_eq!(
        expect_summary_value(&values, "opening_external_source_descriptor_action_hint"),
        "opening_descriptor_reupload_after_retention_reject"
    );
}

#[test]
fn prove_timing_root_summary_reports_opening_descriptor_action_hint_branches() {
    let base = [
        "timing_total_ms=1000",
        "input_bytes=1024",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ];
    let field = "opening_external_source_descriptor_action_hint";

    let mut no_external = base.to_vec();
    no_external.extend([
        "timing_finish_witness_opening_external_source_count=0",
        "timing_finish_witness_external_source_descriptor_upload_bytes=880",
    ]);
    assert_eq!(prove_timing_root_summary_value(&no_external, field), "none");

    let mut no_budget = base.to_vec();
    no_budget.extend([
        "timing_finish_witness_opening_external_source_count=2",
        "timing_finish_witness_external_source_descriptor_upload_ms=37",
        "timing_finish_witness_external_source_descriptor_upload_bytes=880",
        "timing_guest_descriptor_buffer_retention_limit_bytes=0",
    ]);
    assert_eq!(
        prove_timing_root_summary_value(&no_budget, field),
        "opening_descriptor_reupload_without_retention_budget"
    );

    let mut plain_reupload = base.to_vec();
    plain_reupload.extend([
        "timing_finish_witness_opening_external_source_count=2",
        "timing_finish_witness_external_source_descriptor_upload_ms=37",
        "timing_finish_witness_external_source_descriptor_upload_bytes=880",
        "timing_guest_descriptor_buffer_retention_limit_bytes=1400",
    ]);
    assert_eq!(
        prove_timing_root_summary_value(&plain_reupload, field),
        "opening_descriptor_reupload"
    );

    let mut bytes_only = base.to_vec();
    bytes_only.extend([
        "timing_finish_witness_opening_external_source_count=2",
        "timing_finish_witness_external_source_descriptor_upload_bytes=880",
        "timing_guest_descriptor_buffer_retention_limit_bytes=1400",
    ]);
    assert_eq!(
        prove_timing_root_summary_value(&bytes_only, field),
        "opening_descriptor_reupload_bytes_only"
    );
}

#[test]
fn prove_timing_root_summary_reports_fri_transcript_and_contribution_work() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join(format!(
        "../../temp/prove-timing-final-proof-work-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("final proof timing fixture dir should be created");
    let log_path = dir.join("final-proof.log");
    let input = [
        "timing_total_ms=100",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_fri_opening_ms=10",
        "timing_finish_fri_opening_unit_build_ms=8",
        "timing_finish_fri_opening_layer_tree_ms=2",
        "timing_finish_fri_opening_query_ms=3",
        "timing_finish_fri_opening_fold_ms=1",
        "timing_finish_fri_opening_unit_count=1",
        "timing_finish_fri_opening_layer_count=2",
        "timing_finish_fri_opening_query_count=3",
        "timing_finish_fri_transcript_unit_build_ms=4",
        "timing_finish_fri_transcript_layer_tree_ms=2",
        "timing_finish_fri_transcript_fold_ms=1",
        "timing_finish_fri_transcript_unit_count=1",
        "timing_finish_fri_transcript_layer_count=2",
        "timing_finish_contribution_segment_ms=5",
        "timing_finish_contribution_verify_ms=6",
        "timing_finish_contribution_challenge_ms=7",
    ]
    .join("\n");
    std::fs::write(&log_path, input).expect("final proof timing fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should finish");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        success,
        "prove timing root summary should parse final proof input: stderr={stderr}"
    );
    let values = parse_summary_values(&stdout);

    assert_eq!(
        expect_summary_value(&values, "fri_transcript_unit_build_ms"),
        "4"
    );
    assert_eq!(
        expect_summary_value(&values, "fri_transcript_layer_tree_ms"),
        "2"
    );
    assert_eq!(expect_summary_value(&values, "fri_transcript_fold_ms"), "1");
    assert_eq!(expect_summary_value(&values, "fri_transcript_units"), "1");
    assert_eq!(expect_summary_value(&values, "fri_transcript_layers"), "2");
    assert_eq!(
        expect_summary_value(&values, "fri_transcript_layers_per_unit"),
        "2.000"
    );
    assert_eq!(
        expect_summary_value(&values, "contribution_segment_ms"),
        "5"
    );
    assert_eq!(expect_summary_value(&values, "contribution_verify_ms"), "6");
    assert_eq!(
        expect_summary_value(&values, "contribution_challenge_ms"),
        "7"
    );
    assert_eq!(expect_summary_value(&values, "contribution_total_ms"), "18");
    assert_eq!(
        expect_summary_value(&values, "fri_opening_total_pct"),
        "10.000"
    );
    assert_eq!(
        expect_summary_value(&values, "fri_transcript_unit_build_total_pct"),
        "4.000"
    );
    assert_eq!(
        expect_summary_value(&values, "contribution_total_pct"),
        "18.000"
    );
    assert_eq!(
        expect_summary_value(&values, "final_proof_timing_hint"),
        "profile_final_proof_contribution"
    );
}

#[test]
fn prove_timing_root_summary_reports_final_proof_hint_branches() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let output = Command::new("python3")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .arg("-c")
        .arg(
            r#"
import importlib.util
import sys

spec = importlib.util.spec_from_file_location("root_summary", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
for case in [
    (100, 0, 0, 0),
    (1000, 10, 4, 18),
    (100, 10, 10, 10),
    (0, 1, 2, 3),
]:
    print(module.final_proof_timing_hint(*case))
"#,
        )
        .arg(&script_path)
        .output()
        .expect("prove timing root summary helper should run");

    assert!(
        output.status.success(),
        "prove timing root summary helper should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(
        lines,
        [
            "none",
            "final_proof_not_dominant",
            "profile_final_proof_fri_opening",
            "profile_final_proof_contribution",
        ]
    );
}

#[test]
fn prove_timing_root_summary_reports_root_grouping_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let source = std::fs::read_to_string(&script_path)
        .expect("prove timing root summary source should read");

    for required in [
        "timing_guest_stage_tree_commit_root_count",
        "timing_guest_stage_tree_commit_root_materialization_groups",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size",
        "timing_guest_stage_leaf_kernel_work_ms",
        "timing_guest_stage_leaf_coset_extend_calls",
        "timing_guest_stage_leaf_coset_extend_columns",
        "timing_guest_stage_leaf_coset_extend_ntt_launches",
        "timing_guest_stage_leaf_coset_extend_ntt_stage_launches",
        "timing_guest_stage_leaf_coset_extend_ntt_block_twiddle_launches",
        "timing_cuda_direct_copy_d2h_wait_ns",
        "timing_cuda_direct_copy_d2h_hot_bytes",
        "timing_cuda_direct_copy_d2h_hot_count",
        "timing_cuda_direct_copy_d2h_hot_wait_ns",
        "direct_d2h_hot_wait_pct",
        "direct_d2h_action_hint",
        "timing_cuda_setup_init_calls",
        "timing_cuda_setup_init_wait_ns",
        "timing_cuda_setup_init_max_wait_ns",
        "timing_cuda_setup_cache_hits",
        "timing_cuda_setup_cache_hit_wait_ns",
        "timing_cuda_setup_cache_hit_max_wait_ns",
        "timing_cuda_setup_native_init_calls",
        "timing_cuda_setup_native_init_wait_ns",
        "timing_cuda_setup_native_init_max_wait_ns",
        "timing_cuda_current_device_calls",
        "timing_cuda_current_device_wait_ns",
        "timing_cuda_current_device_max_wait_ns",
        "timing_cuda_memory_info_calls",
        "timing_cuda_memory_info_wait_ns",
        "timing_cuda_memory_info_max_wait_ns",
        "timing_cuda_allocator_malloc_calls",
        "timing_cuda_allocator_malloc_wait_ns",
        "timing_cuda_allocator_malloc_max_wait_ns",
        "cuda_allocator_malloc_wait_ms",
        "timing_cuda_allocator_host_register_wait_ns",
        "timing_cuda_allocator_copy_h2d_bytes",
        "timing_cuda_allocator_copy_h2d_wait_ns",
        "timing_cuda_allocator_copy_h2d_hot_bytes",
        "timing_cuda_allocator_copy_h2d_hot_count",
        "timing_cuda_allocator_copy_h2d_hot_wait_ns",
        "timing_guest_trace_runner_ms",
        "timing_guest_trace_lowerer_ms",
        "timing_guest_trace_lower_ms",
        "trace_lower_ms",
        "timing_guest_trace_report_ms",
        "trace_report_ms",
        "trace_non_report_ms",
        "trace_runner_lowerer_overlap_ms",
        "trace_lowerer_non_lower_ms",
        "timing_guest_trace_stream_elapsed_ms",
        "timing_guest_trace_stream_ms",
        "timing_guest_segment_commit_ms",
        "timing_guest_segment_commit_initial_workers",
        "timing_guest_segment_commit_effective_workers",
        "timing_guest_segment_commit_worker_submits",
        "timing_guest_segment_commit_worker_joins",
        "timing_guest_segment_commit_worker_backpressure_joins",
        "timing_guest_segment_commit_worker_backpressure_join_ms",
        "timing_guest_segment_commit_worker_finish_joins",
        "timing_guest_segment_commit_worker_finish_join_ms",
        "timing_guest_segment_commit_worker_max_in_flight",
        "timing_guest_segment_commit_oom_retries",
        "timing_guest_segment_commit_attempt_ms",
        "timing_guest_segment_commit_oom_retry_ms",
        "timing_guest_segment_input_gap_ms",
        "timing_guest_segment_input_gap_max_ms",
        "timing_guest_segment_input_gap_count",
        "segment_input_gap_avg_ms",
        "timing_guest_trace_segment_receive_wait_ms",
        "timing_guest_trace_pending_receive_wait_ms",
        "timing_guest_trace_pending_send_wait_ms",
        "timing_guest_trace_parallel_lower_workers",
        "timing_guest_trace_parallel_lower_dispatched",
        "timing_guest_trace_parallel_lower_received",
        "timing_guest_trace_parallel_lower_emitted",
        "timing_guest_trace_parallel_lower_max_reorder",
        "timing_guest_trace_parallel_lower_snapshot_replay_count",
        "timing_guest_trace_parallel_lower_snapshot_replay_ms",
        "timing_guest_trace_parallel_lower_report_elided_count",
        "timing_guest_trace_parallel_lower_stream_segments",
        "timing_guest_trace_parallel_lower_stream_chunks",
        "timing_guest_trace_parallel_lower_stream_fallbacks",
        "timing_guest_trace_parallel_lower_stream_retained_reports",
        "timing_guest_trace_owned_streaming_lower_segments",
        "parallel_lower_stream_chunks_per_segment",
        "parallel_lower_stream_reports_per_chunk",
        "parallel_lower_stream_shape_hint",
        "timing_guest_trace_report_apply_ms",
        "timing_guest_trace_unit_summary_ms",
        "timing_guest_trace_parallel_lower_dispatch_wait_ms",
        "timing_guest_trace_parallel_lower_stream_chunk_process_ms",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms",
        "timing_guest_trace_parallel_lower_dispatch_blocked_count",
        "timing_guest_trace_segment_replay_count",
        "timing_guest_trace_reports",
        "timing_guest_trace_report_rows",
        "timing_guest_trace_report_chunk_sent",
        "timing_guest_trace_report_chunk_received",
        "timing_guest_trace_report_chunk_reports",
        "timing_guest_trace_report_chunk_rows",
        "timing_guest_trace_report_chunk_max_queued",
        "timing_guest_trace_external_op_runs",
        "timing_guest_trace_external_op_max_run",
        "timing_guest_trace_copy_runs",
        "timing_guest_trace_copy_max_run",
        "trace_shape_run_hint",
        "timing_guest_trace_report_buffer_capacity",
        "timing_guest_trace_report_buffer_max_capacity",
        "timing_guest_trace_report_buffer_excess_capacity",
        "timing_guest_trace_report_record_size_bytes",
        "timing_guest_trace_report_instruction_size_bytes",
        "timing_guest_trace_report_register_write_list_size_bytes",
        "timing_guest_trace_report_memory_access_list_size_bytes",
        "timing_guest_trace_report_precompile_access_list_size_bytes",
        "timing_guest_trace_report_storage_bytes",
        "timing_guest_trace_report_buffer_capacity_bytes",
        "timing_guest_trace_report_buffer_excess_bytes",
        "trace_report_buffer_shape_hint",
        "trace_report_storage_gib",
        "trace_report_buffer_capacity_gib",
        "timing_guest_trace_descriptor_rows",
        "timing_guest_trace_descriptor_compact_rows",
        "timing_guest_trace_descriptor_wide_rows",
        "timing_guest_device_source_build_ms",
        "timing_guest_device_source_descriptor_upload_ms",
        "timing_guest_device_source_trace_expand_ms",
        "device_source_build_ms",
        "descriptor_upload_ms",
        "device_source_trace_expand_ms",
        "timing_guest_device_source_descriptor_upload_bytes",
        "timing_guest_device_source_descriptor_upload_rows",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows",
        "timing_guest_trace_descriptor_high32_a_values",
        "timing_guest_trace_descriptor_high32_b_values",
        "timing_guest_trace_descriptor_high32_c_values",
        "timing_guest_trace_descriptor_high32_a_payload_values",
        "timing_guest_trace_descriptor_high32_b_payload_values",
        "timing_guest_trace_descriptor_high32_store_payload_values",
        "timing_guest_trace_descriptor_high32_store_prev_value_values",
        "timing_guest_trace_descriptor_high32_rows_with_0_fields",
        "timing_guest_trace_descriptor_high32_rows_with_7_fields",
        "descriptor_sparse_high32_estimated_upload_bytes",
        "descriptor_sparse_high32_shape_hint",
        "descriptor_shape_hint",
        "timing_guest_trace_seed_direct_lift_attempts",
        "timing_guest_trace_seed_direct_lift_successes",
        "seed_snapshot_runtime_hint",
        "timing_guest_trace_seed_full_advances",
        "timing_finish_witness_opening_ms",
        "timing_finish_witness_external_source_ms",
        "timing_finish_witness_external_source_descriptor_upload_ms",
        "timing_finish_witness_external_source_descriptor_upload_bytes",
        "timing_finish_witness_external_source_descriptor_upload_words",
        "timing_finish_witness_external_source_descriptor_upload_rows",
        "timing_finish_witness_external_source_trace_expand_ms",
        "opening_external_source_ms",
        "opening_external_source_descriptor_upload_ms",
        "opening_external_source_descriptor_upload_bytes",
        "opening_external_source_descriptor_upload_words",
        "opening_external_source_descriptor_upload_rows",
        "opening_external_source_trace_expand_ms",
        "timing_finish_witness_opening_query_unit_count",
        "timing_finish_witness_opening_single_query_unit_count",
        "timing_finish_witness_opening_query_count",
        "timing_finish_witness_opening_max_queries_per_unit",
        "timing_finish_witness_opening_stage_count",
        "timing_finish_witness_opening_retained_source_count",
        "timing_finish_witness_opening_external_source_count",
        "timing_finish_witness_opening_embedded_source_count",
        "timing_finish_witness_opening_missing_source_count",
        "timing_guest_stage_source_retention_attempts",
        "timing_guest_stage_source_retention_retained",
        "timing_guest_stage_source_retention_rejected",
        "timing_guest_stage_source_retention_max_retained_bytes",
        "timing_guest_stage_source_retention_max_rejected_bytes",
        "timing_guest_stage_source_retention_limit_bytes",
        "timing_guest_stage_source_upload_ms",
        "timing_guest_retained_trace_artifact_ms",
        "opening_source_rebuild_hint",
        "opening_external_source_descriptor_action_hint",
        "timing_finish_witness_opening_row_values_device_rows",
        "timing_finish_witness_opening_row_values_source_rows",
        "timing_finish_witness_opening_row_value_source_extend_ms",
        "opening_row_value_source_extend_ms",
        "opening_row_value_source_extend_pct",
        "opening_source_row_value_action_hint",
        "timing_finish_witness_opening_retained_leaf_digest_openings",
        "timing_finish_witness_opening_retained_leaf_digest_rows",
        "timing_finish_witness_opening_retained_leaf_digest_all_single_row_openings",
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms",
        "timing_finish_witness_opening_path_parent_hash_launches_per_stage",
        "timing_finish_witness_opening_row_values_device_download_batches",
        "OPENING_STAGE_ROW_VALUE_DEVICE_DOWNLOAD_BATCH_RE",
        "opening_row_value_device_batch_stage_count",
        "opening_row_value_device_batch_max_stage",
        "opening_row_value_device_batch_stage_sum",
        "opening_row_value_device_batch_unattributed",
        "timing_constant_material_validation_elapsed_ms",
        "timing_constant_material_validation_join_wait_ms",
        "constant_material_validation_overlap_hint",
        "input_bytes",
        "needs_cross_segment_root_pipeline",
        "opening_batching_hint",
        "opening_retained_parent_checkpoint_action_hint",
        "retained_parent_checkpoint_path_time_secondary",
        "leaf_launch_pressure",
        "primary_bottleneck",
        "trace_structure_hint",
        "trace_to_leaf_ratio",
        "proof_12s_gap_ms",
        "proof_12s_gap_hint",
        "root_pipeline_policy_hint",
        "large_input_root_pipeline_gated",
        "stream_commit_residual_ms",
        "AGGREGATE_HEADER",
        "sample_spread_pct",
        "close_samples",
        "max_outlier",
        "dominant_trace_pipeline_action_hint",
        "trace_pipeline_action_consensus",
        "cuda_host_register_wait_ms",
        "cuda_setup_init_calls",
        "cuda_setup_native_init_wait_ms",
        "cuda_memory_info_wait_ms",
        "cuda_allocator_malloc_calls",
        "cuda_allocator_malloc_max_wait_ms",
        "cuda_h2d_bytes",
        "cuda_transfer_action_hint",
        "dominant_cuda_transfer_action_hint",
        "cuda_transfer_action_consensus",
        "dominant_segment_commit_memory_pressure_hint",
        "segment_commit_memory_pressure_consensus",
        "dominant_segment_commit_memory_diagnostic_hint",
        "segment_commit_memory_diagnostic_consensus",
        "copy_summary_gpu_residency_hint",
        "copy_summary_small_d2h_batching_hint",
        "kernel_graph_fusion_priority_hint",
        "kernel_next_action_hint",
        "kernel_graph_fusion_upper_bound_ms",
        "kernel_top_stream_idle_ms",
        "kernel_separation_hint",
        "kernel_top_stream_idle_gap_previous_kernel",
        "kernel_top_stream_idle_gap_next_kernel",
        "kernel_top_stream_idle_gap_calls",
        "kernel_top_stream_idle_gap_ms",
        "kernel_stream_idle_boundary_hint",
        "ncu_top_kernel_separation_hint",
        "perf_lowered_report_row_self_pct",
        "perf_memmove_self_pct",
        "perf_memmove_guest_machine_pct",
        "perf_memmove_trace_slice_pct",
        "perf_memmove_source_hint",
        "perf_pending_segment_drop_self_pct",
        "perf_sha256_self_pct",
        "perf_sha256_source_hint",
        "cpu_trace_hotspot_hint",
        "perf_live_stream_message_self_pct",
        "cpu_trace_live_stream_action_hint",
        "perf_prepare_instruction_self_pct",
        "perf_append_descriptor_self_pct",
        "perf_source_value_self_pct",
        "cpu_trace_lowerer_action_hint",
        "perf_trace_segment_build_self_pct",
        "perf_advance_guest_machine_self_pct",
        "perf_guest_memory_write_self_pct",
        "perf_biguint_modpow_self_pct",
        "perf_guest_memory_read_self_pct",
        "perf_decode_instruction_self_pct",
        "perf_effect_record_memory_write_self_pct",
        "perf_effect_record_memory_read_self_pct",
        "cpu_runner_hotspot_hint",
        "trace_pipeline_action_hint",
        "timing_guest_trace_report_detail_samples",
        "trace_report_detail_sample_hint",
        "trace_report_detail_action_hint",
        "timing_guest_trace_report_validation_ms",
        "trace_report_validation_ms",
        "timing_guest_trace_report_row_validation_ms",
        "trace_report_row_validation_ms",
        "timing_guest_trace_report_source_values_ms",
        "trace_report_source_values_ms",
        "timing_guest_trace_report_source_value_record_ms",
        "trace_report_source_value_record_ms",
        "timing_guest_trace_report_source_value_record_sampled_ns",
        "trace_report_source_value_record_lowerer_share_ms",
        "trace_report_exact_hotspot",
        "trace_report_exact_action_hint",
        "fri_opening_ms",
        "fri_opening_unit_build_scope_pct",
        "fri_opening_layer_tree_nested_pct",
        "fri_opening_query_nested_pct",
        "fri_opening_fold_nested_pct",
        "fri_opening_known_nested_ms",
        "fri_opening_known_nested_pct",
        "fri_opening_unit_build_residual_ms",
        "fri_opening_unit_build_residual_pct",
        "fri_opening_scope_hint",
        "fri_opening_queries",
        "fri_queries_per_unit",
        "timing_finish_fri_transcript_unit_build_ms",
        "timing_finish_fri_transcript_layer_tree_ms",
        "timing_finish_fri_transcript_fold_ms",
        "timing_finish_fri_transcript_unit_count",
        "timing_finish_fri_transcript_layer_count",
        "fri_transcript_unit_build_ms",
        "fri_transcript_layer_tree_ms",
        "fri_transcript_fold_ms",
        "fri_transcript_layers_per_unit",
        "timing_finish_contribution_segment_ms",
        "timing_finish_contribution_verify_ms",
        "timing_finish_contribution_challenge_ms",
        "contribution_segment_ms",
        "contribution_verify_ms",
        "contribution_challenge_ms",
        "contribution_total_ms",
        "fri_opening_total_pct",
        "fri_transcript_unit_build_total_pct",
        "contribution_total_pct",
        "final_proof_timing_hint",
        "timing_framed_guest_input_ms",
        "framed_guest_input_ms",
        "timing_gpu_memory_preflight_ms",
        "gpu_memory_preflight_ms",
        "timing_gpu_setup_ms",
        "gpu_setup_ms",
        "top_level_unattributed_ms",
        "gpu_memory_preflight_pct",
        "gpu_setup_pct",
        "top_level_bottleneck",
    ] {
        assert!(
            source.contains(required),
            "prove timing root summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("prove timing root summary self-test should run");

    assert!(
        output.status.success(),
        "prove timing root summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for required in [
        "profile,input_bytes,total_ms,constant_material_validation_elapsed_ms,constant_material_validation_join_wait_ms,constant_material_validation_overlap_hint,runner_ms,lowerer_ms,trace_lower_ms,trace_runner_lowerer_overlap_ms,trace_lowerer_non_lower_ms,stream_elapsed_ms,stream_worker_ms,segment_commit_ms,segment_commit_initial_workers,segment_commit_effective_workers,segment_commit_oom_retries,segment_commit_attempt_ms,segment_commit_oom_retry_ms,stream_commit_residual_ms,segment_receive_wait_ms,pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,parallel_lower_max_reorder,trace_reports,trace_report_rows,trace_rows_per_report,trace_report_buffer_capacity,trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,trace_report_lifetime_hint,descriptor_rows,descriptor_compact_rows,descriptor_wide_rows,descriptor_upload_bytes,descriptor_bytes_per_row,descriptor_high32_nonzero_values,descriptor_high32_nonzero_rows,descriptor_high32_row_pct,descriptor_high32_a_values,descriptor_high32_b_values,descriptor_high32_c_values,descriptor_high32_a_payload_values,descriptor_high32_b_payload_values,descriptor_high32_store_payload_values,descriptor_high32_store_prev_value_values,descriptor_high32_rows_with_0_fields,descriptor_high32_rows_with_1_fields,descriptor_high32_rows_with_2_fields,descriptor_high32_rows_with_3_fields,descriptor_high32_rows_with_4_fields,descriptor_high32_rows_with_5_fields,descriptor_high32_rows_with_6_fields,descriptor_high32_rows_with_7_fields,descriptor_sparse_high32_estimated_upload_bytes,descriptor_sparse_high32_estimated_upload_savings_pct,descriptor_sparse_high32_high_words,descriptor_sparse_high32_shape_hint,descriptor_shape_hint,seed_direct_lift_attempts,seed_direct_lift_successes,seed_full_advances,finish_opening_ms,opening_query_units,opening_single_query_units,opening_queries,opening_max_queries_per_unit,opening_stage_count,opening_source_shape_hint,opening_row_value_device_rows,opening_row_value_source_rows,opening_row_value_source_extend_ms,opening_row_value_source_extend_pct,opening_source_row_value_action_hint,retained_leaf_openings,retained_leaf_rows,retained_leaf_all_single_row,retained_leaf_path_launches,retained_parent_checkpoint_openings,retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_path_launches,retained_parent_checkpoint_cross_stage_gather_estimated_launches,retained_parent_checkpoint_cross_stage_gather_launch_savings,opening_path_parent_hash_launches_per_stage,opening_row_value_device_download_batches,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings,opening_batching_hint,root_count,materialization_groups,materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,root_pipeline_policy_hint,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,trace_to_leaf_ratio,primary_bottleneck,trace_structure_hint,proof_12s_gap_ms,proof_12s_gap_hint,perf_lowered_report_row_self_pct,perf_memmove_self_pct,perf_memmove_guest_machine_pct,perf_memmove_trace_slice_pct,perf_memmove_source_hint,perf_pending_segment_drop_self_pct,perf_sha256_self_pct,perf_sha256_source_hint,cpu_trace_hotspot_hint",
        "single-root-groups,2758032,9050,0,0,none,7800,7812,0,5700,0,9912,7812,2100,2,2,0,0,0,0,6000,1200,345,2,23,23,23,1,93843537,93917088,1.001,94371840,4194304,528303,0.560,report_buffer_capacity_tight,tight_report_buffer_and_pending_drop,1000,1000,0,88000,88.000,6,4,0.400,1,0,2,0,1,0,2,10,3,2,1,0,1,0,0,72024,18.155,3,sparse_high32_descriptor_candidate,high32_sparse_compact_descriptor,22,22,1,476,23,23,0,0,0,single_query_cross_root_with_no_sources,0,0,0,0.000,none,23,23,yes,276,0,0,no,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,cross_segment_retained_leaf_opening_candidate,23,23,1,1.000,yes,enable_cross_segment_root_pipeline,858,23,874,41078,15732,23598,1786.000,192.974,yes,9.105,stream_elapsed,parallel_lower_waiting,0,within_12s_target,26.350,20.940,10.610,8.670,guest_machine_and_trace_slice,7.410,23.170,sha256_digest_unresolved,report_lifetime_and_data_movement",
        "batched-roots,2758032,9050,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0,0.000,none,none,0,0,0,0,0.000,0,0,0.000,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,none,none,0,0,0,0,0,0,0,0,0,none,0,0,0,0.000,none,0,0,no,0,0,0,no,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,none,23,1,23,23.000,no,root_batches_already_grouped,0,0,0,0,0,0,0.000,0.000,no,0.000,total,none,0,within_12s_target,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "slow-sample,12447640,18100,0,0,none,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,0,0,0.000,none,none,0,0,0,0,0.000,0,0,0.000,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.000,0,none,none,0,0,0,0,0,0,0,0,0,none,0,0,0,0.000,none,0,0,no,0,0,0,no,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,none,120,120,1,1.000,yes,enable_cross_segment_root_pipeline,0,0,0,0,0,0,0.000,0.000,no,0.000,total,none,6100,target_gap_needs_timing_breakdown,0.000,0.000,0.000,0.000,none,0.000,0.000,none,none",
        "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier",
        "aggregate,3,3,9050,12066.667,9050.000,18100,100.000,no,yes",
    ] {
        let required = if required.starts_with("profile,input_bytes,total_ms") {
            "profile,input_bytes,total_ms,catalog_ms"
        } else if required.starts_with("single-root-groups,") {
            "single-root-groups,2758032,9050"
        } else if required.starts_with("batched-roots,") {
            "batched-roots,2758032,9050"
        } else if required.starts_with("slow-sample,") {
            "slow-sample,12447640,18100"
        } else {
            required
        };
        assert!(
            stdout.contains(required),
            "prove timing root summary should print {required}"
        );
    }
}

#[test]
fn prove_timing_root_summary_reports_top_level_proof_phase_fields() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join(format!(
        "../../temp/prove-timing-top-level-phases-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("summary fixture dir should be created");
    let log_path = dir.join("proof.log");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=200",
            "timing_catalog_ms=1",
            "timing_eth_input_ms=2",
            "timing_public_inputs_ms=3",
            "timing_plan_ms=4",
            "timing_framed_guest_input_ms=20",
            "timing_gpu_memory_preflight_ms=100",
            "timing_gpu_setup_ms=100",
            "timing_auxiliary_inputs_ms=5",
            "timing_trace_inputs_ms=6",
            "timing_witness_ms=7",
            "timing_proof_ms=8",
            "timing_output_write_ms=9",
            "timing_summary_ms=10",
        ]
        .join("\n")
            + "\n",
    )
    .expect("summary fixture log should write");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        success,
        "prove timing root summary should pass: stderr={stderr}"
    );
    let mut lines = stdout.lines();
    let header = lines.next().expect("summary should print a header");
    let row = lines.next().expect("summary should print one row");
    let headers = header.split(',').collect::<Vec<_>>();
    let values = row.split(',').collect::<Vec<_>>();
    assert_eq!(
        headers.len(),
        values.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let field = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        values[index]
    };
    assert_eq!(field("framed_guest_input_ms"), "20");
    assert_eq!(field("gpu_memory_preflight_ms"), "100");
    assert_eq!(field("gpu_setup_ms"), "100");
    assert_eq!(field("witness_ms"), "7");
    assert_eq!(field("top_level_unattributed_ms"), "0");
    assert_eq!(field("gpu_memory_preflight_pct"), "50.000");
    assert_eq!(field("gpu_setup_pct"), "50.000");
    assert_eq!(field("top_level_bottleneck"), "gpu_memory_preflight");
}

#[test]
fn prove_timing_root_summary_reports_segment_input_gap_timing() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-segment-input-gap");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("segment input gap fixture dir should be created");
    let log_path = dir.join("segment-input-gap.log");
    let input = [
        "timing_total_ms=200",
        "timing_guest_trace_stream_elapsed_ms=100",
        "timing_guest_segment_commit_ms=20",
        "timing_guest_segment_input_gap_ms=30",
        "timing_guest_segment_input_gap_max_ms=12",
        "timing_guest_segment_input_gap_count=5",
    ]
    .join("\n");
    std::fs::write(&log_path, input).expect("segment input gap fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should finish");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        success,
        "prove timing root summary should parse segment input gap timing: stderr={stderr}"
    );
    let mut lines = stdout.lines();
    let header = lines.next().expect("summary should print a header");
    let row = lines.next().expect("summary should print one row");
    let headers = parse_csv_line(header);
    let values = parse_csv_line(row);
    assert_eq!(
        headers.len(),
        values.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let field = |name: &str| {
        let index = headers
            .iter()
            .position(|header| header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        values[index].as_str()
    };
    assert_eq!(field("segment_input_gap_ms"), "30");
    assert_eq!(field("segment_input_gap_max_ms"), "12");
    assert_eq!(field("segment_input_gap_count"), "5");
    assert_eq!(field("segment_input_gap_avg_ms"), "6.000");
}

#[test]
fn prove_timing_root_summary_reports_source_retention_rebuild_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-source-retention");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("source retention fixture dir should be created");
    let log_path = dir.join("source-retention.log");
    let input = [
        "timing_total_ms=51642",
        "input_bytes=12447640",
        "timing_guest_trace_stream_elapsed_ms=42310",
        "timing_guest_segment_commit_ms=20214",
        "timing_finish_witness_opening_ms=8993",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_retained_source_count=0",
        "timing_finish_witness_opening_external_source_count=240",
        "timing_finish_witness_opening_embedded_source_count=0",
        "timing_finish_witness_opening_missing_source_count=0",
        "timing_guest_stage_source_retention_attempts=240",
        "timing_guest_stage_source_retention_retained=0",
        "timing_guest_stage_source_retention_rejected=240",
        "timing_guest_stage_source_retention_retained_bytes=0",
        "timing_guest_stage_source_retention_rejected_bytes=314069483520",
        "timing_guest_stage_source_retention_max_retained_bytes=0",
        "timing_guest_stage_source_retention_max_rejected_bytes=1308622848",
        "timing_guest_stage_source_retention_limit_bytes=0",
        "timing_guest_stage_source_upload_ms=128",
        "timing_guest_retained_trace_artifact_ms=3",
        "timing_guest_segment_commit_cuda_memory_total_bytes=33711521792",
        "timing_cuda_allocator_copy_h2d_bytes=88120305952",
        "timing_cuda_allocator_copy_h2d_wait_ns=7040040536",
        "timing_cuda_allocator_host_register_wait_ns=1609017316",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");
    std::fs::write(&log_path, input).expect("source retention fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should finish");

    assert!(
        output.status.success(),
        "prove timing root summary should parse source retention input: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines.next().expect("summary should include a header");
    let row = lines.next().expect("summary should include a data row");
    let headers = headers.split(',').collect::<Vec<_>>();
    let row = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| -> &str {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing header {name}: {headers:?}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("missing value for {name}: {row:?}"))
    };

    assert_eq!(value("source_retention_attempts"), "240");
    assert_eq!(value("source_retention_retained"), "0");
    assert_eq!(value("source_retention_rejected"), "240");
    assert_eq!(value("source_retention_retained_bytes"), "0");
    assert_eq!(value("source_retention_rejected_bytes"), "314069483520");
    assert_eq!(value("source_retention_max_retained_bytes"), "0");
    assert_eq!(value("source_retention_max_rejected_bytes"), "1308622848");
    assert_eq!(value("source_retention_limit_bytes"), "0");
    assert_eq!(value("stage_source_upload_ms"), "128");
    assert_eq!(value("retained_trace_artifact_ms"), "3");
    assert_eq!(
        value("source_retention_rejected_total_exceeds_device_memory"),
        "yes"
    );
    assert_eq!(
        value("source_retention_max_rejected_exceeds_device_memory"),
        "no"
    );
    assert_eq!(
        value("opening_source_rebuild_hint"),
        "retained_source_disabled_external_rebuild"
    );
    assert_eq!(
        value("data_residency_action_hint"),
        "source_residency_requires_chunked_design"
    );
}

#[test]
fn prove_timing_root_summary_avoids_increasing_partial_source_residency_beyond_device_memory() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=28713",
        "input_bytes=12447640",
        "timing_guest_trace_runner_ms=22556",
        "timing_guest_trace_lowerer_ms=22758",
        "timing_guest_trace_stream_elapsed_ms=22934",
        "timing_guest_segment_commit_ms=35867",
        "timing_guest_stage_leaf_kernel_work_ms=13078",
        "timing_finish_witness_opening_ms=5461",
        "timing_finish_witness_opening_query_unit_count=477",
        "timing_finish_witness_opening_single_query_unit_count=477",
        "timing_finish_witness_opening_query_count=477",
        "timing_finish_witness_opening_stage_count=477",
        "timing_finish_witness_opening_retained_source_count=12",
        "timing_finish_witness_opening_external_source_count=465",
        "timing_finish_witness_opening_embedded_source_count=0",
        "timing_finish_witness_opening_missing_source_count=0",
        "timing_guest_stage_source_retention_attempts=477",
        "timing_guest_stage_source_retention_retained=12",
        "timing_guest_stage_source_retention_rejected=465",
        "timing_guest_stage_source_retention_retained_bytes=3925868544",
        "timing_guest_stage_source_retention_rejected_bytes=152127406080",
        "timing_guest_stage_source_retention_max_retained_bytes=327155712",
        "timing_guest_stage_source_retention_max_rejected_bytes=327155712",
        "timing_guest_stage_source_retention_limit_bytes=4000000000",
        "timing_guest_segment_commit_cuda_memory_total_bytes=33711521792",
        "timing_cuda_allocator_copy_h2d_bytes=74068771056",
        "timing_cuda_allocator_copy_h2d_wait_ns=1530124000",
        "timing_cuda_allocator_host_register_wait_ns=1513227000",
        "timing_guest_stage_tree_commit_root_count=477",
        "timing_guest_stage_tree_commit_root_materialization_groups=60",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=8",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(
        value("opening_source_rebuild_hint"),
        "partial_retained_source_external_rebuild"
    );
    assert_eq!(
        value("data_residency_action_hint"),
        "source_residency_requires_chunked_design"
    );
}

#[test]
fn prove_timing_root_summary_reads_sibling_nsys_copy_residency_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-copy-residency-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let copy_summary_path = dir.join("sample.copy-summary.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=2758032",
            "timing_total_ms=8250",
            "timing_guest_stage_tree_commit_root_count=23",
            "timing_guest_stage_tree_commit_root_materialization_groups=23",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
            "timing_guest_device_source_descriptor_upload_bytes=8264703744",
            "timing_guest_descriptor_buffer_retention_attempts=23",
            "timing_guest_descriptor_buffer_retention_retained=23",
            "timing_guest_descriptor_buffer_retention_rejected=0",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &copy_summary_path,
        [
            "cuda_transfer_triage",
            "metric,value,detail",
            "h2d_bulk_app_frame_hint,reuse_device_source_for_hot_frame,bytes=369098752 calls=22 app_frame=lzvm_prover::guest_pc_trace_backend::record_device_source_build_duration::hash@/workspace/target/release/lzvm",
        ]
        .join("\n"),
    )
    .expect("copy summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("data_residency_action_hint"),
        "trace_descriptor_residency_pipeline"
    );
    assert_eq!(
        value("copy_summary_h2d_bulk_app_frame_hint"),
        "bytes=369098752_calls=22_app_frame=lzvm_prover::guest_pc_trace_backend::record_device_source_build_duration::hash@/workspace/target/release/lzvm"
    );
}

#[test]
fn prove_timing_root_summary_promotes_copy_residency_hint_when_descriptor_retention_is_active() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-copy-residency-promote-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let copy_summary_path = dir.join("sample.copy-summary.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=2758032",
            "timing_total_ms=8050",
            "timing_guest_stage_tree_commit_root_count=23",
            "timing_guest_stage_tree_commit_root_materialization_groups=23",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
            "timing_guest_device_source_descriptor_upload_bytes=8264703744",
            "timing_guest_descriptor_buffer_retention_attempts=23",
            "timing_guest_descriptor_buffer_retention_retained=23",
            "timing_guest_descriptor_buffer_retention_rejected=0",
            "timing_cuda_allocator_copy_h2d_bytes=8399047121",
            "timing_cuda_allocator_copy_h2d_wait_ns=765604919",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &copy_summary_path,
        [
            "cuda_transfer_triage",
            "metric,value,detail",
            "gpu_residency_hint,prefer_reused_device_residency_for_h2d_inputs,prioritize changes that remove host round trips without changing verifier outputs",
            "h2d_bulk_app_frame_hint,none,no CUDA memcpy callchain frame was available",
        ]
        .join("\n"),
    )
    .expect("copy summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("cuda_transfer_action_hint"),
        "initial_descriptor_upload_retention_active"
    );
    assert_eq!(
        value("copy_summary_gpu_residency_hint"),
        "prefer_reused_device_residency_for_h2d_inputs"
    );
    assert_eq!(
        value("data_residency_action_hint"),
        "prefer_reused_device_residency_for_h2d_inputs"
    );
}

#[test]
fn prove_timing_root_summary_reads_sibling_nsys_copy_small_d2h_hints() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-copy-d2h-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let copy_summary_path = dir.join("sample.copy-summary.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=12447640",
            "timing_total_ms=55693",
            "timing_guest_stage_tree_commit_root_count=120",
            "timing_guest_stage_tree_commit_root_materialization_groups=120",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
            "timing_cuda_allocator_copy_h2d_bytes=88120305500",
            "timing_cuda_allocator_copy_h2d_wait_ns=2883439000",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &copy_summary_path,
        [
            "cuda_transfer_triage",
            "metric,value,detail",
            "gpu_residency_hint,batch_or_keep_small_d2h_on_device,prioritize data residency before relying on Graph or fusion speedups",
            "small_d2h_batching_hint,batch_small_d2h_by_size,bytes=1152 calls=41 host_api_ms=3387.322 previous_kernel=poseidon2_merkle_digest_parent_kernel",
            "cuda_api_backtrace_hint",
            "missing_callchain_calls,missing_host_api_ms,recommended_nsys_options",
            "1182,626.112,--trace=cuda,nvtx,osrt --sample=process-tree --cudabacktrace=memory:80000",
        ]
        .join("\n"),
    )
    .expect("copy summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("copy_summary_gpu_residency_hint"),
        "batch_or_keep_small_d2h_on_device"
    );
    assert_eq!(
        value("copy_summary_small_d2h_batching_hint"),
        "batch_small_d2h_by_size"
    );
    assert_eq!(
        value("copy_summary_cuda_api_backtrace_hint"),
        "--trace=cuda|nvtx|osrt_--sample=process-tree_--cudabacktrace=memory:80000"
    );
}

#[test]
fn prove_timing_root_summary_reads_hyphen_sibling_nsys_copy_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-hyphen-copy-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let copy_summary_path = dir.join("sample-copy-summary.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=2758032",
            "timing_total_ms=8250",
            "timing_guest_stage_tree_commit_root_count=23",
            "timing_guest_stage_tree_commit_root_materialization_groups=23",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &copy_summary_path,
        [
            "cuda_transfer_triage",
            "metric,value,detail",
            "gpu_residency_hint,batch_or_keep_small_d2h_on_device,prioritize data residency before relying on Graph or fusion speedups",
        ]
        .join("\n"),
    )
    .expect("copy summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let index = headers
        .iter()
        .position(|header| *header == "copy_summary_gpu_residency_hint")
        .unwrap_or_else(|| panic!("summary should expose copy hint: stdout={stdout}"));
    assert_eq!(
        row.get(index),
        Some(&"batch_or_keep_small_d2h_on_device"),
        "hyphen sibling copy summary should feed root summary: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reads_explicit_nsys_copy_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-explicit-copy-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let copy_summary_path = dir.join("detached-copy-report.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=12447640",
            "timing_total_ms=55693",
            "timing_guest_stage_tree_commit_root_count=120",
            "timing_guest_stage_tree_commit_root_materialization_groups=120",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
            "timing_cuda_allocator_copy_h2d_bytes=88120305500",
            "timing_cuda_allocator_copy_h2d_wait_ns=2883439000",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &copy_summary_path,
        [
            "cuda_transfer_triage",
            "metric,value,detail",
            "gpu_residency_hint,batch_or_keep_small_d2h_on_device,explicit report should be merged",
        ]
        .join("\n"),
    )
    .expect("copy summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--nsys-copy-summary")
        .arg(&copy_summary_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let index = headers
        .iter()
        .position(|header| *header == "copy_summary_gpu_residency_hint")
        .unwrap_or_else(|| panic!("summary should expose copy hint: stdout={stdout}"));
    assert_eq!(
        row.get(index),
        Some(&"batch_or_keep_small_d2h_on_device"),
        "explicit copy summary should feed root summary: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reads_explicit_nsys_kernel_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-explicit-kernel-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let kernel_summary_path = dir.join("detached-kernel-report.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=12447640",
            "timing_total_ms=55693",
            "timing_guest_stage_tree_commit_root_count=120",
            "timing_guest_stage_tree_commit_root_materialization_groups=120",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &kernel_summary_path,
        [
            "stream_idle_gap_hotspots",
            "previous_kernel,next_kernel,calls,idle_gap_ms,max_idle_gap_ms",
            "\"poseidon2_merkle_digest_parent_kernel<16, 4>\",\"trace_descriptor_expand_kernel<8, 2>\",120,22171.505,1776.126",
            "",
            "cuda_graph_fusion_separation_triage",
            "metric,value,detail",
            "graph_or_fusion_upper_bound_ms,863.000,launch API time before synchronization or transfer costs",
            "top_stream_idle_ms,2500.000,active-window time not covered by kernels on the top stream",
            "next_action_hint,inspect_stream_idle_or_cpu_producer,top kernel stream is idle for more than a quarter of its active window",
            "graph_fusion_priority_hint,defer_graph_or_fusion_until_stream_idle_is_explained,top stream idle exceeds launch upper bound",
            "kernel_separation_hint,use_ncu_occupancy_before_splitting,profile top kernels with NCU before splitting kernels",
        ]
        .join("\n"),
    )
    .expect("kernel summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--nsys-kernel-summary")
        .arg(&kernel_summary_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = parse_csv_line(lines.next().expect("summary should include a header"));
    let row = parse_csv_line(lines.next().expect("summary should include a data row"));
    assert_eq!(
        headers.len(),
        row.len(),
        "summary CSV header and row should stay aligned: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("kernel_graph_fusion_priority_hint"),
        "defer_graph_or_fusion_until_stream_idle_is_explained"
    );
    assert_eq!(
        value("kernel_next_action_hint"),
        "inspect_stream_idle_or_cpu_producer"
    );
    assert_eq!(value("kernel_graph_fusion_upper_bound_ms"), "863.000");
    assert_eq!(value("kernel_top_stream_idle_ms"), "2500.000");
    assert_eq!(
        value("kernel_separation_hint"),
        "use_ncu_occupancy_before_splitting"
    );
    assert_eq!(
        value("kernel_top_stream_idle_gap_previous_kernel"),
        "poseidon2_merkle_digest_parent_kernel<16, 4>"
    );
    assert_eq!(
        value("kernel_top_stream_idle_gap_next_kernel"),
        "trace_descriptor_expand_kernel<8, 2>"
    );
    assert_eq!(value("kernel_top_stream_idle_gap_calls"), "120");
    assert_eq!(value("kernel_top_stream_idle_gap_ms"), "22171.505");
    assert_eq!(
        value("kernel_stream_idle_boundary_hint"),
        "commit_root_to_trace_descriptor_idle"
    );
}

#[test]
fn prove_timing_root_summary_reads_sibling_nsys_kernel_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-sibling-kernel-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let kernel_summary_path = dir.join("sample-kernel-summary.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=12447640",
            "timing_total_ms=55693",
            "timing_guest_stage_tree_commit_root_count=120",
            "timing_guest_stage_tree_commit_root_materialization_groups=120",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &kernel_summary_path,
        [
            "stream_idle_gap_hotspots",
            "previous_kernel,next_kernel,calls,idle_gap_ms,max_idle_gap_ms",
            "poseidon2_merkle_digest_parent_kernel,trace_descriptor_expand_kernel,120,22171.505,1776.126",
            "",
            "cuda_graph_fusion_separation_triage",
            "metric,value,detail",
            "graph_or_fusion_upper_bound_ms,863.000,launch API time before synchronization or transfer costs",
            "top_stream_idle_ms,2500.000,active-window time not covered by kernels on the top stream",
            "next_action_hint,inspect_stream_idle_or_cpu_producer,top kernel stream is idle for more than a quarter of its active window",
            "graph_fusion_priority_hint,defer_graph_or_fusion_until_stream_idle_is_explained,top stream idle exceeds launch upper bound",
            "kernel_separation_hint,use_ncu_occupancy_before_splitting,profile top kernels with NCU before splitting kernels",
        ]
        .join("\n"),
    )
    .expect("kernel summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("kernel_graph_fusion_priority_hint"),
        "defer_graph_or_fusion_until_stream_idle_is_explained"
    );
    assert_eq!(
        value("kernel_next_action_hint"),
        "inspect_stream_idle_or_cpu_producer"
    );
    assert_eq!(value("kernel_graph_fusion_upper_bound_ms"), "863.000");
    assert_eq!(value("kernel_top_stream_idle_ms"), "2500.000");
    assert_eq!(
        value("kernel_separation_hint"),
        "use_ncu_occupancy_before_splitting"
    );
    assert_eq!(
        value("kernel_top_stream_idle_gap_previous_kernel"),
        "poseidon2_merkle_digest_parent_kernel"
    );
    assert_eq!(
        value("kernel_top_stream_idle_gap_next_kernel"),
        "trace_descriptor_expand_kernel"
    );
    assert_eq!(value("kernel_top_stream_idle_gap_calls"), "120");
    assert_eq!(value("kernel_top_stream_idle_gap_ms"), "22171.505");
    assert_eq!(
        value("kernel_stream_idle_boundary_hint"),
        "commit_root_to_trace_descriptor_idle"
    );
}

#[test]
fn prove_timing_root_summary_reads_explicit_ncu_kernel_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-explicit-ncu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let ncu_summary_path = dir.join("detached-ncu-report.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=12447640",
            "timing_total_ms=55693",
            "timing_guest_stage_tree_commit_root_count=120",
            "timing_guest_stage_tree_commit_root_materialization_groups=120",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &ncu_summary_path,
        [
            "profile=self-test",
            "",
            "metric_collection_quality",
            "metric,value,detail",
            "collection_hint,duration_and_throughput_metrics,usable throughput rows",
            "duration_profiles,3,3 of 3 kernel rows carried duration metrics",
            "",
            "kernel_metric_summary",
            "kernel,profiles,duration_ms,avg_duration_us,sm_throughput_pct,dram_throughput_pct,compute_memory_pct,issue_active_pct,active_warps_pct,registers_per_thread,shared_mem_kb_per_block",
            "\"poseidon2_merkle_digest_parent_kernel<16, 4>\",2,0.100,50.000,63.000,59.000,59.000,55.000,90.000,38.000,1.104",
            "poseidon2_width16_merkle_parent_kernel,1,0.020,20.000,35.000,15.000,18.000,20.000,42.000,64.000,2.000",
            "",
            "occupancy_limits",
            "kernel,profiles,register_limit_blocks,shared_mem_limit_blocks,warp_limit_blocks,block_limit_blocks,registers_per_thread,shared_mem_kb_per_block,limiting_factors",
            "\"poseidon2_merkle_digest_parent_kernel<16, 4>\",2,6.000,14.000,6.000,24.000,38.000,1.104,register_limited|warp_limited",
            "",
            "kernel_separation_candidates",
            "kernel,profiles,duration_ms,registers_per_thread,register_limit_blocks,warp_limit_blocks,shared_mem_limit_blocks,sm_throughput_pct,issue_active_pct,separation_hint",
            "\"poseidon2_merkle_digest_parent_kernel<16, 4>\",2,0.100,38.000,6.000,6.000,14.000,63.000,55.000,kernel_time_secondary",
            "",
            "descriptor_expansion_shape_candidates",
            "kernel,profiles,duration_ms,dram_throughput_pct,sm_throughput_pct,issue_active_pct,registers_per_thread,descriptor_hint",
            "expand_zisk_main_trace_descriptors_kernel,1,4.662,35.193,4.009,0.394,40.000,redesign_descriptor_fields_before_kernel_split",
        ]
        .join("\n"),
    )
    .expect("NCU summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--ncu-kernel-summary")
        .arg(&ncu_summary_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("ncu_metric_collection_hint"),
        "duration_and_throughput_metrics"
    );
    assert_eq!(
        value("ncu_top_kernel"),
        "poseidon2_merkle_digest_parent_kernel<16|_4>"
    );
    assert_eq!(value("ncu_top_kernel_duration_ms"), "0.100");
    assert_eq!(
        value("ncu_top_kernel_limiting_factors"),
        "register_limited|warp_limited"
    );
    assert_eq!(
        value("ncu_top_kernel_separation_hint"),
        "kernel_time_secondary"
    );
    assert_eq!(
        value("ncu_descriptor_expansion_hint"),
        "redesign_descriptor_fields_before_kernel_split"
    );
}

#[test]
fn prove_timing_root_summary_reads_sibling_ncu_kernel_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under workspace root");
    let script_path = workspace_root.join("scripts/prove-timing-root-summary.py");
    let dir = workspace_root.join("temp").join(format!(
        "prove-timing-root-summary-sibling-ncu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("sample.log");
    let ncu_summary_path = dir.join("sample.ncu-ntt-summary.txt");
    std::fs::write(
        &log_path,
        [
            "input_bytes=12447640",
            "timing_total_ms=55693",
            "timing_guest_stage_tree_commit_root_count=120",
            "timing_guest_stage_tree_commit_root_materialization_groups=120",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing fixture should be written");
    std::fs::write(
        &ncu_summary_path,
        [
            "profile=self-test",
            "",
            "metric_collection_quality",
            "metric,value,detail",
            "collection_hint,duration_and_throughput_metrics,usable throughput rows",
            "duration_profiles,3,3 of 3 kernel rows carried duration metrics",
            "",
            "kernel_metric_summary",
            "kernel,profiles,duration_ms,avg_duration_us,sm_throughput_pct,dram_throughput_pct,compute_memory_pct,issue_active_pct,active_warps_pct,registers_per_thread,shared_mem_kb_per_block",
            "ntt_stage_kernel,2,0.100,50.000,63.000,59.000,59.000,55.000,90.000,38.000,1.104",
            "poseidon2_width16_merkle_parent_kernel,1,0.020,20.000,35.000,15.000,18.000,20.000,42.000,64.000,2.000",
            "",
            "occupancy_limits",
            "kernel,profiles,register_limit_blocks,shared_mem_limit_blocks,warp_limit_blocks,block_limit_blocks,registers_per_thread,shared_mem_kb_per_block,limiting_factors",
            "ntt_stage_kernel,2,6.000,14.000,6.000,24.000,38.000,1.104,register_limited|warp_limited",
            "",
            "kernel_separation_candidates",
            "kernel,profiles,duration_ms,registers_per_thread,register_limit_blocks,warp_limit_blocks,shared_mem_limit_blocks,sm_throughput_pct,issue_active_pct,separation_hint",
            "ntt_stage_kernel,2,0.100,38.000,6.000,6.000,14.000,63.000,55.000,kernel_time_secondary",
        ]
        .join("\n"),
    )
    .expect("NCU summary fixture should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should include a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should include a data row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("ncu_metric_collection_hint"),
        "duration_and_throughput_metrics"
    );
    assert_eq!(value("ncu_top_kernel"), "ntt_stage_kernel");
    assert_eq!(value("ncu_top_kernel_duration_ms"), "0.100");
    assert_eq!(
        value("ncu_top_kernel_limiting_factors"),
        "register_limited|warp_limited"
    );
    assert_eq!(
        value("ncu_top_kernel_separation_hint"),
        "kernel_time_secondary"
    );
}

#[test]
fn prove_timing_root_summary_reports_seed_direct_lift_miss_reasons() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_seed_direct_lift_attempts=8",
        "timing_guest_trace_seed_direct_lift_successes=2",
        "timing_guest_trace_seed_direct_lift_empty_segments=1",
        "timing_guest_trace_seed_direct_lift_pending_dma_single_reports=2",
        "timing_guest_trace_seed_direct_lift_amo_boundaries=3",
        "timing_guest_trace_seed_direct_lift_store_conditional_boundaries=4",
        "timing_guest_trace_seed_direct_lift_dma_prepare_missing_lookaheads=5",
        "timing_guest_trace_seed_direct_lift_boundary_c_unavailable=6",
        "timing_guest_trace_seed_direct_lift_ms=7",
        "timing_guest_trace_seed_full_advance_ms=123",
        "timing_guest_trace_seed_full_advances=6",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("seed_direct_lift_empty_segments"), "1");
    assert_eq!(value("seed_direct_lift_success_pct"), "25.000");
    assert_eq!(
        value("seed_direct_lift_dominant_miss_reason"),
        "boundary_c_unavailable"
    );
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "profile_boundary_c_unavailable"
    );
    assert_eq!(value("seed_direct_lift_pending_dma_single_reports"), "2");
    assert_eq!(value("seed_direct_lift_amo_boundaries"), "3");
    assert_eq!(value("seed_direct_lift_store_conditional_boundaries"), "4");
    assert_eq!(
        value("seed_direct_lift_dma_prepare_missing_lookaheads"),
        "5"
    );
    assert_eq!(value("seed_direct_lift_boundary_c_unavailable"), "6");
    assert_eq!(value("seed_direct_lift_ms"), "7");
    assert_eq!(value("seed_full_advance_ms"), "123");
}

#[test]
fn prove_timing_root_summary_reports_trace_report_detail_sample_coverage() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lowerer_ms=2000",
        "timing_guest_trace_lower_ms=1500",
        "timing_guest_trace_report_ms=1500",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_report_detail_samples=10",
        "timing_guest_trace_report_sampled_ns=1000",
        "timing_guest_trace_report_lowering_sampled_ns=150",
        "timing_guest_trace_report_row_validation_sampled_ns=500",
        "timing_guest_trace_report_row_validation_timer_bookkeeping_sampled_ns=50",
        "timing_guest_trace_report_memory_columns_sampled_ns=60",
        "timing_guest_trace_report_source_values_sampled_ns=180",
        "timing_guest_trace_report_source_a_value_sampled_ns=140",
        "timing_guest_trace_report_source_b_value_sampled_ns=90",
        "timing_guest_trace_report_source_value_record_sampled_ns=40",
        "timing_guest_trace_report_register_access_sampled_ns=100",
        "timing_guest_trace_report_memory_access_sampled_ns=80",
        "timing_guest_trace_report_precompile_memory_sampled_ns=20",
        "timing_guest_trace_report_visit_sampled_ns=200",
        "timing_guest_trace_descriptor_sampled_ns=50",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "trace_report_detail_samples,trace_report_detail_sample_pct,trace_report_detail_sample_ppm,trace_report_detail_sample_hint,trace_report_detail_avg_ns,trace_report_detail_lowerer_share_ms,trace_report_row_validation_lowerer_share_ms,trace_report_memory_columns_lowerer_share_ms,trace_report_source_values_lowerer_share_ms,trace_report_source_lookup_lowerer_share_ms,trace_report_source_value_record_lowerer_share_ms,trace_report_source_values_residual_lowerer_share_ms,trace_report_precompile_memory_lowerer_share_ms,trace_report_instruction_result_lowerer_share_ms,trace_report_next_pc_lowerer_share_ms,trace_report_register_access_lowerer_share_ms,trace_report_memory_access_lowerer_share_ms,trace_report_store_apply_lowerer_share_ms,trace_report_row_validation_timer_bookkeeping_lowerer_share_ms,trace_report_row_validation_residual_lowerer_share_ms,trace_report_visit_lowerer_share_ms,trace_report_descriptor_lowerer_share_ms,trace_report_detail_hotspot,trace_report_detail_hotspot_pct,trace_report_detail_action_hint,trace_report_row_validation_hotspot,trace_report_row_validation_hotspot_pct,trace_report_row_validation_explained_pct,trace_report_row_validation_residual_pct,trace_report_source_values_lookup_pct,trace_report_source_values_record_pct,trace_report_source_values_residual_pct,source_immediate_reads,source_immediate_read_pct,source_register_reads,source_register_read_pct,source_memory_reads,source_memory_read_pct,source_indirect_reads,source_indirect_read_pct,source_last_c_reads,source_last_c_read_pct,trace_report_source_kind_hotspot,trace_report_source_kind_hotspot_pct,trace_report_source_kind_coverage_pct,trace_report_source_kind_residual_pct,trace_report_detail_visit_pct,trace_report_visit_descriptor_pct,trace_report_visit_residual_pct"
        ),
        "prove timing root summary should expose detail sample, lowerer-share cost, source-value, and visit drilldown columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",none,0.000,use_sampled_detail_breakdown,10,1.000,10000.000,detail_timing_sampled,"),
        "prove timing root summary should route missing exact detail fields to sampled detail breakdown: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",10,1.000,10000.000,detail_timing_sampled,100,1500.000,750.000,90.000,270.000,345.000,60.000,0.000,30.000,0.000,0.000,150.000,120.000,0.000,75.000,0.000,300.000,75.000,row_validation,50.000,profile_row_validation,source_a_value,28.000,100.000,0.000,127.778,22.222,0.000,0,0.000,0,0.000,0,0.000,0,0.000,0,0.000,none,0.000,0.000,100.000,20.000,25.000,75.000"
        ),
        "prove timing root summary should classify sampled detail, scale costs by actual trace lower work, source-value lookup coverage, row-validation, and visit hotspots: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "trace_report_source_value_record_ns_per_row,trace_report_source_values_residual_ns_per_row,trace_report_row_validation_timer_bookkeeping_ns_per_row,trace_report_row_validation_residual_ns_per_row,trace_report_visit_residual_ns_per_row,trace_report_descriptor_ns_per_row"
        ),
        "prove timing root summary should expose per-row residual and descriptor costs: stdout={stdout}"
    );
    assert!(
        stdout.contains(",60000.000,0.000,75000.000,0.000,225000.000,75000.000"),
        "prove timing root summary should scale residual and descriptor costs to ns per trace row: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_source_value_kind_detail() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=1800",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_report_detail_samples=10",
        "timing_guest_trace_report_sampled_ns=2000",
        "timing_guest_trace_report_source_values_sampled_ns=1800",
        "timing_guest_trace_report_source_immediate_reads=2",
        "timing_guest_trace_report_source_register_reads=4",
        "timing_guest_trace_report_source_memory_reads=1",
        "timing_guest_trace_report_source_indirect_reads=3",
        "timing_guest_trace_report_source_last_c_reads=0",
        "timing_guest_trace_report_source_immediate_read_sampled_ns=100",
        "timing_guest_trace_report_source_register_read_sampled_ns=400",
        "timing_guest_trace_report_source_memory_read_sampled_ns=200",
        "timing_guest_trace_report_source_indirect_read_sampled_ns=900",
        "timing_guest_trace_report_source_last_c_read_sampled_ns=0",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should have a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should have a row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| -> &str {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing column {name}: stdout={stdout}"));
        row[index]
    };

    assert_eq!(value("source_immediate_reads"), "2");
    assert_eq!(value("source_immediate_read_pct"), "20.000");
    assert_eq!(value("source_register_reads"), "4");
    assert_eq!(value("source_register_read_pct"), "40.000");
    assert_eq!(value("source_memory_reads"), "1");
    assert_eq!(value("source_memory_read_pct"), "10.000");
    assert_eq!(value("source_indirect_reads"), "3");
    assert_eq!(value("source_indirect_read_pct"), "30.000");
    assert_eq!(value("source_last_c_reads"), "0");
    assert_eq!(value("source_last_c_read_pct"), "0.000");
    assert_eq!(value("trace_report_source_kind_hotspot"), "indirect_read");
    assert_eq!(value("trace_report_source_kind_hotspot_pct"), "50.000");
    assert_eq!(value("trace_report_source_kind_coverage_pct"), "88.889");
    assert_eq!(value("trace_report_source_kind_residual_pct"), "11.111");
}

#[test]
fn prove_timing_root_summary_reports_trace_report_layout_breakdown() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_reports=67108864",
        "timing_guest_trace_report_record_size_bytes=192",
        "timing_guest_trace_report_instruction_size_bytes=16",
        "timing_guest_trace_report_register_write_list_size_bytes=32",
        "timing_guest_trace_report_memory_access_list_size_bytes=80",
        "timing_guest_trace_report_precompile_access_list_size_bytes=24",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_report_instruction_size_bytes"), "16");
    assert_eq!(value("trace_report_register_write_list_size_bytes"), "32");
    assert_eq!(value("trace_report_memory_access_list_size_bytes"), "80");
    assert_eq!(
        value("trace_report_precompile_access_list_size_bytes"),
        "24"
    );
    assert_eq!(value("trace_report_instruction_storage_gib"), "1.000");
    assert_eq!(
        value("trace_report_register_write_list_storage_gib"),
        "2.000"
    );
    assert_eq!(
        value("trace_report_memory_access_list_storage_gib"),
        "5.000"
    );
    assert_eq!(
        value("trace_report_precompile_access_list_storage_gib"),
        "1.500"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_lower_work_and_wall_overlap() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=2758032",
        "timing_total_ms=9900",
        "timing_guest_trace_runner_ms=7800",
        "timing_guest_trace_lowerer_ms=7812",
        "timing_guest_trace_lower_ms=6200",
        "timing_guest_trace_report_ms=6100",
        "timing_guest_trace_stream_elapsed_ms=9912",
        "timing_guest_trace_stream_ms=7812",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = parse_csv_line(lines.next().expect("summary should print a header"));
    let row = parse_csv_line(lines.next().expect("summary should print one row"));
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("runner_ms"), "7800");
    assert_eq!(value("lowerer_ms"), "7812");
    assert_eq!(value("trace_lower_ms"), "6200");
    assert_eq!(value("trace_report_ms"), "6100");
    assert_eq!(value("trace_non_report_ms"), "100");
    assert_eq!(value("trace_runner_lowerer_overlap_ms"), "5700");
    assert_eq!(value("trace_lowerer_non_lower_ms"), "1612");
    assert_eq!(value("stream_elapsed_ms"), "9912");
}

#[test]
fn prove_timing_root_summary_reports_trace_report_exact_breakdown() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=2758032",
        "timing_total_ms=9900",
        "timing_guest_trace_lower_ms=1200",
        "timing_guest_trace_report_ms=1000",
        "timing_guest_trace_report_validation_ms=900",
        "timing_guest_trace_emit_ms=70",
        "timing_guest_trace_descriptor_ms=40",
        "timing_guest_trace_report_lowering_ms=80",
        "timing_guest_trace_report_row_validation_ms=620",
        "timing_guest_trace_report_memory_columns_ms=120",
        "timing_guest_trace_report_source_values_ms=240",
        "timing_guest_trace_report_source_a_value_ms=160",
        "timing_guest_trace_report_source_b_value_ms=60",
        "timing_guest_trace_report_precompile_memory_ms=10",
        "timing_guest_trace_report_instruction_result_ms=30",
        "timing_guest_trace_report_next_pc_ms=20",
        "timing_guest_trace_report_register_access_ms=50",
        "timing_guest_trace_report_memory_access_ms=40",
        "timing_guest_trace_report_store_apply_ms=30",
        "timing_guest_trace_report_visit_ms=140",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_report_validation_ms"), "900");
    assert_eq!(value("trace_report_emit_ms"), "70");
    assert_eq!(value("trace_report_lowering_ms"), "80");
    assert_eq!(value("trace_report_row_validation_ms"), "620");
    assert_eq!(value("trace_report_memory_columns_ms"), "120");
    assert_eq!(value("trace_report_source_values_ms"), "240");
    assert_eq!(value("trace_report_source_a_value_ms"), "160");
    assert_eq!(value("trace_report_source_b_value_ms"), "60");
    assert_eq!(value("trace_report_precompile_memory_ms"), "10");
    assert_eq!(value("trace_report_instruction_result_ms"), "30");
    assert_eq!(value("trace_report_next_pc_ms"), "20");
    assert_eq!(value("trace_report_register_access_ms"), "50");
    assert_eq!(value("trace_report_memory_access_ms"), "40");
    assert_eq!(value("trace_report_store_apply_ms"), "30");
    assert_eq!(value("trace_report_visit_ms"), "140");
    assert_eq!(value("trace_report_exact_hotspot"), "row_validation");
    assert_eq!(value("trace_report_exact_hotspot_pct"), "62.000");
    assert_eq!(
        value("trace_report_exact_action_hint"),
        "profile_row_validation"
    );
}

#[test]
fn prove_timing_root_summary_ignores_partial_sampled_exact_breakdown() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=62000",
        "timing_guest_trace_lower_ms=45000",
        "timing_guest_trace_report_ms=44850",
        "timing_guest_trace_reports=500000000",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_report_detail_samples=5000",
        "timing_guest_trace_report_sampled_ns=7100000",
        "timing_guest_trace_report_row_validation_sampled_ns=4300000",
        "timing_guest_trace_report_source_values_sampled_ns=1600000",
        "timing_guest_trace_report_source_a_value_sampled_ns=1",
        "timing_guest_trace_report_source_b_value_sampled_ns=0",
        "timing_guest_trace_report_row_validation_ms=4",
        "timing_guest_trace_report_source_values_ms=1",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_report_exact_hotspot"), "none");
    assert_eq!(value("trace_report_exact_hotspot_pct"), "0.000");
    assert_eq!(
        value("trace_report_exact_action_hint"),
        "use_sampled_detail_breakdown"
    );
    assert_eq!(value("trace_report_detail_hotspot"), "row_validation");
    assert_eq!(
        value("trace_report_detail_action_hint"),
        "enable_shape_timing_for_row_validation_residual"
    );
}

#[test]
fn prove_timing_root_summary_classifies_trace_pipeline_action() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52000",
        "timing_guest_trace_runner_ms=41000",
        "timing_guest_trace_lowerer_ms=35000",
        "timing_guest_trace_lower_ms=33000",
        "timing_guest_trace_stream_elapsed_ms=43000",
        "timing_guest_trace_stream_ms=22000",
        "timing_guest_segment_commit_ms=21000",
        "timing_guest_trace_segment_receive_wait_ms=22000",
        "timing_guest_trace_pending_receive_wait_ms=1000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let hint_index = headers
        .iter()
        .position(|header| *header == "trace_pipeline_action_hint")
        .expect("summary should expose trace pipeline action hint");
    assert_eq!(
        fields.get(hint_index),
        Some(&"trace_generation_and_commit_pipeline_candidate"),
        "trace-heavy run with a large commit gate should point at the combined pipeline lever: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_source_row_value_extend_priority() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52000",
        "timing_guest_trace_runner_ms=41000",
        "timing_guest_trace_lowerer_ms=35000",
        "timing_guest_trace_lower_ms=33000",
        "timing_guest_trace_stream_elapsed_ms=43000",
        "timing_guest_trace_stream_ms=22000",
        "timing_guest_segment_commit_ms=21000",
        "timing_guest_trace_segment_receive_wait_ms=22000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_external_source_count=120",
        "timing_finish_witness_opening_embedded_source_count=120",
        "timing_finish_witness_opening_row_values_source_rows=77",
        "timing_finish_witness_opening_row_value_source_extend_ms=1134",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("opening_row_value_source_extend_ms"), "1134");
    assert_eq!(value("opening_row_value_source_extend_pct"), "2.181");
    assert_eq!(
        value("opening_source_row_value_action_hint"),
        "trace_pipeline_before_source_row_values"
    );
}

#[test]
fn prove_timing_root_summary_reports_external_source_row_value_boundary_without_d2h() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=29054",
        "timing_guest_trace_runner_ms=23254",
        "timing_guest_trace_lowerer_ms=23289",
        "timing_guest_trace_lower_ms=27703",
        "timing_guest_trace_stream_elapsed_ms=23423",
        "timing_guest_trace_stream_ms=8364",
        "timing_guest_segment_commit_ms=15058",
        "timing_guest_trace_segment_receive_wait_ms=8362",
        "timing_guest_trace_pending_receive_wait_ms=23251",
        "timing_guest_stage_tree_commit_root_count=477",
        "timing_guest_stage_tree_commit_root_materialization_groups=60",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=8",
        "timing_finish_witness_opening_query_unit_count=477",
        "timing_finish_witness_opening_single_query_unit_count=477",
        "timing_finish_witness_opening_query_count=477",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_external_source_count=477",
        "timing_finish_witness_opening_row_values_source_rows=1866",
        "timing_finish_witness_opening_row_value_source_extend_ms=2832",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("opening_external_source_boundary_hint"),
        "external_source_unit_boundary_blocks_row_value_batch"
    );
    assert_eq!(
        value("opening_source_row_value_action_hint"),
        "profile_external_source_row_value_rebuilds"
    );
}

#[test]
fn prove_timing_root_summary_aggregates_trace_pipeline_action() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-aggregate-action");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_trace_runner_ms=41000".to_owned(),
            "timing_guest_trace_lowerer_ms=35000".to_owned(),
            "timing_guest_trace_lower_ms=33000".to_owned(),
            "timing_guest_trace_stream_elapsed_ms=43000".to_owned(),
            "timing_guest_trace_stream_ms=22000".to_owned(),
            "timing_guest_segment_commit_ms=21000".to_owned(),
            "timing_guest_trace_segment_receive_wait_ms=22000".to_owned(),
            "timing_guest_trace_pending_receive_wait_ms=1000".to_owned(),
            "timing_guest_trace_parallel_lower_workers=2".to_owned(),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let paths = [52000_u64, 52100, 51950]
        .into_iter()
        .enumerate()
        .map(|(index, total_ms)| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms)).expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "dominant_trace_pipeline_action_hint,trace_pipeline_action_consensus,dominant_trace_structure_hint,trace_structure_consensus"
        ),
        "aggregate summary should expose cross-sample action hint stability: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate,3,3,51950,52016.667,52000.000,52100,0.288,yes,no,trace_generation_and_commit_pipeline_candidate,yes,parallel_lower_waiting,yes"
        ),
        "aggregate row should report the dominant trace pipeline action, trace structure, and consensus: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_excludes_diagnostic_shape_profiles_from_aggregate() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-aggregate-diagnostic-shape");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64, shape_profile: bool| {
        let mut lines = vec![
            "input_bytes=2758032".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_trace_report_rows=1000".to_owned(),
            "timing_guest_stage_tree_commit_root_count=23".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=23".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ];
        if shape_profile {
            lines.push("timing_guest_trace_external_op_rows=500".to_owned());
        }
        lines.join("\n")
    };
    let fixtures = [
        (8_000_u64, false),
        (8_100, false),
        (8_200, false),
        (20_000, true),
    ];
    let paths = fixtures
        .into_iter()
        .enumerate()
        .map(|(index, (total_ms, shape_profile))| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms, shape_profile))
                .expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let profile_headers = lines[0].split(',').collect::<Vec<_>>();
    let diagnostic_row = lines[4].split(',').collect::<Vec<_>>();
    let profile_hint_index = profile_headers
        .iter()
        .position(|header| *header == "trace_shape_profile_hint")
        .expect("summary should expose trace shape profile hint");
    assert_eq!(
        diagnostic_row.get(profile_hint_index),
        Some(&"diagnostic_only_shape_profile"),
        "shape-profile sample should be tagged before aggregation: stdout={stdout}"
    );

    let aggregate_headers = lines[5].split(',').collect::<Vec<_>>();
    let aggregate_fields = lines[6].split(',').collect::<Vec<_>>();
    let aggregate_value = |name: &str| {
        let index = aggregate_headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("aggregate should expose {name}: stdout={stdout}"));
        aggregate_fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("aggregate row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(aggregate_value("total_count"), "4");
    assert_eq!(aggregate_value("valid_total_count"), "3");
    assert_eq!(aggregate_value("total_mean_ms"), "8100.000");
    assert_eq!(aggregate_value("total_max_ms"), "8200");
    assert_eq!(aggregate_value("close_samples"), "yes");
}

#[test]
fn prove_timing_root_summary_aggregates_key_profile_means() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-profile-mean-aggregate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64,
                  witness_ms: u64,
                  runner_ms: u64,
                  row_value_source_extend_ms: u64,
                  host_register_wait_ns: u64,
                  h2d_bytes: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            format!("timing_witness_ms={witness_ms}"),
            format!("timing_guest_trace_runner_ms={runner_ms}"),
            format!("timing_finish_witness_opening_row_value_source_extend_ms={row_value_source_extend_ms}"),
            format!("timing_cuda_allocator_host_register_wait_ns={host_register_wait_ns}"),
            format!("timing_cuda_allocator_copy_h2d_bytes={h2d_bytes}"),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let fixtures = [
        (
            15_000_u64,
            5_000_u64,
            4_000_u64,
            100_u64,
            1_000_000_000_u64,
            1_000_u64,
        ),
        (18_000, 6_000, 6_000, 200, 2_000_000_000, 2_000),
        (21_000, 7_000, 8_000, 300, 3_000_000_000, 3_000),
    ];
    let paths = fixtures
        .into_iter()
        .enumerate()
        .map(|(index, fixture)| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(
                &path,
                sample(
                    fixture.0, fixture.1, fixture.2, fixture.3, fixture.4, fixture.5,
                ),
            )
            .expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let aggregate_headers = parse_csv_line(lines[4]);
    let aggregate_fields = parse_csv_line(lines[5]);
    let aggregate_value = |name: &str| {
        let index = aggregate_headers
            .iter()
            .position(|header| header == name)
            .unwrap_or_else(|| panic!("aggregate summary should expose {name}: stdout={stdout}"));
        aggregate_fields
            .get(index)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("aggregate row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(aggregate_value("witness_ms_mean"), "6000.000");
    assert_eq!(
        aggregate_value("top_level_unattributed_ms_mean"),
        "12000.000"
    );
    assert_eq!(aggregate_value("runner_ms_mean"), "6000.000");
    assert_eq!(
        aggregate_value("opening_row_value_source_extend_ms_mean"),
        "200.000"
    );
    assert_eq!(
        aggregate_value("cuda_host_register_wait_ms_mean"),
        "2000.000"
    );
    assert_eq!(aggregate_value("cuda_h2d_bytes_mean"), "2000.000");
    assert_eq!(aggregate_value("proof_12s_gap_ms_mean"), "6000.000");
}

#[test]
fn prove_timing_root_summary_aggregates_cuda_transfer_action() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-transfer-action");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
            "timing_cuda_allocator_host_register_wait_ns=5286762509".to_owned(),
            "timing_cuda_allocator_copy_h2d_bytes=88120303688".to_owned(),
            "timing_cuda_allocator_copy_h2d_wait_ns=7329175000".to_owned(),
            "timing_cuda_direct_copy_d2h_hot_bytes=1152".to_owned(),
            "timing_cuda_direct_copy_d2h_hot_count=41".to_owned(),
            "timing_cuda_direct_copy_d2h_hot_wait_ns=3388755526".to_owned(),
        ]
        .join("\n")
    };
    let paths = [60139_u64, 60080, 60200]
        .into_iter()
        .enumerate()
        .map(|(index, total_ms)| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms)).expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    let transfer_hint_index = headers
        .iter()
        .position(|header| *header == "cuda_transfer_action_hint")
        .expect("summary should expose CUDA transfer action hint");
    assert_eq!(
        fields.get(transfer_hint_index),
        Some(&"reduce_bulk_h2d_source_uploads"),
        "large H2D upload and host registration waits should point at source upload reduction: stdout={stdout}"
    );

    let aggregate_headers = lines[4].split(',').collect::<Vec<_>>();
    let aggregate_fields = lines[5].split(',').collect::<Vec<_>>();
    let dominant_index = aggregate_headers
        .iter()
        .position(|header| *header == "dominant_cuda_transfer_action_hint")
        .expect("aggregate summary should expose dominant CUDA transfer action hint");
    let consensus_index = aggregate_headers
        .iter()
        .position(|header| *header == "cuda_transfer_action_consensus")
        .expect("aggregate summary should expose CUDA transfer action consensus");
    assert_eq!(
        aggregate_fields.get(dominant_index),
        Some(&"reduce_bulk_h2d_source_uploads"),
        "aggregate row should report the dominant CUDA transfer action: stdout={stdout}"
    );
    assert_eq!(
        aggregate_fields.get(consensus_index),
        Some(&"yes"),
        "aggregate row should report stable CUDA transfer action consensus: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_aggregates_segment_memory_hints() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-segment-memory-aggregate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64, min_free_bytes: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_segment_commit_ms=21000".to_owned(),
            "timing_guest_segment_commit_cuda_memory_total_bytes=1000".to_owned(),
            "timing_guest_segment_commit_cuda_memory_min_free_bytes=".to_owned()
                + &min_free_bytes.to_string(),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let paths = [(60139_u64, 50_u64), (60080, 60), (60200, 70)]
        .into_iter()
        .enumerate()
        .map(|(index, (total_ms, min_free_bytes))| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms, min_free_bytes))
                .expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let aggregate_headers = lines[4].split(',').collect::<Vec<_>>();
    let aggregate_fields = lines[5].split(',').collect::<Vec<_>>();
    let aggregate_value = |name: &str| {
        let index = aggregate_headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("aggregate summary should expose {name}: stdout={stdout}"));
        aggregate_fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("aggregate row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        aggregate_value("dominant_segment_commit_memory_pressure_hint"),
        "segment_commit_memory_pressure"
    );
    assert_eq!(
        aggregate_value("segment_commit_memory_pressure_consensus"),
        "yes"
    );
    assert_eq!(
        aggregate_value("dominant_segment_commit_memory_diagnostic_hint"),
        "none"
    );
    assert_eq!(
        aggregate_value("segment_commit_memory_diagnostic_consensus"),
        "yes"
    );
}

#[test]
fn prove_timing_root_summary_aggregates_segment_memory_sampling_gaps() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-segment-memory-gap-aggregate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |total_ms: u64| {
        [
            "input_bytes=12447640".to_owned(),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_segment_commit_ms=21000".to_owned(),
            "timing_guest_segment_commit_cuda_memory_total_bytes=1000".to_owned(),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let paths = [60139_u64, 60080, 60200]
        .into_iter()
        .enumerate()
        .map(|(index, total_ms)| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(total_ms)).expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 6,
        "multi-sample summary should include data and aggregate rows: stdout={stdout}"
    );
    let aggregate_headers = lines[4].split(',').collect::<Vec<_>>();
    let aggregate_fields = lines[5].split(',').collect::<Vec<_>>();
    let aggregate_value = |name: &str| {
        let index = aggregate_headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("aggregate summary should expose {name}: stdout={stdout}"));
        aggregate_fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("aggregate row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        aggregate_value("dominant_segment_commit_memory_pressure_hint"),
        "memory_timing_missing"
    );
    assert_eq!(
        aggregate_value("segment_commit_memory_pressure_consensus"),
        "yes"
    );
    assert_eq!(
        aggregate_value("dominant_segment_commit_memory_diagnostic_hint"),
        "profile_segment_commit_memory_timing"
    );
    assert_eq!(
        aggregate_value("segment_commit_memory_diagnostic_consensus"),
        "yes"
    );
}

#[test]
fn prove_timing_root_summary_reports_allocator_d2h_wait_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=48396",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_external_source_count=120",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
        "timing_cuda_allocator_copy_d2h_bytes=291360",
        "timing_cuda_allocator_copy_d2h_wait_ns=3429156569",
        "timing_cuda_allocator_copy_d2h_hot_bytes=304",
        "timing_cuda_allocator_copy_d2h_hot_count=120",
        "timing_cuda_allocator_copy_d2h_hot_wait_ns=3409364047",
        "timing_cuda_setup_init_calls=3",
        "timing_cuda_setup_init_wait_ns=70123000",
        "timing_cuda_setup_init_max_wait_ns=61000000",
        "timing_cuda_setup_cache_hits=2",
        "timing_cuda_setup_cache_hit_wait_ns=9000",
        "timing_cuda_setup_cache_hit_max_wait_ns=5000",
        "timing_cuda_setup_native_init_calls=1",
        "timing_cuda_setup_native_init_wait_ns=61000000",
        "timing_cuda_setup_native_init_max_wait_ns=61000000",
        "timing_cuda_current_device_calls=3",
        "timing_cuda_current_device_wait_ns=9230000",
        "timing_cuda_current_device_max_wait_ns=9000000",
        "timing_cuda_memory_info_calls=1",
        "timing_cuda_memory_info_wait_ns=127000000",
        "timing_cuda_memory_info_max_wait_ns=127000000",
        "timing_cuda_allocator_malloc_calls=24",
        "timing_cuda_allocator_malloc_wait_ns=61289905",
        "timing_cuda_allocator_malloc_max_wait_ns=61156723",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("cuda_allocator_d2h_bytes"), "291360");
    assert_eq!(value("cuda_setup_init_calls"), "3");
    assert_eq!(value("cuda_setup_init_wait_ms"), "70.123");
    assert_eq!(value("cuda_setup_init_max_wait_ms"), "61.000");
    assert_eq!(value("cuda_setup_cache_hits"), "2");
    assert_eq!(value("cuda_setup_cache_hit_wait_ms"), "0.009");
    assert_eq!(value("cuda_setup_cache_hit_max_wait_ms"), "0.005");
    assert_eq!(value("cuda_setup_native_init_calls"), "1");
    assert_eq!(value("cuda_setup_native_init_wait_ms"), "61.000");
    assert_eq!(value("cuda_setup_native_init_max_wait_ms"), "61.000");
    assert_eq!(value("cuda_current_device_calls"), "3");
    assert_eq!(value("cuda_current_device_wait_ms"), "9.230");
    assert_eq!(value("cuda_current_device_max_wait_ms"), "9.000");
    assert_eq!(value("cuda_memory_info_calls"), "1");
    assert_eq!(value("cuda_memory_info_wait_ms"), "127.000");
    assert_eq!(value("cuda_memory_info_max_wait_ms"), "127.000");
    assert_eq!(value("cuda_allocator_malloc_calls"), "24");
    assert_eq!(value("cuda_allocator_malloc_wait_ms"), "61.290");
    assert_eq!(value("cuda_allocator_malloc_max_wait_ms"), "61.157");
    assert_eq!(value("cuda_allocator_d2h_wait_ms"), "3429.157");
    assert_eq!(value("cuda_allocator_d2h_hot_bytes"), "304");
    assert_eq!(value("cuda_allocator_d2h_hot_count"), "120");
    assert_eq!(value("cuda_allocator_d2h_hot_wait_ms"), "3409.364");
    assert_eq!(value("cuda_allocator_d2h_hot_wait_pct"), "99.423");
    assert_eq!(
        value("cuda_allocator_d2h_action_hint"),
        "opening_row_value_d2h_wait_secondary"
    );
    assert_eq!(
        value("opening_batching_hint"),
        "single_query_unit_boundary_blocks_row_value_batch"
    );
    assert_eq!(
        value("opening_external_source_boundary_hint"),
        "external_source_unit_boundary_blocks_row_value_batch"
    );
}

#[test]
fn prove_timing_root_summary_groups_aggregate_samples_by_input_size() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-input-size-aggregate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let sample = |input_bytes: u64, total_ms: u64| {
        [
            format!("input_bytes={input_bytes}"),
            format!("timing_total_ms={total_ms}"),
            "timing_guest_trace_runner_ms=41000".to_owned(),
            "timing_guest_trace_lowerer_ms=35000".to_owned(),
            "timing_guest_trace_lower_ms=33000".to_owned(),
            "timing_guest_trace_stream_elapsed_ms=43000".to_owned(),
            "timing_guest_trace_stream_ms=22000".to_owned(),
            "timing_guest_segment_commit_ms=21000".to_owned(),
            "timing_guest_trace_segment_receive_wait_ms=22000".to_owned(),
            "timing_guest_trace_pending_receive_wait_ms=1000".to_owned(),
            "timing_guest_stage_tree_commit_root_count=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_groups=120".to_owned(),
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1".to_owned(),
        ]
        .join("\n")
    };
    let fixtures = [
        (2_758_032_u64, 8_270_u64),
        (12_447_640, 50_650),
        (2_758_032, 8_330),
        (12_447_640, 51_026),
        (2_758_032, 8_365),
        (12_447_640, 50_792),
    ];
    let paths = fixtures
        .into_iter()
        .enumerate()
        .map(|(index, (input_bytes, total_ms))| {
            let path = dir.join(format!("sample-{index}.log"));
            std::fs::write(&path, sample(input_bytes, total_ms))
                .expect("sample timing log should be written");
            path
        })
        .collect::<Vec<_>>();

    let output = Command::new("python3")
        .arg(&script_path)
        .args(&paths)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("aggregate,6,6,8270,29572.167,29507.500,51026,144.899,no,yes"),
        "global aggregate should still show mixed small and large samples are not close: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate_by_input_bytes,input_bytes,total_count,valid_total_count,total_min_ms,total_mean_ms,total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier"
        ),
        "grouped aggregate should expose the input size discriminator: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate_by_input_bytes,2758032,3,3,8270,8321.667,8330.000,8365,1.140,yes,no"
        ),
        "small samples should be summarized as a close input-size group: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "aggregate_by_input_bytes,12447640,3,3,50650,50822.667,50792.000,51026,0.740,yes,no"
        ),
        "large samples should be summarized as a close input-size group: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_segment_commit_memory_margin() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-segment-commit-memory");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let input = [
        "input_bytes=12447640",
        "timing_total_ms=50142",
        "timing_guest_segment_commit_initial_workers=3",
        "timing_guest_segment_commit_effective_workers=3",
        "timing_guest_segment_commit_worker_submits=120",
        "timing_guest_segment_commit_worker_joins=120",
        "timing_guest_segment_commit_worker_backpressure_joins=117",
        "timing_guest_segment_commit_worker_backpressure_join_ms=19876",
        "timing_guest_segment_commit_worker_finish_joins=3",
        "timing_guest_segment_commit_worker_finish_join_ms=231",
        "timing_guest_segment_commit_worker_max_in_flight=3",
        "timing_guest_segment_commit_oom_retries=0",
        "timing_guest_segment_commit_cuda_memory_total_bytes=34359738368",
        "timing_guest_segment_commit_cuda_memory_initial_free_bytes=12025908428",
        "timing_guest_segment_commit_cuda_memory_effective_free_bytes=12025908428",
        "timing_guest_segment_commit_cuda_memory_min_free_bytes=1717986918",
        "timing_guest_segment_commit_cuda_memory_sample_ms=73",
        "timing_guest_segment_commit_cuda_memory_samples=2",
        "timing_guest_segment_commit_cuda_allocator_initial_cached_bytes=0",
        "timing_guest_segment_commit_cuda_allocator_effective_cached_bytes=0",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");
    let path = dir.join("sample.log");
    std::fs::write(&path, input).expect("sample timing log should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    for (header, expected) in [
        ("segment_commit_cuda_memory_total_bytes", "34359738368"),
        (
            "segment_commit_cuda_memory_initial_free_bytes",
            "12025908428",
        ),
        (
            "segment_commit_cuda_memory_effective_free_bytes",
            "12025908428",
        ),
        ("segment_commit_cuda_memory_min_free_bytes", "1717986918"),
        ("segment_commit_cuda_memory_sample_ms", "73"),
        ("segment_commit_cuda_memory_sample_count", "2"),
        ("segment_commit_cuda_allocator_initial_cached_bytes", "0"),
        ("segment_commit_cuda_allocator_effective_cached_bytes", "0"),
        ("segment_commit_worker_submits", "120"),
        ("segment_commit_worker_joins", "120"),
        ("segment_commit_worker_backpressure_joins", "117"),
        ("segment_commit_worker_backpressure_join_ms", "19876"),
        ("segment_commit_worker_finish_joins", "3"),
        ("segment_commit_worker_finish_join_ms", "231"),
        ("segment_commit_worker_max_in_flight", "3"),
        (
            "segment_commit_worker_pressure_hint",
            "worker_backpressure_dominant",
        ),
        ("segment_commit_cuda_memory_min_free_pct", "5.000"),
        (
            "segment_commit_memory_pressure_hint",
            "segment_commit_memory_pressure",
        ),
        ("segment_commit_memory_diagnostic_hint", "none"),
    ] {
        let index = headers
            .iter()
            .position(|candidate| *candidate == header)
            .unwrap_or_else(|| panic!("summary should expose {header}: stdout={stdout}"));
        assert_eq!(
            fields.get(index),
            Some(&expected),
            "summary should report {header}: stdout={stdout}"
        );
    }
}

#[test]
fn prove_timing_root_summary_does_not_infer_memory_pressure_from_total_only() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=50142",
        "timing_guest_segment_commit_initial_workers=2",
        "timing_guest_segment_commit_effective_workers=2",
        "timing_guest_segment_commit_oom_retries=0",
        "timing_guest_segment_commit_ms=20463",
        "timing_guest_segment_commit_cuda_memory_total_bytes=34359738368",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("timing input should write");
    let output = child.wait_with_output().expect("summary should finish");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("segment_commit_memory_pressure_hint"),
        "memory_timing_missing"
    );
    assert_eq!(
        value("segment_commit_memory_diagnostic_hint"),
        "profile_segment_commit_memory_timing"
    );
    assert_eq!(value("segment_commit_cuda_memory_min_free_pct"), "0.000");
}

#[test]
fn prove_timing_root_summary_prioritizes_commit_worker_oom_fallback() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=91023",
        "timing_guest_trace_runner_ms=40000",
        "timing_guest_trace_lowerer_ms=40000",
        "timing_guest_trace_lower_ms=29141",
        "timing_guest_trace_stream_elapsed_ms=40128",
        "timing_guest_trace_stream_ms=20000",
        "timing_guest_segment_commit_ms=20463",
        "timing_guest_trace_segment_receive_wait_ms=0",
        "timing_guest_trace_pending_receive_wait_ms=39999",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=39997",
        "timing_guest_segment_commit_initial_workers=2",
        "timing_guest_segment_commit_effective_workers=1",
        "timing_guest_segment_commit_oom_retries=1",
        "timing_guest_segment_commit_oom_retry_ms=41722",
        "timing_guest_segment_commit_worker_max_in_flight=1",
        "timing_guest_segment_commit_cuda_memory_total_bytes=33711521792",
        "timing_guest_segment_commit_cuda_memory_min_free_bytes=33160429568",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=3",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=11",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("timing input should write");
    let output = child.wait_with_output().expect("summary should finish");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("segment_commit_memory_pressure_hint"),
        "segment_commit_oom_fallback"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "avoid_segment_commit_worker_oom_fallback"
    );
    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "retained_parent_checkpoint_path_time_secondary"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "avoid_segment_commit_worker_oom_fallback"
    );
}

#[test]
fn prove_timing_root_summary_reports_descriptor_retention_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let dir = crate_root.join("../../temp/prove-timing-descriptor-retention");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing summary fixture directory should be created");

    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58706",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_allocator_copy_h2d_bytes=80369229896",
        "timing_cuda_allocator_copy_h2d_wait_ns=3444077241",
        "timing_cuda_direct_copy_d2h_hot_bytes=1152",
        "timing_cuda_direct_copy_d2h_hot_count=61",
        "timing_cuda_direct_copy_d2h_hot_wait_ns=4960767295",
        "timing_guest_descriptor_buffer_retention_attempts=120",
        "timing_guest_descriptor_buffer_retention_retained=21",
        "timing_guest_descriptor_buffer_retention_rejected=99",
        "timing_guest_descriptor_buffer_retention_retained_bytes=7751073792",
        "timing_guest_descriptor_buffer_retention_rejected_bytes=36241643328",
        "timing_guest_descriptor_buffer_retention_limit_bytes=8000000000",
    ]
    .join("\n");
    let path = dir.join("sample.log");
    std::fs::write(&path, input).expect("sample timing log should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    for (header, expected) in [
        ("descriptor_retention_attempts", "120"),
        ("descriptor_retention_retained", "21"),
        ("descriptor_retention_rejected", "99"),
        ("descriptor_retention_retained_bytes", "7751073792"),
        ("descriptor_retention_rejected_bytes", "36241643328"),
        ("descriptor_retention_limit_bytes", "8000000000"),
        (
            "cuda_transfer_action_hint",
            "retained_descriptor_d2h_tradeoff",
        ),
    ] {
        let index = headers
            .iter()
            .position(|candidate| *candidate == header)
            .unwrap_or_else(|| panic!("summary should expose {header}: stdout={stdout}"));
        assert_eq!(
            fields.get(index),
            Some(&expected),
            "summary should report {header}: stdout={stdout}"
        );
    }
}

#[test]
fn prove_timing_root_summary_classifies_initial_descriptor_upload_when_retained() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=2758032",
        "timing_total_ms=8250",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_allocator_copy_h2d_bytes=8264703744",
        "timing_cuda_allocator_copy_h2d_wait_ns=645000000",
        "timing_guest_device_source_descriptor_upload_bytes=8264703744",
        "timing_guest_device_source_descriptor_upload_rows=93917088",
        "timing_guest_descriptor_buffer_retention_attempts=23",
        "timing_guest_descriptor_buffer_retention_retained=23",
        "timing_guest_descriptor_buffer_retention_rejected=0",
        "timing_guest_descriptor_buffer_retention_retained_bytes=8264703744",
        "timing_guest_descriptor_buffer_retention_rejected_bytes=0",
        "timing_guest_descriptor_buffer_retention_limit_bytes=10000000000",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|candidate| *candidate == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("descriptor_upload_bytes"), "8264703744");
    assert_eq!(value("descriptor_retention_attempts"), "23");
    assert_eq!(value("descriptor_retention_retained"), "23");
    assert_eq!(value("descriptor_retention_rejected"), "0");
    assert_eq!(
        value("cuda_transfer_action_hint"),
        "initial_descriptor_upload_retention_active"
    );
}

#[test]
fn prove_timing_root_summary_reports_direct_d2h_hot_copy_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=4156548184",
        "timing_cuda_direct_copy_d2h_hot_bytes=1152",
        "timing_cuda_direct_copy_d2h_hot_count=41",
        "timing_cuda_direct_copy_d2h_hot_wait_ns=3389722844",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("direct_d2h_hot_bytes,direct_d2h_hot_count,direct_d2h_hot_wait_ms"),
        "prove timing root summary should expose hot direct D2H copy shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(",1152,41,3389.723"),
        "prove timing root summary should report the dominant direct D2H wait bucket: stdout={stdout}"
    );
    let lines = stdout.lines().collect::<Vec<_>>();
    let headers = lines[0].split(',').collect::<Vec<_>>();
    let fields = lines[1].split(',').collect::<Vec<_>>();
    for (header, expected) in [
        ("direct_d2h_hot_wait_pct", "81.551"),
        (
            "direct_d2h_action_hint",
            "single_query_unit_boundary_blocks_row_value_batch",
        ),
    ] {
        let index = headers
            .iter()
            .position(|candidate| *candidate == header)
            .unwrap_or_else(|| panic!("summary should expose {header}: stdout={stdout}"));
        assert_eq!(
            fields.get(index),
            Some(&expected),
            "summary should report {header}: stdout={stdout}"
        );
    }
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_counts() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1100",
        "timing_guest_trace_single_row_reports=900",
        "timing_guest_trace_multi_row_reports=100",
        "timing_guest_trace_pending_dma_reports=50",
        "timing_guest_trace_amo_reports=25",
        "timing_guest_trace_store_conditional_reports=10",
        "timing_guest_trace_external_op_rows=300",
        "timing_guest_trace_copy_rows=400",
        "timing_guest_trace_copy_memory_source_rows=260",
        "timing_guest_trace_copy_indirect_memory_rows=140",
        "timing_guest_trace_copy_register_store_rows=240",
        "timing_guest_trace_copy_memory_store_rows=120",
        "timing_guest_trace_copy_no_store_rows=40",
        "timing_guest_trace_copy_no_memory_rows=180",
        "timing_guest_trace_flag_rows=20",
        "timing_guest_trace_precompile_rows=8",
        "timing_guest_trace_indirect_memory_rows=500",
        "timing_guest_trace_copy_source_memory_read_ms=40",
        "timing_guest_trace_copy_source_indirect_read_ms=20",
        "timing_guest_trace_copy_source_memory_reads=260",
        "timing_guest_trace_copy_source_indirect_reads=140",
        "timing_guest_trace_copy_source_memory_read_sampled_ns=900",
        "timing_guest_trace_copy_source_indirect_read_sampled_ns=2100",
        "timing_guest_trace_copy_source_memory_read_avg_sample_ns=3",
        "timing_guest_trace_copy_source_indirect_read_avg_sample_ns=15",
        "timing_guest_trace_report_detail_samples=1000",
        "timing_guest_trace_report_sampled_ns=100000",
        "timing_guest_trace_report_source_values_sampled_ns=30000",
        "timing_guest_trace_report_source_a_value_sampled_ns=2000",
        "timing_guest_trace_report_source_b_value_sampled_ns=3000",
        "timing_guest_trace_register_source_reads=1400",
        "timing_guest_trace_memory_source_reads=300",
        "timing_guest_trace_register_store_rows=700",
        "timing_guest_trace_memory_store_rows=200",
        "timing_guest_trace_no_store_rows=100",
        "timing_guest_trace_row_shape_top_1_pattern=176643",
        "timing_guest_trace_row_shape_top_1_count=333",
        "timing_guest_trace_row_shape_top_2_pattern=1094147",
        "timing_guest_trace_row_shape_top_2_count=222",
        "timing_guest_trace_row_shape_top_3_pattern=1151491",
        "timing_guest_trace_row_shape_top_3_count=111",
        "timing_guest_trace_row_shape_top_4_pattern=268468803",
        "timing_guest_trace_row_shape_top_4_count=55",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "single_row_reports,multi_row_reports,pending_dma_reports,amo_reports,store_conditional_reports,external_op_rows,copy_rows,flag_rows,precompile_rows,indirect_memory_rows,indirect_memory_row_pct,register_source_reads,memory_source_reads,memory_source_read_pct,register_store_rows,memory_store_rows,memory_store_row_pct,no_store_rows,no_store_row_pct,trace_shape_sample_hint,row_shape_top_1_pattern,row_shape_top_1_count,row_shape_top_1_shape,row_shape_top_2_pattern,row_shape_top_2_count,row_shape_top_2_shape,row_shape_top_3_pattern,row_shape_top_3_count,row_shape_top_3_shape,row_shape_top_4_pattern,row_shape_top_4_count,row_shape_top_4_shape"
        ),
        "prove timing root summary should expose trace shape columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",900,100,50,25,10,300,400,20,8,500,45.455,1400,300,27.273,700,200,18.182,100,9.091,shape_timing_enabled,176643,333,op=CopyB;a=reg;b=indirect;store=reg;ind_width=1;store_pc=0;set_pc=0;m32=0;external=0;precompiled=0,1094147,222,op=CopyB;a=reg;b=indirect;store=reg;ind_width=8;store_pc=0;set_pc=0;m32=0;external=0;precompiled=0,1151491,111,op=CopyB;a=reg;b=reg;store=indirect;ind_width=8;store_pc=0;set_pc=0;m32=0;external=0;precompiled=0,268468803,55,op=Sll;a=reg;b=imm;store=reg;ind_width=0;store_pc=0;set_pc=0;m32=0;external=1;precompiled=0,"
        ),
        "prove timing root summary should classify trace shape ratios: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "copy_memory_source_rows,copy_memory_source_row_pct,copy_indirect_memory_rows,copy_indirect_memory_row_pct,copy_register_store_rows,copy_memory_store_rows,copy_no_store_rows,copy_no_memory_rows,copy_no_memory_row_pct,trace_copy_shape_hint,trace_copy_action_hint"
        ),
        "prove timing root summary should expose copy row shape columns: stdout={stdout}"
    );
    assert!(
        stdout
            .contains(",260,65.000,140,35.000,240,120,40,180,45.000,copy_memory_source_dominant,target_copy_memory_source_and_indirect_validation,"),
        "prove timing root summary should classify copy row shape ratios: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "copy_source_memory_read_ms,copy_source_indirect_read_ms,copy_source_memory_read_pct,copy_source_indirect_read_pct,copy_source_memory_reads,copy_source_indirect_reads"
        ),
        "prove timing root summary should expose CopyB source-read timing columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",40,20,66.667,33.333,260,140,"),
        "prove timing root summary should classify CopyB source-read timing split: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "copy_source_memory_read_sampled_ns,copy_source_indirect_read_sampled_ns,copy_source_memory_read_avg_sample_ns,copy_source_indirect_read_avg_sample_ns"
        ),
        "prove timing root summary should expose sampled CopyB source-read timing columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",260,140,900,2100,3,15,"),
        "prove timing root summary should report sampled CopyB source-read timing: stdout={stdout}"
    );
    assert!(
        stdout.contains("trace_copy_source_action_hint"),
        "prove timing root summary should expose a CopyB source-read action hint: stdout={stdout}"
    );
    assert!(
        stdout.contains(",900,2100,3,15,target_copy_source_values_residual,"),
        "prove timing root summary should not target thin CopyB source lookup when source-values residual dominates: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_skips_precompile_probe_when_shape_has_no_precompile_rows() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=47000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=500000000",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_single_row_reports=499000000",
        "timing_guest_trace_external_op_rows=237000000",
        "timing_guest_trace_copy_rows=253000000",
        "timing_guest_trace_precompile_rows=0",
        "timing_guest_trace_indirect_memory_rows=224000000",
        "timing_guest_trace_register_source_reads=661000000",
        "timing_guest_trace_memory_source_reads=145000000",
        "timing_guest_trace_register_store_rows=372000000",
        "timing_guest_trace_memory_store_rows=81000000",
        "timing_guest_trace_no_store_rows=47000000",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let fields = lines
        .next()
        .expect("summary should print a row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|candidate| *candidate == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_shape_sample_hint"), "shape_timing_enabled");
    assert_eq!(value("precompile_rows"), "0");
    assert_eq!(
        value("trace_precompile_action_hint"),
        "skip_precompile_microprobes"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_row_mix_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=72000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=500000000",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_single_row_reports=499000000",
        "timing_guest_trace_external_op_rows=237000000",
        "timing_guest_trace_copy_rows=254000000",
        "timing_guest_trace_indirect_memory_rows=224000000",
        "timing_guest_trace_memory_source_reads=146000000",
        "timing_guest_trace_memory_store_rows=81200000",
        "timing_guest_trace_no_store_rows=46400000",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("external_op_row_pct,copy_row_pct,trace_shape_row_mix_hint"),
        "prove timing root summary should expose external-op and copy row mix columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",47.400,50.800,copy_and_external_op_rows_dominate"),
        "prove timing root summary should report row mix percentages and hotspot hint: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_duration_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=72000",
        "timing_guest_trace_lower_ms=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_external_op_rows=300",
        "timing_guest_trace_copy_rows=400",
        "timing_guest_trace_external_op_row_lower_ms=474",
        "timing_guest_trace_copy_row_lower_ms=508",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "external_op_row_lower_ms,copy_row_lower_ms,external_op_row_lower_ns_per_row,copy_row_lower_ns_per_row,external_op_row_lower_pct,copy_row_lower_pct,trace_shape_duration_hint,trace_shape_unit_cost_hint"
        ),
        "prove timing root summary should expose external-op and copy duration, per-row, and unit-cost hint columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",474,508,1580000.000,1270000.000,47.400,50.800,copy_and_external_op_duration_dominate,external_op_unit_cost_higher"
        ),
        "prove timing root summary should classify external-op and copy duration dominance and per-row cost skew: stdout={stdout}"
    );

    let balanced_input = [
        "timing_total_ms=72000",
        "timing_guest_trace_lower_ms=51154",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_external_op_rows=237231598",
        "timing_guest_trace_copy_rows=253826801",
        "timing_guest_trace_external_op_row_lower_ms=14926",
        "timing_guest_trace_copy_row_lower_ms=16895",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(balanced_input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            ",14926,16895,62.917,66.561,29.179,33.028,mixed_trace_shape_duration,row_volume_dominates_shape_duration"
        ),
        "prove timing root summary should classify balanced shape unit costs as row-volume dominated: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_shape_run_lengths() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=1500",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_single_row_reports=1000",
        "timing_guest_trace_external_op_rows=600",
        "timing_guest_trace_copy_rows=300",
        "timing_guest_trace_external_op_row_lower_ms=600",
        "timing_guest_trace_copy_row_lower_ms=300",
        "timing_guest_trace_external_op_runs=30",
        "timing_guest_trace_external_op_max_run=80",
        "timing_guest_trace_copy_runs=150",
        "timing_guest_trace_copy_max_run=3",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "external_op_runs,external_op_avg_run,external_op_max_run,copy_runs,copy_avg_run,copy_max_run,trace_shape_run_hint"
        ),
        "prove timing root summary should expose trace shape run-length columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(",30,20.000,80,150,2.000,3,external_op_runs_long"),
        "prove timing root summary should classify long external-op row runs: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_parallel_reexecution_hint_for_row_volume_floor() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=54000",
        "timing_guest_trace_runner_ms=41000",
        "timing_guest_trace_lowerer_ms=40500",
        "timing_guest_trace_lower_ms=27000",
        "timing_guest_trace_stream_elapsed_ms=41200",
        "timing_guest_trace_stream_ms=21000",
        "timing_guest_segment_commit_ms=20000",
        "timing_guest_trace_segment_receive_wait_ms=19000",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_external_op_rows=235000000",
        "timing_guest_trace_copy_rows=255000000",
        "timing_guest_trace_indirect_memory_rows=220000000",
        "timing_guest_trace_external_op_row_lower_ms=12600",
        "timing_guest_trace_copy_row_lower_ms=13900",
        "timing_guest_trace_external_op_runs=78000000",
        "timing_guest_trace_copy_runs=77000000",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_shape_unit_cost_hint"),
        "row_volume_dominates_shape_duration"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "parallel_segment_reexecution_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "parallel_segment_reexecution_candidate"
    );
    assert_eq!(
        value("trace_shape_profile_hint"),
        "diagnostic_only_shape_profile"
    );
}

#[test]
fn prove_timing_root_summary_reports_spiky_trace_shape_run_lengths() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=74000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=53000",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_single_row_reports=499366777",
        "timing_guest_trace_external_op_rows=237231598",
        "timing_guest_trace_copy_rows=253826801",
        "timing_guest_trace_external_op_row_lower_ms=15939",
        "timing_guest_trace_copy_row_lower_ms=17901",
        "timing_guest_trace_external_op_runs=78604119",
        "timing_guest_trace_external_op_max_run=99",
        "timing_guest_trace_copy_runs=77229084",
        "timing_guest_trace_copy_max_run=250",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(",78604119,3.018,99,77229084,3.287,250,shape_runs_spiky"),
        "prove timing root summary should distinguish sparse long-tail runs from strong long-run batching candidates: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_marks_trace_shape_timing_disabled_or_zero() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_report_rows=1100",
        "timing_guest_trace_single_row_reports=0",
        "timing_guest_trace_indirect_memory_rows=0",
        "timing_guest_trace_memory_source_reads=0",
        "timing_guest_trace_memory_store_rows=0",
        "timing_guest_trace_no_store_rows=0",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let fields = lines
        .next()
        .expect("summary should print a row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|candidate| *candidate == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert!(
        stdout.contains(
            ",0,0.000,0,0,0.000,0,0,0.000,0,0.000,shape_timing_disabled_or_zero,"
        ),
        "prove timing root summary should say shape timing is disabled instead of implying zero-shape rows: stdout={stdout}"
    );
    assert_eq!(
        value("trace_precompile_action_hint"),
        "enable_shape_timing_for_precompile_rows"
    );
}

#[test]
fn prove_timing_root_summary_requests_shape_profile_after_detail_only_trace_sample() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=63027",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lower_ms=38619",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_single_row_reports=0",
        "timing_guest_trace_indirect_memory_rows=0",
        "timing_guest_trace_memory_source_reads=0",
        "timing_guest_trace_memory_store_rows=0",
        "timing_guest_trace_no_store_rows=0",
        "timing_guest_trace_report_detail_samples=596",
        "timing_guest_trace_report_sampled_ns=1861501",
        "timing_guest_trace_report_row_validation_sampled_ns=987695",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("shape_timing_missing_for_detail_profile"),
        "prove timing root summary should request shape timing when detail samples exist but shape counters are absent: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_marks_sampled_trace_shape_profile() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=36000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000000",
        "timing_guest_trace_report_rows=1000000",
        "timing_guest_trace_shape_samples=5",
        "timing_guest_trace_shape_sample_rows=10",
        "timing_guest_trace_single_row_reports=10",
        "timing_guest_trace_copy_rows=6",
        "timing_guest_trace_external_op_rows=4",
        "timing_guest_trace_row_shape_top_1_pattern=176643",
        "timing_guest_trace_row_shape_top_1_count=6",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = parse_csv_line(lines.next().expect("summary should include a header"));
    let row = parse_csv_line(lines.next().expect("summary should include a data row"));
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|candidate| candidate == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_shape_sample_hint"), "shape_timing_sampled");
    assert_eq!(value("external_op_row_pct"), "40.000");
    assert_eq!(value("copy_row_pct"), "60.000");
    assert_eq!(
        value("trace_shape_row_mix_hint"),
        "copy_and_external_op_rows_dominate"
    );
    assert_eq!(
        value("trace_precompile_action_hint"),
        "skip_precompile_microprobes"
    );
    assert_eq!(
        value("trace_shape_profile_hint"),
        "diagnostic_only_shape_profile"
    );
}

#[test]
fn prove_timing_root_summary_reports_tiny_detail_sample_coverage_ppm() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=1000000",
        "timing_guest_trace_report_rows=1000000",
        "timing_guest_trace_report_detail_samples=1",
        "timing_guest_trace_report_sampled_ns=1000",
        "timing_guest_trace_report_row_validation_sampled_ns=500",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("trace_report_detail_sample_pct,trace_report_detail_sample_ppm,"),
        "prove timing root summary should expose ppm coverage next to percent coverage: stdout={stdout}"
    );
    assert!(
        stdout.contains(",1,0.000,1.000,detail_timing_sampled,1000,"),
        "prove timing root summary should preserve tiny sampled coverage that rounds to zero percent: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_retained_parent_checkpoint_opening_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52335",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_external_source_count=240",
        "timing_finish_witness_opening_retained_leaf_digest_openings=77",
        "timing_finish_witness_opening_retained_leaf_digest_rows=77",
        "timing_finish_witness_opening_retained_leaf_digest_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches=0",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=3",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=14",
        "timing_finish_witness_opening_row_dedup_input_rows=120",
        "timing_finish_witness_opening_row_dedup_unique_rows=119",
        "timing_finish_witness_opening_row_dedup_elided_rows=1",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "retained_parent_checkpoint_openings,retained_parent_checkpoint_rows,retained_parent_checkpoint_all_single_row,retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_prefix_ms,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_suffix_ms,retained_parent_checkpoint_path_launches,retained_parent_checkpoint_path_ms,retained_parent_checkpoint_cross_stage_gather_estimated_launches,retained_parent_checkpoint_cross_stage_gather_launch_savings"
        ),
        "prove timing root summary should expose retained parent checkpoint opening shape: stdout={stdout}"
    );
    assert!(
        stdout.contains("retained_parent_checkpoint_batching_hint"),
        "prove timing root summary should expose retained parent checkpoint batching shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "opening_row_value_device_download_batches,opening_row_value_device_batch_stage_count,opening_row_value_device_batch_max_stage,opening_row_value_device_batch_stage_sum,opening_row_value_device_batch_unattributed,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings,opening_batching_hint,opening_external_source_boundary_hint"
        ),
        "prove timing root summary should expose single-row device row-value download boundaries: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",43,0,0,0.000,none,120,119,1,0.833,77,77,yes,0,79,79,yes,0,0,79,3,0,0,790,14,869,17,11,858,device_batched_path_secondary,0,0,0,0,0,0,43,0,0,0,single_query_unit_boundary_blocks_row_value_batch,external_source_unit_boundary_blocks_row_value_batch,"
        ),
        "prove timing root summary should identify external-source unit boundaries behind single-row D2H: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "opening_row_dedup_input_rows,opening_row_dedup_unique_rows,opening_row_dedup_elided_rows,opening_row_dedup_elided_pct"
        ),
        "prove timing root summary should expose row dedup shape: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_retained_parent_checkpoint_action_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52335",
        "timing_finish_witness_opening_ms=9000",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=1300",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=2600",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("opening_batching_hint"),
        "single_query_unit_boundary_blocks_row_value_batch"
    );
    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "cross_stage_retained_parent_checkpoint_prefix_suffix_gather_candidate"
    );
}

#[test]
fn prove_timing_root_summary_reports_opening_query_unit_scope() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=52335",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_fri_opening_ms=41",
        "timing_finish_fri_opening_unit_build_ms=40",
        "timing_finish_fri_opening_layer_tree_ms=7",
        "timing_finish_fri_opening_query_ms=11",
        "timing_finish_fri_opening_fold_ms=13",
        "timing_finish_fri_opening_unit_count=3",
        "timing_finish_fri_opening_layer_count=12",
        "timing_finish_fri_opening_query_count=30",
        "timing_finish_witness_opening_retained_source_count=77",
        "timing_finish_witness_opening_external_source_count=79",
        "timing_finish_witness_opening_embedded_source_count=84",
        "timing_finish_witness_opening_missing_source_count=0",
        "timing_finish_witness_opening_row_values_device_rows=79",
        "timing_finish_witness_opening_row_values_source_rows=77",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("prove timing root summary should print a header");
    let row = lines
        .next()
        .expect("prove timing root summary should print a data row");
    let headers = header.split(',').collect::<Vec<_>>();
    let fields = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        fields
            .get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("opening_queries"), "120");
    assert_eq!(value("opening_max_queries_per_unit"), "1");
    assert_eq!(value("opening_stage_count"), "240");
    assert_eq!(value("fri_opening_ms"), "41");
    assert_eq!(value("fri_opening_unit_build_ms"), "40");
    assert_eq!(value("fri_opening_layer_tree_ms"), "7");
    assert_eq!(value("fri_opening_query_ms"), "11");
    assert_eq!(value("fri_opening_fold_ms"), "13");
    assert_eq!(value("fri_opening_unit_build_scope_pct"), "97.561");
    assert_eq!(value("fri_opening_layer_tree_nested_pct"), "17.073");
    assert_eq!(value("fri_opening_query_nested_pct"), "26.829");
    assert_eq!(value("fri_opening_fold_nested_pct"), "31.707");
    assert_eq!(value("fri_opening_known_nested_ms"), "31");
    assert_eq!(value("fri_opening_known_nested_pct"), "75.610");
    assert_eq!(value("fri_opening_unit_build_residual_ms"), "9");
    assert_eq!(value("fri_opening_unit_build_residual_pct"), "21.951");
    assert_eq!(
        value("fri_opening_scope_hint"),
        "profile_fri_opening_nested_fold"
    );
    assert_eq!(value("fri_opening_units"), "3");
    assert_eq!(value("fri_opening_layers"), "12");
    assert_eq!(value("fri_opening_queries"), "30");
    assert_eq!(value("fri_layers_per_unit"), "4.000");
    assert_eq!(value("fri_queries_per_unit"), "10.000");
    assert_eq!(
        value("opening_source_shape_hint"),
        "single_query_cross_root_with_mixed_sources"
    );
    assert_eq!(value("source_retention_attempts"), "0");
    assert_eq!(value("source_retention_retained"), "0");
    assert_eq!(value("source_retention_rejected"), "0");
    assert_eq!(
        value("opening_source_rebuild_hint"),
        "mixed_retained_and_external_sources"
    );
    assert_eq!(value("opening_row_value_device_rows"), "79");
    assert_eq!(value("opening_row_value_source_rows"), "77");
}

#[test]
fn prove_timing_root_summary_reports_opening_parent_hash_work_scope() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows=165675008",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes=21206401024",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=3",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows=13808034",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes=1767428352",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=170",
        "timing_finish_witness_opening_path_parent_hash_launches_per_stage=5",
        "timing_finish_witness_opening_row_values_device_download_batches=43",
        "timing_finish_witness_stage_1_opening_row_values_device_download_batches=31",
        "timing_finish_witness_stage_2_opening_row_values_device_download_batches=12",
        "timing_finish_witness_stage_3_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=0",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "retained_parent_checkpoint_prefix_rows,retained_parent_checkpoint_prefix_bytes,retained_parent_checkpoint_prefix_launches,retained_parent_checkpoint_prefix_ms,retained_parent_checkpoint_suffix_rows,retained_parent_checkpoint_suffix_bytes,retained_parent_checkpoint_suffix_launches,retained_parent_checkpoint_suffix_ms,retained_parent_checkpoint_path_launches,retained_parent_checkpoint_path_ms,retained_parent_checkpoint_cross_stage_gather_estimated_launches,retained_parent_checkpoint_cross_stage_gather_launch_savings,retained_parent_checkpoint_batching_hint,opening_path_parent_hash_launches_per_stage,opening_row_value_device_download_batches,opening_row_value_device_batch_stage_count,opening_row_value_device_batch_max_stage,opening_row_value_device_batch_stage_sum,opening_row_value_device_batch_unattributed,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings"
        ),
        "prove timing root summary should expose opening parent-hash work scope columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",79,79,yes,165675008,21206401024,79,3,13808034,1767428352,790,170,869,173,11,858,device_batched_path_secondary,5,43,2,31,43,0,0,"
        ),
        "prove timing root summary should report retained parent checkpoint cross-stage gather launch savings: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            ",device_batched_path_secondary,5,43,2,31,43,0,0,0,0,0,retained_parent_checkpoint_path_time_secondary,"
        ),
        "prove timing root summary should downgrade retained parent checkpoint batching when measured path time is secondary: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_device_row_value_batch_stage_coverage_gap() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_row_values_device_download_batches=43",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines.next().expect("summary should include a header");
    let row = lines.next().expect("summary should include a data row");
    let headers = headers.split(',').collect::<Vec<_>>();
    let row = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing summary header {name}: {headers:?}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("missing summary value {name}: {row:?}"))
    };

    assert_eq!(value("opening_row_value_device_download_batches"), "43");
    assert_eq!(value("opening_row_value_device_batch_stage_count"), "0");
    assert_eq!(value("opening_row_value_device_batch_max_stage"), "0");
    assert_eq!(value("opening_row_value_device_batch_stage_sum"), "0");
    assert_eq!(value("opening_row_value_device_batch_unattributed"), "43");
}

#[test]
fn prove_timing_root_summary_clamps_device_row_value_batch_unattributed() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_row_values_device_download_batches=10",
        "timing_finish_witness_stage_1_opening_row_values_device_download_batches=31",
        "timing_finish_witness_stage_2_opening_row_values_device_download_batches=12",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines.next().expect("summary should include a header");
    let row = lines.next().expect("summary should include a data row");
    let headers = headers.split(',').collect::<Vec<_>>();
    let row = row.split(',').collect::<Vec<_>>();
    assert_eq!(
        headers.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing summary header {name}: {headers:?}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("missing summary value {name}: {row:?}"))
    };

    assert_eq!(value("opening_row_value_device_download_batches"), "10");
    assert_eq!(value("opening_row_value_device_batch_stage_count"), "2");
    assert_eq!(value("opening_row_value_device_batch_max_stage"), "31");
    assert_eq!(value("opening_row_value_device_batch_stage_sum"), "43");
    assert_eq!(value("opening_row_value_device_batch_unattributed"), "0");
}

#[test]
fn prove_timing_root_summary_reports_device_row_value_single_download_stage_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "input_bytes=12447640",
        "timing_total_ms=58552",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_cuda_direct_copy_d2h_wait_ns=3389670000",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_stage_count=240",
        "timing_finish_witness_opening_row_values_device_rows=43",
        "timing_finish_witness_opening_row_values_device_download_batches=0",
        "timing_finish_witness_opening_row_values_device_single_downloads=43",
        "timing_finish_witness_stage_1_opening_row_values_device_single_downloads=31",
        "timing_finish_witness_stage_2_opening_row_values_device_single_downloads=12",
        "timing_finish_witness_stage_3_opening_row_values_device_single_downloads=0",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "opening_row_value_device_batch_stage_count,opening_row_value_device_batch_max_stage,opening_row_value_device_batch_stage_sum,opening_row_value_device_batch_unattributed,opening_row_value_device_single_downloads,opening_row_value_device_single_stage_count,opening_row_value_device_single_max_stage,opening_row_value_device_cross_unit_batch_savings"
        ),
        "prove timing root summary should expose stage-level device single-download shape: stdout={stdout}"
    );
    assert!(
        stdout.contains(",0,0,0,0,0,43,2,31,41,single_query_unit_boundary_blocks_row_value_batch,"),
        "prove timing root summary should report the single-query unit boundary instead of a row-value gather estimate: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_buffer_shape() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=94371840",
        "timing_guest_trace_report_buffer_max_capacity=4194304",
        "timing_guest_trace_report_buffer_excess_capacity=528303",
        "timing_guest_trace_report_record_size_bytes=128",
        "timing_guest_trace_report_storage_bytes=12011972736",
        "timing_guest_trace_report_buffer_capacity_bytes=12079595520",
        "timing_guest_trace_report_buffer_excess_bytes=67622784",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            "trace_reports,trace_report_rows,trace_rows_per_report,trace_report_record_size_bytes,trace_report_instruction_size_bytes,trace_report_register_write_list_size_bytes,trace_report_memory_access_list_size_bytes,trace_report_precompile_access_list_size_bytes,trace_report_instruction_storage_gib,trace_report_register_write_list_storage_gib,trace_report_memory_access_list_storage_gib,trace_report_precompile_access_list_storage_gib,trace_report_storage_bytes,trace_report_storage_gib,trace_report_buffer_capacity,trace_report_buffer_max_capacity,trace_report_buffer_excess_capacity,trace_report_buffer_capacity_bytes,trace_report_buffer_capacity_gib,trace_report_buffer_excess_bytes,trace_report_buffer_excess_pct,trace_report_buffer_shape_hint,"
        ),
        "prove timing root summary should expose trace report buffer columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "93843537,93917088,1.001,128,0,0,0,0,0.000,0.000,0.000,0.000,12011972736,11.187,94371840,4194304,528303,12079595520,11.250,67622784,0.560,report_buffer_capacity_tight,"
        ),
        "prove timing root summary should classify tight report buffer capacity: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_lifetime_pressure() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=94371840",
        "timing_guest_trace_report_buffer_max_capacity=4194304",
        "timing_guest_trace_report_buffer_excess_capacity=528303",
        "timing_guest_trace_report_record_size_bytes=128",
        "timing_guest_trace_report_storage_bytes=12011972736",
        "    16.21%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     7.41%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("trace_report_lifetime_hint,"),
        "prove timing root summary should expose trace report lifetime hint: stdout={stdout}"
    );
    let mut lines = stdout.lines();
    let headers = lines.next().expect("summary should include a header");
    let row = lines.next().expect("summary should include a data row");
    let headers = headers.split(',').collect::<Vec<_>>();
    let row = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| -> &str {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing header {name}: {headers:?}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("missing value for {name}: {row:?}"))
    };
    assert_eq!(value("trace_report_chunk_sent"), "0");
    assert_eq!(value("trace_report_chunk_received"), "0");
    assert_eq!(value("trace_report_chunk_reports"), "0");
    assert_eq!(value("trace_report_chunk_rows"), "0");
    assert_eq!(value("trace_report_chunk_max_queued"), "0");
    assert!(
        stdout.contains("tight_report_buffer_and_pending_drop"),
        "prove timing root summary should classify report lifetime pressure: stdout={stdout}"
    );
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "fused_runner_lowerer_report_storage_candidate"
    );
}

#[test]
fn prove_timing_root_summary_prioritizes_report_storage_when_seed_is_ready() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=39146",
        "timing_guest_trace_lowerer_ms=39146",
        "timing_guest_trace_lower_ms=30784",
        "timing_guest_trace_stream_elapsed_ms=39272",
        "timing_guest_trace_stream_ms=19164",
        "timing_guest_segment_commit_ms=20107",
        "timing_guest_trace_segment_receive_wait_ms=19162",
        "timing_guest_trace_pending_receive_wait_ms=39145",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=39144",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=94371840",
        "timing_guest_trace_report_buffer_max_capacity=4194304",
        "timing_guest_trace_report_buffer_excess_capacity=528303",
        "timing_guest_trace_report_record_size_bytes=128",
        "timing_guest_trace_report_storage_bytes=12011972736",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    16.21%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     7.41%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "seed_direct_lift_ready"
    );
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "fused_runner_lowerer_report_storage_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "fused_runner_lowerer_report_storage_candidate"
    );
}

#[test]
fn prove_timing_root_summary_distinguishes_elided_report_buffer_from_missing_data() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=13203",
        "timing_guest_trace_runner_ms=12433",
        "timing_guest_trace_lowerer_ms=0",
        "timing_guest_trace_stream_elapsed_ms=12582",
        "timing_guest_trace_stream_ms=10816",
        "timing_guest_segment_commit_ms=1766",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_buffer_capacity=0",
        "timing_guest_trace_report_buffer_max_capacity=0",
        "timing_guest_trace_report_buffer_excess_capacity=0",
        "timing_guest_trace_report_record_size_bytes=144",
        "timing_guest_trace_report_storage_bytes=13513469328",
        "timing_guest_trace_report_buffer_capacity_bytes=0",
        "timing_guest_trace_report_buffer_excess_bytes=0",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=23",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let headers = lines.next().expect("summary should include a header");
    let row = lines.next().expect("summary should include a data row");
    let headers = headers.split(',').collect::<Vec<_>>();
    let row = row.split(',').collect::<Vec<_>>();
    let value = |name: &str| -> &str {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("missing header {name}: {headers:?}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("missing value for {name}: {row:?}"))
    };
    assert_eq!(
        value("trace_report_buffer_shape_hint"),
        "report_buffer_elided"
    );
    assert_eq!(
        value("trace_runner_report_buffer_shape_hint"),
        "runner_report_buffer_capacity_missing"
    );
    assert_eq!(
        value("trace_report_lifetime_hint"),
        "report_buffer_elided_but_trace_serialized"
    );
}

#[test]
fn prove_timing_root_summary_reports_runner_report_buffer_capacity() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=13203",
        "timing_guest_trace_runner_ms=12433",
        "timing_guest_trace_lowerer_ms=0",
        "timing_guest_trace_stream_elapsed_ms=12582",
        "timing_guest_trace_stream_ms=10816",
        "timing_guest_segment_commit_ms=1766",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_rows=93917088",
        "timing_guest_trace_report_chunk_sent=23",
        "timing_guest_trace_report_chunk_received=23",
        "timing_guest_trace_report_chunk_reports=93843537",
        "timing_guest_trace_report_chunk_rows=93917088",
        "timing_guest_trace_report_chunk_max_queued=1",
        "timing_guest_trace_runner_report_buffer_capacity=94371840",
        "timing_guest_trace_runner_report_buffer_max_capacity=4194304",
        "timing_guest_trace_runner_report_buffer_excess_capacity=528303",
        "timing_guest_trace_report_buffer_capacity=0",
        "timing_guest_trace_report_buffer_max_capacity=0",
        "timing_guest_trace_report_buffer_excess_capacity=0",
        "timing_guest_trace_report_record_size_bytes=128",
        "timing_guest_trace_report_storage_bytes=12011972736",
        "timing_guest_trace_runner_report_buffer_capacity_bytes=12079595520",
        "timing_guest_trace_runner_report_buffer_excess_bytes=67622784",
        "timing_guest_trace_report_buffer_capacity_bytes=0",
        "timing_guest_trace_report_buffer_excess_bytes=0",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=23",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert!(
        stdout.contains(
            "trace_runner_report_buffer_capacity,trace_runner_report_buffer_max_capacity,trace_runner_report_buffer_excess_capacity,trace_runner_report_buffer_capacity_bytes,trace_runner_report_buffer_capacity_gib,trace_runner_report_buffer_excess_bytes,trace_runner_report_buffer_excess_pct,trace_runner_report_buffer_shape_hint,"
        ),
        "prove timing root summary should expose runner report buffer columns: stdout={stdout}"
    );
    assert!(
        stdout.contains(
            "94371840,4194304,528303,12079595520,11.250,67622784,0.560,runner_report_buffer_capacity_tight,"
        ),
        "prove timing root summary should classify runner report buffer pressure: stdout={stdout}"
    );
    assert!(
        stdout.contains("post_segment_report_chunk_split"),
        "prove timing root summary should distinguish post-segment chunk splitting from live runner streaming: stdout={stdout}"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "report_chunks_post_segment_split_regression"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "report_chunks_post_segment_split_regression"
    );
}

#[test]
fn prove_timing_root_summary_reports_serial_trace_structure_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_runner_ms=7600",
        "timing_guest_trace_lowerer_ms=7900",
        "timing_guest_trace_stream_elapsed_ms=8200",
        "timing_guest_trace_stream_ms=6100",
        "timing_guest_segment_commit_ms=900",
        "timing_guest_trace_segment_receive_wait_ms=6000",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_stage_leaf_kernel_work_ms=850",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=23",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("primary_bottleneck,trace_structure_hint,"),
        "prove timing root summary should expose trace structure hint: stdout={stdout}"
    );
    assert!(
        stdout.contains("stream_elapsed,trace_stream_cpu_floor,"),
        "prove timing root summary should classify serial CPU trace structure: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reports_runner_bound_parallel_lower() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=39146",
        "timing_guest_trace_lowerer_ms=39146",
        "timing_guest_trace_lower_ms=30784",
        "timing_guest_trace_stream_elapsed_ms=39272",
        "timing_guest_trace_stream_ms=19164",
        "timing_guest_segment_commit_ms=20107",
        "timing_guest_trace_segment_receive_wait_ms=19162",
        "timing_guest_trace_pending_receive_wait_ms=39145",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_dispatch_wait_ms=77",
        "timing_guest_trace_parallel_lower_stream_start_dispatch_wait_ms=1",
        "timing_guest_trace_parallel_lower_stream_chunk_dispatch_wait_ms=70",
        "timing_guest_trace_parallel_lower_stream_segment_dispatch_wait_ms=3",
        "timing_guest_trace_parallel_lower_stream_finish_dispatch_wait_ms=3",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=39144",
        "timing_guest_trace_parallel_lower_dispatch_blocked_count=3",
        "timing_guest_stage_leaf_kernel_work_ms=4499",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(value("trace_structure_hint"), "parallel_lower_runner_bound");
    assert_eq!(value("parallel_lower_dispatch_wait_ms"), "77");
    assert_eq!(value("parallel_lower_stream_start_dispatch_wait_ms"), "1");
    assert_eq!(value("parallel_lower_stream_chunk_dispatch_wait_ms"), "70");
    assert_eq!(value("parallel_lower_stream_segment_dispatch_wait_ms"), "3");
    assert_eq!(value("parallel_lower_stream_finish_dispatch_wait_ms"), "3");
    assert_eq!(value("parallel_lower_result_receive_wait_ms"), "39144");
    assert_eq!(value("parallel_lower_dispatch_blocked_count"), "3");
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "parallel_segment_reexecution_candidate"
    );
}

#[test]
fn prove_timing_root_summary_reports_seed_ready_parallel_lower_runner_bound() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=27883",
        "timing_guest_trace_runner_ms=22417",
        "timing_guest_trace_lowerer_ms=22448",
        "timing_guest_trace_lower_ms=24452",
        "timing_guest_trace_stream_elapsed_ms=22580",
        "timing_guest_trace_stream_ms=7498",
        "timing_guest_segment_commit_ms=15082",
        "timing_guest_trace_segment_receive_wait_ms=1200",
        "timing_guest_trace_pending_receive_wait_ms=17756",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_dispatched=477",
        "timing_guest_trace_parallel_lower_received=477",
        "timing_guest_trace_parallel_lower_emitted=477",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=17756",
        "timing_guest_trace_seed_direct_lift_attempts=476",
        "timing_guest_trace_seed_direct_lift_successes=476",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_report_storage_bytes=27973158808",
        "timing_guest_stage_leaf_kernel_work_ms=4825",
        "timing_guest_stage_tree_commit_root_count=477",
        "timing_guest_stage_tree_commit_root_materialization_groups=477",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(value("trace_structure_hint"), "parallel_lower_runner_bound");
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "seed_direct_lift_ready"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "parallel_lower_runner_bound_after_seed_ready"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "parallel_lower_runner_bound_after_seed_ready"
    );
}

#[test]
fn prove_timing_root_summary_flags_seed_ready_parallel_lower_result_bound() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=43100",
        "timing_guest_trace_lowerer_ms=43080",
        "timing_guest_trace_lower_ms=32000",
        "timing_guest_trace_stream_elapsed_ms=43120",
        "timing_guest_trace_stream_ms=34500",
        "timing_guest_segment_commit_ms=8000",
        "timing_guest_trace_segment_receive_wait_ms=22000",
        "timing_guest_trace_pending_receive_wait_ms=0",
        "timing_guest_trace_parallel_lower_workers=8",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_dispatch_wait_ms=24000",
        "timing_guest_trace_parallel_lower_stream_chunk_dispatch_wait_ms=23000",
        "timing_guest_trace_parallel_lower_stream_chunk_process_ms=30000",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=43000",
        "timing_guest_trace_parallel_lower_dispatch_blocked_count=98000",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_stage_leaf_kernel_work_ms=4499",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(
        value("trace_structure_hint"),
        "parallel_lower_worker_result_bound"
    );
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "seed_direct_lift_ready"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "parallel_lower_result_bound_after_seed_ready"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "parallel_lower_result_bound_after_seed_ready"
    );
}

#[test]
fn prove_timing_root_summary_avoids_repeating_seed_ready_streamed_lower_regression() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=67839",
        "timing_guest_trace_runner_ms=25884",
        "timing_guest_trace_lowerer_ms=35162",
        "timing_guest_trace_lower_ms=46433",
        "timing_guest_trace_stream_elapsed_ms=61865",
        "timing_guest_trace_stream_ms=38969",
        "timing_guest_segment_commit_ms=22895",
        "timing_guest_segment_input_gap_ms=35103",
        "timing_guest_trace_segment_receive_wait_ms=61860",
        "timing_guest_trace_pending_receive_wait_ms=35103",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_stream_segments=477",
        "timing_guest_trace_owned_streaming_lower_segments=0",
        "timing_guest_trace_seed_direct_lift_attempts=476",
        "timing_guest_trace_seed_direct_lift_successes=476",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_stage_leaf_kernel_work_ms=9170",
        "timing_guest_stage_tree_commit_root_count=477",
        "timing_guest_stage_tree_commit_root_materialization_groups=477",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "avoid_seed_ready_streamed_lower_reexecution"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "avoid_seed_ready_streamed_lower_reexecution"
    );
}

#[test]
fn prove_timing_root_summary_flags_live_stream_segment_serial_bound() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=43100",
        "timing_guest_trace_lowerer_ms=43080",
        "timing_guest_trace_lower_ms=32000",
        "timing_guest_trace_stream_elapsed_ms=43120",
        "timing_guest_trace_stream_ms=34500",
        "timing_guest_segment_commit_ms=8000",
        "timing_guest_trace_segment_receive_wait_ms=22000",
        "timing_guest_trace_pending_receive_wait_ms=0",
        "timing_guest_trace_parallel_lower_workers=8",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_job_receive_wait_ms=260000",
        "timing_guest_trace_parallel_lower_result_send_wait_ms=0",
        "timing_guest_trace_parallel_lower_stream_chunk_process_ms=30000",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=43000",
        "timing_guest_trace_report_chunk_sent=22916",
        "timing_guest_trace_report_chunk_received=22916",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_stage_leaf_kernel_work_ms=4499",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "parallel_lower_live_stream_segment_serial_bound"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "parallel_lower_live_stream_segment_serial_bound"
    );
}

#[test]
fn prove_timing_root_summary_requires_seed_lift_before_parallel_reexecution() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=39146",
        "timing_guest_trace_lowerer_ms=39146",
        "timing_guest_trace_lower_ms=30784",
        "timing_guest_trace_stream_elapsed_ms=39272",
        "timing_guest_trace_stream_ms=19164",
        "timing_guest_segment_commit_ms=20107",
        "timing_guest_trace_segment_receive_wait_ms=19162",
        "timing_guest_trace_pending_receive_wait_ms=39145",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=39144",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=40",
        "timing_guest_trace_seed_direct_lift_boundary_c_unavailable=79",
        "timing_guest_stage_leaf_kernel_work_ms=4499",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(value("trace_structure_hint"), "parallel_lower_runner_bound");
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "profile_boundary_c_unavailable"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "seed_direct_lift_before_parallel_reexecution"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "seed_direct_lift_before_parallel_reexecution"
    );
}

#[test]
fn prove_timing_root_summary_reports_replay_duplicate_work() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=39146",
        "timing_guest_trace_lowerer_ms=39146",
        "timing_guest_trace_lower_ms=30784",
        "timing_guest_trace_stream_elapsed_ms=39272",
        "timing_guest_trace_stream_ms=19164",
        "timing_guest_segment_commit_ms=20107",
        "timing_guest_trace_segment_receive_wait_ms=19162",
        "timing_guest_trace_pending_receive_wait_ms=39145",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=120",
        "timing_guest_trace_parallel_lower_snapshot_replay_ms=31787",
        "timing_guest_trace_parallel_lower_report_elided_count=120",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=39144",
        "timing_cuda_allocator_copy_h2d_bytes=88120305624",
        "timing_guest_stage_leaf_kernel_work_ms=4499",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=1",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=3",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=11",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(
        value("trace_structure_hint"),
        "parallel_lower_replay_duplicate_work"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "avoid_replay_only_parallel_lower"
    );
    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "retained_parent_checkpoint_path_time_secondary"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "avoid_replay_only_parallel_lower"
    );
}

#[test]
fn prove_timing_root_summary_reports_worker_result_bound_parallel_lower() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48121",
        "timing_guest_trace_runner_ms=12000",
        "timing_guest_trace_lowerer_ms=39146",
        "timing_guest_trace_lower_ms=39146",
        "timing_guest_trace_stream_elapsed_ms=39272",
        "timing_guest_trace_stream_ms=19164",
        "timing_guest_segment_commit_ms=8000",
        "timing_guest_trace_segment_receive_wait_ms=1000",
        "timing_guest_trace_pending_receive_wait_ms=1000",
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_parallel_lower_dispatched=120",
        "timing_guest_trace_parallel_lower_received=120",
        "timing_guest_trace_parallel_lower_emitted=120",
        "timing_guest_trace_parallel_lower_result_receive_wait_ms=30100",
        "timing_guest_stage_leaf_kernel_work_ms=4499",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(
        value("trace_structure_hint"),
        "parallel_lower_worker_result_bound"
    );
    assert_eq!(value("parallel_lower_result_receive_wait_ms"), "30100");
}

#[test]
fn prove_timing_root_summary_reports_twelve_second_gap_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=55200",
        "timing_guest_trace_runner_ms=42100",
        "timing_guest_trace_lowerer_ms=43800",
        "timing_guest_trace_stream_elapsed_ms=44100",
        "timing_guest_trace_stream_ms=39700",
        "timing_guest_segment_commit_ms=10700",
        "timing_finish_witness_opening_ms=3200",
        "timing_guest_stage_leaf_kernel_work_ms=6200",
        "timing_guest_stage_tree_commit_root_count=79",
        "timing_guest_stage_tree_commit_root_materialization_groups=79",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("summary should include {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should include {name}: stdout={stdout}"))
    };
    assert_eq!(value("proof_12s_gap_ms"), "43200");
    assert_eq!(
        value("proof_12s_gap_hint"),
        "cpu_trace_generation_above_target"
    );
}

#[test]
fn prove_timing_root_summary_distinguishes_disabled_high32_stats_from_zero_high32_stats() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let disabled_input = [
        "timing_total_ms=8000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_descriptor_rows=1000",
        "timing_guest_trace_descriptor_compact_rows=1000",
        "timing_guest_trace_descriptor_wide_rows=0",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values=0",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows=0",
    ]
    .join("\n");
    let enabled_zero_input = [
        "timing_total_ms=8000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_descriptor_rows=1000",
        "timing_guest_trace_descriptor_compact_rows=1000",
        "timing_guest_trace_descriptor_wide_rows=0",
        "timing_guest_trace_descriptor_high32_stats_enabled=1",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_values=0",
        "timing_guest_trace_descriptor_unpaired_high32_nonzero_rows=0",
    ]
    .join("\n");

    for (label, input, expected_hint) in [
        (
            "disabled",
            disabled_input,
            "compact_descriptor_no_high32_stats",
        ),
        (
            "enabled-zero",
            enabled_zero_input,
            "high32_zero_compact_descriptor",
        ),
    ] {
        let mut child = Command::new("python3")
            .arg(&script_path)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("{label} summary should spawn: {error}"));
        child
            .stdin
            .as_mut()
            .expect("stdin should be open")
            .write_all(input.as_bytes())
            .expect("stdin should write");
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("{label} summary should run: {error}"));

        assert!(
            output.status.success(),
            "{label} summary should pass: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(
            stdout.contains(expected_hint),
            "{label} summary should report {expected_hint}: stdout={stdout}"
        );
    }
}

#[test]
fn prove_timing_root_summary_uses_thread_name_for_memmove_source_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=1000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    21.23%    21.23%  lzvm-gp-runner  libc.so.6             [.] __memmove_avx512_unaligned_erms",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("summary should include {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should include {name}: stdout={stdout}"))
    };
    assert_eq!(value("perf_memmove_source_hint"), "guest_runner_thread");
    assert_eq!(value("cpu_trace_hotspot_hint"), "guest_state_copies");
}

#[test]
fn prove_timing_root_summary_reads_sibling_perf_report() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-perf-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let perf_report_path = temp_dir.join("sample.perf.report");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=1000",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &perf_report_path,
        [
            "    22.58%  [.] lzvm_prover::guest_pc_trace_backend::apply_main_lowered_report_row",
            "    17.70%  [.] __memmove_avx512_unaligned_erms",
            "     4.70%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
        ]
        .join("\n"),
    )
    .expect("sibling perf report should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains(
            ",22.580,17.700,0.000,0.000,none,4.700,0.000,none,report_lifetime_and_data_movement"
        ),
        "prove timing root summary should merge sibling perf report hotspots: stdout={stdout}"
    );
}

#[test]
fn prove_timing_root_summary_reads_sibling_nsys_cpu_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-nsys-cpu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let cpu_summary_path = temp_dir.join("sample.cpu-summary.txt");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=8314",
            "timing_guest_trace_parallel_lower_workers=8",
            "timing_guest_trace_parallel_lower_job_receive_wait_ms=51589",
            "timing_guest_trace_report_chunk_sent=22916",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &cpu_summary_path,
        [
            "application_cpu_hotspots",
            "symbol,module,samples,application_sample_pct",
            "lzvm_prover::guest_pc_trace_backend::apply_main_lowered_report_row,/path/lzvm,2884,34.975",
            "lzvm_prover::guest_pc_trace_backend::produce_guest_pc_trace_live_pending_messages,/path/lzvm,1484,16.322",
            "lzvm_prover::guest_pc_trace_backend::ZiskMainOwnedStreamingDeviceReportFeeder::push_report,/path/lzvm,1047,11.516",
            "lzvm_prover::guest_pc_trace_backend::emit_guest_pc_trace_live_pending_segment_messages,/path/lzvm,508,5.587",
            "lzvm_prover::guest_machine::advance_guest_machine_prepared_inner,/path/lzvm,1066,12.927",
            "core::ptr::drop_in_place$LT$lzvm_prover..guest_pc_trace_backend..GuestPcTracePendingSegmentSlice$GT$::hash,/path/lzvm,376,4.560",
        ]
        .join("\n"),
    )
    .expect("sibling nsys CPU summary should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_lowered_report_row_self_pct"), "34.975");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "12.927");
    assert_eq!(value("perf_pending_segment_drop_self_pct"), "4.560");
    assert_eq!(value("perf_live_stream_message_self_pct"), "33.425");
    assert_eq!(
        value("cpu_trace_live_stream_action_hint"),
        "reduce_live_report_message_overhead"
    );
    assert_eq!(value("cpu_runner_hotspot_hint"), "guest_machine_advance");
}

#[test]
fn prove_timing_root_summary_reads_csv_sibling_nsys_cpu_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-nsys-cpu-csv-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let cpu_summary_path = temp_dir.join("sample.cpu-summary.csv");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=50311",
            "timing_guest_trace_reports=499520693",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &cpu_summary_path,
        [
            "application_cpu_hotspots",
            "symbol,module,samples,application_sample_pct",
            "lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row,/path/lzvm,11598,25.745",
            "lzvm_prover::guest_pc_trace_backend::run_guest_pc_trace_segment_slice,/path/lzvm,8307,18.440",
            "lzvm_prover::guest_machine::advance_guest_machine_prepared_inner_with_report_shape,/path/lzvm,6458,14.338",
            "lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor,/path/lzvm,3069,6.817",
        ]
        .join("\n"),
    )
    .expect("sibling nsys CPU summary should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_lowered_report_row_self_pct"), "25.745");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "14.338");
    assert_eq!(value("perf_append_descriptor_self_pct"), "6.817");
}

#[test]
fn prove_timing_root_summary_reads_real_nsys_cpu_trace_hints() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-real-nsys-cpu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let cpu_summary_path = temp_dir.join("sample.cpu-summary.txt");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=8662",
            "timing_guest_trace_reports=93843537",
            "timing_guest_trace_report_record_size_bytes=144",
            "timing_guest_trace_report_storage_bytes=13513469328",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &cpu_summary_path,
        [
            "top_cpu_self_samples",
            "symbol,module,samples,cpu_sample_pct",
            "__memcpy_avx512_unaligned_erms,/usr/lib64/libc.so.6,237,2.849",
            "application_cpu_hotspots",
            "symbol,module,samples,application_sample_pct",
            concat!(
                "lzvm_prover::guest_pc_trace_backend::apply_",
                "z",
                "isk_main_lowered_report_row::h9c857518d59ff394,/path/lzvm,2851,34.197"
            ),
            "lzvm_prover::guest_instruction::decode_riscv_instruction::h8b811b1d61b1f884,/path/lzvm,35,0.420",
            "cpu_trace_memcpy_action_hints",
            "nearest_app_symbol,samples,libc_sample_pct,action_hint",
            "lzvm_prover::guest_pc_trace_backend::run_guest_pc_trace_segment_slice::ha818f4736a72f376,51,61.446,trace_report_storage_structural_candidate",
        ]
        .join("\n"),
    )
    .expect("sibling nsys CPU summary should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_lowered_report_row_self_pct"), "34.197");
    assert_eq!(value("perf_memmove_self_pct"), "2.849");
    assert_eq!(value("perf_decode_instruction_self_pct"), "0.420");
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "trace_report_storage_memcpy_secondary"
    );
    assert_eq!(value("cpu_trace_memcpy_report_storage_hint_pct"), "61.446");
    assert_eq!(value("cpu_trace_memcpy_report_storage_total_pct"), "1.751");
}

#[test]
fn prove_timing_root_summary_reads_short_sibling_nsys_cpu_summary() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let temp_dir = crate_root.join("../../temp").join(format!(
        "prove-timing-root-summary-short-nsys-cpu-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("test temp directory should be created");
    let log_path = temp_dir.join("sample.log");
    let cpu_summary_path = temp_dir.join("sample.cpu.txt");
    std::fs::write(
        &log_path,
        [
            "timing_total_ms=48330",
            "timing_guest_stage_tree_commit_root_count=1",
            "timing_guest_stage_tree_commit_root_materialization_groups=1",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        ]
        .join("\n"),
    )
    .expect("timing log should be written");
    std::fs::write(
        &cpu_summary_path,
        [
            "application_cpu_hotspots",
            "symbol,module,samples,application_sample_pct",
            "lzvm_prover::guest_machine::prepare_current_guest_instruction,/path/lzvm,971,11.234",
            "lzvm_prover::guest_machine::advance_guest_machine_prepared_inner,/path/lzvm,1094,12.663",
        ]
        .join("\n"),
    )
    .expect("short sibling nsys CPU summary should be written");

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&log_path)
        .output()
        .expect("prove timing root summary should run");
    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_prepare_instruction_self_pct"), "11.234");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "12.663");
    assert_eq!(
        value("cpu_runner_hotspot_hint"),
        "instruction_prepare_and_advance"
    );
}

#[test]
fn prove_timing_root_summary_reports_trace_report_storage_action_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=8076",
        "timing_guest_trace_reports=93843537",
        "timing_guest_trace_report_record_size_bytes=144",
        "timing_guest_trace_report_storage_bytes=13513469328",
        "timing_guest_trace_report_buffer_capacity=94371840",
        "timing_guest_trace_report_buffer_max_capacity=4194304",
        "timing_guest_trace_report_buffer_excess_capacity=528303",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    18.96%  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "    10.73%  [.] memmove",
        "     8.67%  [.] lzvm_prover::guest_pc_trace_backend::GuestPcTraceSegmentSlice::from_segment_trace",
        "     5.72%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("cpu_trace_hotspot_hint"),
        "report_lifetime_and_data_movement"
    );
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "fused_runner_lowerer_report_storage_candidate"
    );
}

#[test]
fn prove_timing_root_summary_flags_missing_trace_report_storage_fields() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=51642",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_runner_ms=42305",
        "timing_guest_trace_lowerer_ms=42305",
        "timing_guest_trace_stream_elapsed_ms=42310",
        "timing_guest_trace_segment_receive_wait_ms=22093",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_report_record_size_bytes"), "0");
    assert_eq!(
        value("cpu_trace_report_storage_action_hint"),
        "refresh_trace_report_storage_timing"
    );
}

#[test]
fn prove_timing_root_summary_reports_lowerer_perf_action_hint() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=50750",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    16.21%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     6.12%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor",
        "     2.92%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::zisk_main_source_value",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_append_descriptor_self_pct"), "6.120");
    assert_eq!(value("perf_source_value_self_pct"), "2.920");
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "descriptor_append_candidate"
    );
}

#[test]
fn prove_timing_root_summary_prefers_source_value_when_close_to_descriptor_append() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=51876",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    22.25%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     8.00%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor",
        "     7.06%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::zisk_main_source_value",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_append_descriptor_self_pct"), "8.000");
    assert_eq!(value("perf_source_value_self_pct"), "7.060");
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "source_value_candidate"
    );
}

#[test]
fn prove_timing_root_summary_prefers_detail_lowerer_hotspot_over_perf_symbol() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=50870",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lowerer_ms=32174",
        "timing_guest_trace_lower_ms=31203",
        "timing_guest_trace_reports=500000000",
        "timing_guest_trace_report_rows=500000000",
        "timing_guest_trace_report_detail_samples=5000",
        "timing_guest_trace_report_sampled_ns=100000",
        "timing_guest_trace_report_row_validation_sampled_ns=56000",
        "timing_guest_trace_report_source_values_sampled_ns=14000",
        "timing_guest_trace_report_precompile_memory_sampled_ns=3000",
        "timing_guest_trace_report_instruction_result_sampled_ns=4500",
        "timing_guest_trace_report_next_pc_sampled_ns=2000",
        "timing_guest_trace_report_register_access_sampled_ns=4000",
        "timing_guest_trace_report_memory_access_sampled_ns=5000",
        "timing_guest_trace_report_store_apply_sampled_ns=4000",
        "timing_guest_trace_report_visit_sampled_ns=16000",
        "timing_guest_trace_descriptor_sampled_ns=10000",
        "    24.74%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     8.50%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor",
        "     3.96%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::zisk_main_source_value",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_report_detail_action_hint"),
        "profile_row_validation"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "row_validation_profile_candidate"
    );
}

#[test]
fn prove_timing_root_summary_ignores_source_value_record_bookkeeping_as_runtime_hotspot() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=49119",
        "timing_guest_trace_runner_ms=39999",
        "timing_guest_trace_lowerer_ms=39999",
        "timing_guest_trace_lower_ms=28196",
        "timing_guest_trace_stream_elapsed_ms=40000",
        "timing_guest_trace_stream_ms=19872",
        "timing_guest_segment_commit_ms=20128",
        "timing_guest_trace_segment_receive_wait_ms=19870",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_detail_samples=50000",
        "timing_guest_trace_report_sampled_ns=100000",
        "timing_guest_trace_report_row_validation_sampled_ns=60000",
        "timing_guest_trace_report_source_values_sampled_ns=24000",
        "timing_guest_trace_report_source_a_value_sampled_ns=5000",
        "timing_guest_trace_report_source_b_value_sampled_ns=5000",
        "timing_guest_trace_report_source_value_record_sampled_ns=12000",
        "timing_guest_trace_report_instruction_result_sampled_ns=5000",
        "timing_guest_trace_report_next_pc_sampled_ns=3000",
        "timing_guest_trace_report_register_access_sampled_ns=4000",
        "timing_guest_trace_report_memory_access_sampled_ns=4000",
        "timing_guest_trace_report_store_apply_sampled_ns=3000",
        "timing_guest_trace_report_visit_sampled_ns=10000",
        "timing_guest_trace_descriptor_sampled_ns=2000",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_report_row_validation_hotspot"),
        "source_value_record"
    );
    assert_eq!(
        value("trace_report_detail_action_hint"),
        "profile_row_validation_residual"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "row_validation_residual_profile_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "row_validation_residual_profile_candidate"
    );
}

#[test]
fn prove_timing_root_summary_classifies_row_validation_timer_bookkeeping_as_diagnostic_overhead() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_lowerer_ms=8000",
        "timing_guest_trace_lower_ms=7000",
        "timing_guest_trace_report_rows=1000",
        "timing_guest_trace_report_detail_samples=10",
        "timing_guest_trace_report_sampled_ns=140000",
        "timing_guest_trace_report_row_validation_sampled_ns=100000",
        "timing_guest_trace_report_row_validation_timer_bookkeeping_sampled_ns=45000",
        "timing_guest_trace_report_source_values_sampled_ns=18000",
        "timing_guest_trace_report_source_a_value_sampled_ns=9000",
        "timing_guest_trace_report_source_b_value_sampled_ns=8000",
        "timing_guest_trace_report_source_value_record_sampled_ns=1000",
        "timing_guest_trace_report_instruction_result_sampled_ns=8000",
        "timing_guest_trace_report_next_pc_sampled_ns=6000",
        "timing_guest_trace_report_register_access_sampled_ns=7000",
        "timing_guest_trace_report_memory_access_sampled_ns=6000",
        "timing_guest_trace_report_store_apply_sampled_ns=5000",
        "timing_guest_trace_report_precompile_memory_sampled_ns=4000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_report_row_validation_hotspot"),
        "timer_bookkeeping"
    );
    assert_eq!(
        value("trace_report_detail_action_hint"),
        "detail_timing_bookkeeping_overhead"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "detail_timing_bookkeeping_overhead"
    );
    assert_ne!(
        value("performance_focus_hint"),
        "row_validation_profile_candidate"
    );
}

#[test]
fn prove_timing_root_summary_requires_shape_before_row_validation_residual_probe() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=47000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_lowerer_ms=38131",
        "timing_guest_trace_lower_ms=27911",
        "timing_guest_trace_reports=499520693",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_detail_samples=121975",
        "timing_guest_trace_report_sampled_ns=71000000",
        "timing_guest_trace_report_row_validation_sampled_ns=56000000",
        "timing_guest_trace_report_source_values_sampled_ns=12544572",
        "timing_guest_trace_report_instruction_result_sampled_ns=2487553",
        "timing_guest_trace_report_next_pc_sampled_ns=2195432",
        "timing_guest_trace_report_register_access_sampled_ns=3188792",
        "timing_guest_trace_report_memory_access_sampled_ns=2408014",
        "timing_guest_trace_report_store_apply_sampled_ns=2491891",
        "timing_guest_trace_report_visit_sampled_ns=10448566",
        "timing_guest_trace_descriptor_sampled_ns=5580000",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_shape_sample_hint"),
        "shape_timing_missing_for_detail_profile"
    );
    assert_eq!(
        value("trace_report_detail_action_hint"),
        "enable_shape_timing_for_row_validation_residual"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "shape_timing_required_for_row_validation_residual"
    );
}

#[test]
fn prove_timing_root_summary_reports_segment_replay_count() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=9000",
        "timing_guest_trace_segment_replay_count=23",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=23",
        "timing_guest_trace_parallel_lower_snapshot_replay_ms=321",
        "timing_guest_trace_parallel_lower_report_elided_count=23",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };
    assert_eq!(value("segment_replay_count"), "23");
    assert_eq!(value("parallel_lower_snapshot_replay_count"), "23");
    assert_eq!(value("parallel_lower_snapshot_replay_ms"), "321");
    assert_eq!(value("parallel_lower_report_elided_count"), "23");
}

#[test]
fn prove_timing_root_summary_prioritizes_trace_pipeline_over_secondary_opening_launches() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=51505",
        "timing_guest_trace_runner_ms=42145",
        "timing_guest_trace_lowerer_ms=42195",
        "timing_guest_trace_lower_ms=38619",
        "timing_guest_trace_stream_elapsed_ms=42407",
        "timing_guest_segment_commit_ms=21355",
        "timing_guest_trace_segment_receive_wait_ms=22300",
        "timing_guest_trace_parallel_lower_workers=1",
        "timing_guest_trace_seed_direct_lift_attempts=1",
        "timing_guest_trace_seed_direct_lift_successes=0",
        "timing_guest_trace_seed_direct_lift_boundary_c_unavailable=1",
        "timing_guest_stage_leaf_kernel_work_ms=11000",
        "timing_finish_witness_opening_ms=9600",
        "timing_finish_witness_opening_query_unit_count=41",
        "timing_finish_witness_opening_single_query_unit_count=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=41",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=410",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=92",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=459",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=93",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "retained_parent_checkpoint_path_time_secondary"
    );
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "profile_boundary_c_unavailable"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "trace_pipeline_over_secondary_opening_launches"
    );
}

#[test]
fn prove_timing_root_summary_prioritizes_seed_snapshot_only_probe() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=48424",
        "timing_guest_trace_runner_ms=39357",
        "timing_guest_trace_lowerer_ms=39401",
        "timing_guest_trace_lower_ms=28401",
        "timing_guest_trace_stream_elapsed_ms=39536",
        "timing_guest_segment_commit_ms=20041",
        "timing_guest_trace_segment_receive_wait_ms=19493",
        "timing_guest_trace_pending_receive_wait_ms=7134",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=10",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=869",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=12",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "seed_direct_lift_ready"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "trace_generation_and_commit_pipeline_candidate"
    );
    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "retained_parent_checkpoint_path_time_secondary"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "trusted_seed_snapshot_seed_only_probe"
    );
    assert_eq!(
        value("seed_snapshot_runtime_hint"),
        "trusted_seed_snapshot_seed_only_probe"
    );
}

#[test]
fn prove_timing_root_summary_flags_untrusted_seed_snapshot_validation_overhead() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=63238",
        "timing_guest_trace_runner_ms=53597",
        "timing_guest_trace_lowerer_ms=53597",
        "timing_guest_trace_lower_ms=29802",
        "timing_guest_trace_stream_elapsed_ms=53597",
        "timing_guest_trace_stream_ms=21041",
        "timing_guest_segment_commit_ms=21041",
        "timing_guest_trace_segment_receive_wait_ms=32553",
        "timing_guest_trace_pending_receive_wait_ms=0",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=120",
        "timing_finish_witness_opening_query_unit_count=120",
        "timing_finish_witness_opening_single_query_unit_count=120",
        "timing_finish_witness_opening_query_count=120",
        "timing_finish_witness_opening_max_queries_per_unit=1",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=4",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=16",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "seed_direct_lift_ready"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "trace_generation_and_commit_pipeline_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "avoid_untrusted_seed_snapshot_validation"
    );
}

#[test]
fn prove_timing_root_summary_surfaces_profiled_seed_lift_miss_focus() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=113434",
        "timing_guest_trace_runner_ms=111640",
        "timing_guest_trace_lowerer_ms=111643",
        "timing_guest_trace_lower_ms=63004",
        "timing_guest_trace_stream_elapsed_ms=111644",
        "timing_guest_trace_stream_ms=85700",
        "timing_guest_segment_commit_ms=25943",
        "timing_guest_trace_segment_receive_wait_ms=111572",
        "timing_guest_trace_pending_receive_wait_ms=36050",
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_trace_seed_direct_lift_attempts=476",
        "timing_guest_trace_seed_direct_lift_successes=472",
        "timing_guest_trace_seed_direct_lift_store_conditional_boundaries=1",
        "timing_guest_trace_seed_direct_lift_boundary_c_unavailable=3",
        "timing_guest_trace_seed_full_advances=477",
        "timing_guest_stage_tree_commit_root_count=477",
        "timing_guest_stage_tree_commit_root_materialization_groups=477",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "profile_boundary_c_unavailable"
    );
    assert_eq!(
        value("trace_pipeline_action_hint"),
        "trace_generation_and_commit_pipeline_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "profile_boundary_c_unavailable"
    );
}

#[test]
fn prove_timing_root_summary_requests_seed_snapshot_profile_before_reexecution() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=49119",
        "timing_guest_trace_runner_ms=39999",
        "timing_guest_trace_lowerer_ms=39999",
        "timing_guest_trace_lower_ms=28196",
        "timing_guest_trace_stream_elapsed_ms=40000",
        "timing_guest_trace_stream_ms=19872",
        "timing_guest_segment_commit_ms=20128",
        "timing_guest_trace_segment_receive_wait_ms=19870",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_buffer_capacity=500170752",
        "timing_guest_trace_report_buffer_excess_capacity=650059",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=4",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=16",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_pipeline_action_hint"),
        "trace_generation_and_commit_pipeline_candidate"
    );
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "profile_runner_seed_snapshot"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "profile_runner_seed_snapshot_before_parallel_reexecution"
    );
}

#[test]
fn prove_timing_root_summary_prioritizes_trace_producer_gap_evidence() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=43457",
        "timing_guest_trace_runner_ms=43201",
        "timing_guest_trace_lowerer_ms=43201",
        "timing_guest_trace_lower_ms=33240",
        "timing_guest_trace_stream_elapsed_ms=43201",
        "timing_guest_trace_stream_ms=23937",
        "timing_guest_segment_commit_ms=19264",
        "timing_guest_segment_input_gap_ms=42647",
        "timing_guest_segment_input_gap_max_ms=550",
        "timing_guest_segment_input_gap_count=476",
        "timing_guest_segment_commit_worker_backpressure_join_ms=3",
        "timing_guest_trace_segment_receive_wait_ms=43151",
        "timing_guest_trace_pending_receive_wait_ms=5626",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_buffer_capacity=500170752",
        "timing_guest_trace_report_buffer_excess_capacity=650059",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_pipeline_action_hint"),
        "trace_producer_input_gap_dominant"
    );
    assert_eq!(value("seed_direct_lift_action_hint"), "none");
    assert_eq!(
        value("performance_focus_hint"),
        "trace_producer_input_gap_dominant"
    );
}

#[test]
fn prove_timing_root_summary_keeps_seed_profile_focus_over_perf_only_descriptor_symbol() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=49119",
        "timing_guest_trace_runner_ms=39999",
        "timing_guest_trace_lowerer_ms=39999",
        "timing_guest_trace_lower_ms=28196",
        "timing_guest_trace_stream_elapsed_ms=40000",
        "timing_guest_trace_stream_ms=19872",
        "timing_guest_segment_commit_ms=20128",
        "timing_guest_trace_segment_receive_wait_ms=19870",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_buffer_capacity=500170752",
        "timing_guest_trace_report_buffer_excess_capacity=650059",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "    22.25%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::apply_zisk_main_lowered_report_row",
        "     8.00%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "profile_runner_seed_snapshot"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "descriptor_append_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "profile_runner_seed_snapshot_before_parallel_reexecution"
    );
}

#[test]
fn prove_timing_root_summary_prefers_lowerer_detail_focus_over_seed_snapshot_profile() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=49119",
        "timing_guest_trace_runner_ms=39999",
        "timing_guest_trace_lowerer_ms=39999",
        "timing_guest_trace_lower_ms=28196",
        "timing_guest_trace_stream_elapsed_ms=40000",
        "timing_guest_trace_stream_ms=19872",
        "timing_guest_segment_commit_ms=20128",
        "timing_guest_trace_segment_receive_wait_ms=19870",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_buffer_capacity=500170752",
        "timing_guest_trace_report_buffer_excess_capacity=650059",
        "timing_guest_trace_report_detail_samples=121975",
        "timing_guest_trace_report_sampled_ns=71000000",
        "timing_guest_trace_report_row_validation_sampled_ns=56000000",
        "timing_guest_trace_report_source_values_sampled_ns=12544572",
        "timing_guest_trace_report_instruction_result_sampled_ns=2487553",
        "timing_guest_trace_report_next_pc_sampled_ns=2195432",
        "timing_guest_trace_report_register_access_sampled_ns=3188792",
        "timing_guest_trace_report_memory_access_sampled_ns=2408014",
        "timing_guest_trace_report_store_apply_sampled_ns=2491891",
        "timing_guest_trace_report_visit_sampled_ns=10448566",
        "timing_guest_trace_descriptor_sampled_ns=5580000",
        "timing_guest_trace_external_op_rows=237231598",
        "timing_guest_trace_copy_rows=253826801",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=79",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=79",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=4",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=790",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=16",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_report_detail_action_hint"),
        "split_row_validation_residual_timers"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "row_validation_residual_timer_split_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "row_validation_residual_timer_split_candidate"
    );
}

#[test]
fn prove_timing_root_summary_keeps_seed_snapshot_focus_over_perf_only_descriptor_symbol() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=49119",
        "timing_guest_trace_runner_ms=39999",
        "timing_guest_trace_lowerer_ms=39999",
        "timing_guest_trace_lower_ms=28196",
        "timing_guest_trace_stream_elapsed_ms=40000",
        "timing_guest_trace_stream_ms=19872",
        "timing_guest_segment_commit_ms=20128",
        "timing_guest_trace_segment_receive_wait_ms=19870",
        "timing_guest_trace_report_rows=499917240",
        "timing_guest_trace_report_buffer_capacity=500170752",
        "timing_guest_trace_report_buffer_excess_capacity=650059",
        "timing_guest_trace_seed_direct_lift_attempts=119",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_stage_tree_commit_root_count=120",
        "timing_guest_stage_tree_commit_root_materialization_groups=120",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "     8.00%  lzvm-gp-lower    lzvm                  [.] lzvm_prover::guest_pc_trace_backend::append_main_device_trace_descriptor",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(
        value("trace_pipeline_action_hint"),
        "trace_generation_and_commit_pipeline_candidate"
    );
    assert_eq!(
        value("seed_direct_lift_action_hint"),
        "seed_direct_lift_ready"
    );
    assert_eq!(
        value("cpu_trace_lowerer_action_hint"),
        "descriptor_append_candidate"
    );
    assert_eq!(
        value("performance_focus_hint"),
        "trusted_seed_snapshot_seed_only_probe"
    );
    assert_eq!(
        value("seed_snapshot_runtime_hint"),
        "trusted_seed_snapshot_seed_only_probe"
    );
}

#[test]
fn prove_timing_root_summary_suppresses_secondary_opening_focus_when_trace_target_is_met() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=8500",
        "timing_guest_trace_runner_ms=7158",
        "timing_guest_trace_lowerer_ms=7419",
        "timing_guest_trace_lower_ms=5922",
        "timing_guest_trace_stream_elapsed_ms=7603",
        "timing_guest_segment_commit_ms=2092",
        "timing_guest_trace_segment_receive_wait_ms=5510",
        "timing_guest_trace_parallel_lower_workers=1",
        "timing_guest_stage_leaf_kernel_work_ms=2600",
        "timing_finish_witness_opening_ms=441",
        "timing_finish_witness_opening_query_unit_count=41",
        "timing_finish_witness_opening_single_query_unit_count=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_openings=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_rows=41",
        "timing_finish_witness_opening_retained_parent_checkpoint_all_single_row_openings=41",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=410",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=92",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=459",
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=93",
        "timing_guest_stage_tree_commit_root_count=23",
        "timing_guest_stage_tree_commit_root_materialization_groups=23",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_pipeline_action_hint"), "within_target");
    assert_eq!(
        value("opening_retained_parent_checkpoint_action_hint"),
        "retained_parent_checkpoint_path_time_secondary"
    );
    assert_eq!(value("performance_focus_hint"), "none");
}

#[test]
fn prove_timing_root_summary_reports_runner_perf_hotspots() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=1000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "     5.30%  [.] lzvm_prover::guest_machine::prepare_current_guest_instruction",
        "     5.17%  [.] lzvm_prover::guest_pc_trace_backend::build_layout_zisk_main_trace_segment_for_segment_output",
        "     4.26%  [.] lzvm_prover::guest_machine::advance_guest_machine_prepared_inner",
        "     2.81%  [.] lzvm_prover::guest_machine::memory::GuestMachineMemorySegment::write_range",
        "     1.97%  [.] num_bigint::biguint::monty::monty_modpow",
        "     1.70%  [.] lzvm_prover::guest_machine::memory::GuestMachineMemory::read_range_into",
        "     0.26%  [.] lzvm_prover::guest_machine::GuestInstructionEffects::record_memory_write",
        "     0.14%  [.] lzvm_prover::guest_machine::GuestInstructionEffects::record_memory_read",
        "     0.03%  [.] lzvm_prover::guest_instruction::decode_guest_instruction",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines
        .next()
        .expect("summary should print a header")
        .split(',')
        .collect::<Vec<_>>();
    let row = lines
        .next()
        .expect("summary should print one row")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(
        header.len(),
        row.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("summary should include {name}: stdout={stdout}"));
        row.get(index)
            .copied()
            .unwrap_or_else(|| panic!("summary row should include {name}: stdout={stdout}"))
    };

    assert_eq!(value("perf_prepare_instruction_self_pct"), "5.300");
    assert_eq!(value("perf_trace_segment_build_self_pct"), "5.170");
    assert_eq!(value("perf_advance_guest_machine_self_pct"), "4.260");
    assert_eq!(value("perf_guest_memory_write_self_pct"), "2.810");
    assert_eq!(value("perf_biguint_modpow_self_pct"), "1.970");
    assert_eq!(value("perf_guest_memory_read_self_pct"), "1.700");
    assert_eq!(value("perf_decode_instruction_self_pct"), "0.030");
    assert_eq!(value("perf_effect_record_memory_write_self_pct"), "0.260");
    assert_eq!(value("perf_effect_record_memory_read_self_pct"), "0.140");
    assert_eq!(
        value("cpu_runner_hotspot_hint"),
        "instruction_prepare_and_advance"
    );
}

#[test]
fn prove_timing_root_summary_reports_runner_detail_hotspot() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=10000",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
        "timing_guest_trace_runner_ms=8000",
        "timing_guest_trace_reports=1000",
        "timing_guest_trace_runner_detail_samples=10",
        "timing_guest_trace_runner_detail_sampled_ns=1000000",
        "timing_guest_trace_runner_prepare_instruction_sampled_ns=100000",
        "timing_guest_trace_runner_pre_boundary_sampled_ns=40000",
        "timing_guest_trace_runner_row_plan_sampled_ns=100000",
        "timing_guest_trace_runner_cache_policy_sampled_ns=30000",
        "timing_guest_trace_runner_advance_sampled_ns=500000",
        "timing_guest_trace_runner_advance_setup_sampled_ns=70000",
        "timing_guest_trace_runner_advance_execute_sampled_ns=350000",
        "timing_guest_trace_runner_advance_report_sampled_ns=40000",
        "timing_guest_trace_runner_cache_update_sampled_ns=50000",
        "timing_guest_trace_runner_row_count_sampled_ns=50000",
        "timing_guest_trace_runner_post_boundary_sampled_ns=60000",
        "timing_guest_trace_runner_counter_update_sampled_ns=20000",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = parse_csv_line(lines.next().expect("summary should print a header"));
    let row = parse_csv_line(lines.next().expect("summary should print one row"));
    let value = |name: &str| {
        let index = header
            .iter()
            .position(|header| header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        row.get(index)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("summary row should contain {name}: stdout={stdout}"))
    };

    assert_eq!(value("trace_runner_detail_samples"), "10");
    assert_eq!(value("trace_runner_detail_sample_pct"), "1.000");
    assert_eq!(value("trace_runner_detail_avg_ns"), "100000");
    assert_eq!(
        value("trace_runner_prepare_instruction_sampled_ns"),
        "100000"
    );
    assert_eq!(value("trace_runner_pre_boundary_sampled_ns"), "40000");
    assert_eq!(value("trace_runner_row_plan_sampled_ns"), "100000");
    assert_eq!(value("trace_runner_cache_policy_sampled_ns"), "30000");
    assert_eq!(value("trace_runner_advance_sampled_ns"), "500000");
    assert_eq!(value("trace_runner_advance_setup_sampled_ns"), "70000");
    assert_eq!(value("trace_runner_advance_execute_sampled_ns"), "350000");
    assert_eq!(value("trace_runner_advance_report_sampled_ns"), "40000");
    assert_eq!(value("trace_runner_cache_update_sampled_ns"), "50000");
    assert_eq!(value("trace_runner_row_count_sampled_ns"), "50000");
    assert_eq!(value("trace_runner_post_boundary_sampled_ns"), "60000");
    assert_eq!(value("trace_runner_counter_update_sampled_ns"), "20000");
    assert_eq!(value("trace_runner_detail_hotspot"), "advance");
    assert_eq!(value("trace_runner_detail_hotspot_pct"), "50.000");
    assert_eq!(value("trace_runner_detail_residual_pct"), "5.000");
    assert_eq!(
        value("trace_runner_detail_action_hint"),
        "profile_guest_machine_advance"
    );
}

#[test]
fn prove_timing_root_summary_reports_constant_material_overlap() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/prove-timing-root-summary.py");
    let input = [
        "timing_total_ms=10000",
        "timing_constant_material_validation_elapsed_ms=9000",
        "timing_constant_material_validation_join_wait_ms=125",
        "timing_guest_stage_tree_commit_root_count=1",
        "timing_guest_stage_tree_commit_root_materialization_groups=1",
        "timing_guest_stage_tree_commit_root_materialization_max_group_size=1",
    ]
    .join("\n");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("prove timing root summary should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(input.as_bytes())
        .expect("stdin should write");
    let output = child
        .wait_with_output()
        .expect("prove timing root summary should run");

    assert!(
        output.status.success(),
        "prove timing root summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let mut lines = stdout.lines();
    let header = lines.next().expect("summary should print a header");
    let row = lines.next().expect("summary should print one row");
    let headers = header.split(',').collect::<Vec<_>>();
    let values = row.split(',').collect::<Vec<_>>();
    assert_eq!(
        headers.len(),
        values.len(),
        "summary header and row should have matching column counts: stdout={stdout}"
    );
    let value = |name: &str| {
        let index = headers
            .iter()
            .position(|header| *header == name)
            .unwrap_or_else(|| panic!("summary should expose {name}: stdout={stdout}"));
        values[index]
    };
    assert_eq!(value("constant_material_validation_elapsed_ms"), "9000");
    assert_eq!(value("constant_material_validation_join_wait_ms"), "125");
    assert_eq!(
        value("constant_material_validation_overlap_hint"),
        "mostly_overlapped"
    );
}
