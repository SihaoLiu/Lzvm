use super::args::parsed_inputs;
use super::timing::{
    prover_gpu_mode, write_timing_entries, write_timing_summary, TimingCountEntry, TimingEntry,
};
use super::*;
use lzvm_artifacts::eth_block_input::parse_eth_block_input;
use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;

#[test]
fn rejects_trace_bytes_with_all_units_during_parse() {
    let result = parse_witness_args(&[
        "--trace-bytes",
        "trace.bin",
        "--all-units",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--trace-bytes requires a single-unit witness run"
    ));
}

#[test]
fn rejects_trace_bytes_with_aggregate_during_parse() {
    let result = parse_witness_args(&[
        "--trace-bytes",
        "trace.bin",
        "--aggregate",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--trace-bytes requires a single-unit witness run"
    ));
}

#[test]
fn parses_guest_pc_trace_option_for_witness_args() {
    let result = parse_witness_args(&[
        "--guest-pc-trace",
        "64",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ])
    .expect("witness args should parse");
    let inputs = parsed_inputs(&result);

    assert_eq!(result.guest_pc_trace_instruction_limit, Some(64));
    assert_eq!(inputs.witness_library, None);
    assert_eq!(inputs.guest_image, std::path::PathBuf::from("guest.elf"));
}

#[test]
fn parses_timings_option_for_witness_args() {
    let result = parse_witness_args(&[
        "--timings",
        "--guest-pc-trace",
        "64",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert!(result.timings);
}

#[test]
fn rejects_duplicate_timings_option() {
    let result = parse_witness_args(&[
        "--timings",
        "--timings",
        "--guest-pc-trace",
        "64",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message)) if message == "duplicate --timings option"
    ));
}

#[cfg(not(feature = "cuda"))]
#[test]
fn rejects_large_guest_pc_trace_without_gpu_backend() {
    assert_eq!(
        validate_large_guest_pc_gpu(Some(1_000_000))
            .expect_err("large guest PC trace should require a GPU backend"),
        "large --guest-pc-trace runs require a CUDA-enabled lzvm-cli build"
    );
    assert!(validate_large_guest_pc_gpu(Some(999_999)).is_ok());
    assert!(validate_large_guest_pc_gpu(None).is_ok());
}

#[test]
fn writes_timing_summary_lines() {
    let mut stdout = Vec::new();
    write_timing_entries(
        &mut stdout,
        &[
            TimingEntry {
                name: "witness".to_owned(),
                duration: std::time::Duration::from_millis(23),
            },
            TimingEntry {
                name: "proof".to_owned(),
                duration: std::time::Duration::from_millis(7),
            },
        ],
        &[],
        std::time::Duration::from_millis(31),
    );

    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");
    assert_eq!(
        stdout,
        format!(
            "prover_gpu_mode={}\ntiming_witness_ms=23\ntiming_proof_ms=7\ntiming_total_ms=31\n",
            prover_gpu_mode()
        )
    );
}

#[test]
fn writes_timing_count_summary_lines_without_ms_suffix() {
    let mut stdout = Vec::new();
    write_timing_entries(
        &mut stdout,
        &[TimingEntry {
            name: "witness".to_owned(),
            duration: std::time::Duration::from_millis(23),
        }],
        &[
            TimingCountEntry {
                name: "descriptor_upload_bytes".to_owned(),
                value: 88,
            },
            TimingCountEntry {
                name: "descriptor_upload_rows".to_owned(),
                value: 1,
            },
        ],
        std::time::Duration::from_millis(31),
    );

    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");
    assert_eq!(
        stdout,
        format!(
            "prover_gpu_mode={}\ntiming_witness_ms=23\ntiming_descriptor_upload_bytes=88\ntiming_descriptor_upload_rows=1\ntiming_total_ms=31\n",
            prover_gpu_mode()
        )
    );
}

#[cfg(feature = "cuda")]
#[test]
fn writes_cuda_copy_site_timing_summary_lines() {
    let mut timings = TimingRecorder::new(true);
    super::timing::record_cuda_copy_site_timing_entries(
        &mut timings,
        &[
            lzvm_prover::CudaCopySiteStat {
                label: "copy_from",
                file: "crates/lzvm-prover/src/witness/upload.rs",
                line: 37,
                calls: 2,
                bytes: 128,
                max_bytes: 64,
                wait_ns: 15,
                max_wait_ns: 9,
            },
            lzvm_prover::CudaCopySiteStat {
                label: "copy_prefix_from_u64_words",
                file: "crates/lzvm-prover/src/witness/layout.rs",
                line: 92,
                calls: 3,
                bytes: 96,
                max_bytes: 32,
                wait_ns: 21,
                max_wait_ns: 8,
            },
        ],
    );

    let mut stdout = Vec::new();
    write_timing_summary(&mut stdout, &timings);
    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");

    for expected in [
        "timing_cuda_copy_site_h2d_top_1_copy_from_upload_rs_37_calls=2\n",
        "timing_cuda_copy_site_h2d_top_1_copy_from_upload_rs_37_bytes=128\n",
        "timing_cuda_copy_site_h2d_top_1_copy_from_upload_rs_37_max_bytes=64\n",
        "timing_cuda_copy_site_h2d_top_1_copy_from_upload_rs_37_wait_ns=15\n",
        "timing_cuda_copy_site_h2d_top_1_copy_from_upload_rs_37_max_wait_ns=9\n",
        "timing_cuda_copy_site_h2d_top_1_copy_from_upload_rs_37_avg_wait_per_call_ns=7\n",
        "timing_cuda_copy_site_h2d_top_2_copy_prefix_from_u64_words_layout_rs_92_calls=3\n",
        "timing_cuda_copy_site_h2d_top_2_copy_prefix_from_u64_words_layout_rs_92_bytes=96\n",
        "timing_cuda_copy_site_h2d_top_2_copy_prefix_from_u64_words_layout_rs_92_max_bytes=32\n",
        "timing_cuda_copy_site_h2d_top_2_copy_prefix_from_u64_words_layout_rs_92_wait_ns=21\n",
        "timing_cuda_copy_site_h2d_top_2_copy_prefix_from_u64_words_layout_rs_92_max_wait_ns=8\n",
        "timing_cuda_copy_site_h2d_top_2_copy_prefix_from_u64_words_layout_rs_92_avg_wait_per_call_ns=7\n",
    ] {
        assert!(stdout.contains(expected), "missing {expected} in {stdout}");
    }
}

#[test]
fn records_constant_material_validation_join_wait() {
    let mut timings = TimingRecorder::new(true);
    record_constant_material_validation_timing(
        &mut timings,
        std::time::Duration::from_millis(12),
        std::time::Duration::from_millis(3),
        &[],
    );

    let mut stdout = Vec::new();
    write_timing_summary(&mut stdout, &timings);
    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");

    for expected in [
        "timing_constant_material_validation_elapsed_ms=12\n",
        "timing_constant_material_validation_join_wait_ms=3\n",
        "timing_constant_material_validation_units=0\n",
        "timing_constant_material_validation_bytes=0\n",
    ] {
        assert!(stdout.contains(expected), "missing {expected} in {stdout}");
    }
}

#[test]
fn proof_artifact_timing_reports_parent_hash_shape_counts() {
    let timing = lzvm_prover::WitnessProofArtifactTiming {
        witness_opening_path_parent_hash: std::time::Duration::from_millis(9),
        witness_opening_path_parent_hash_recomputed: std::time::Duration::from_millis(2),
        witness_opening_path_parent_hash_retained_leaf_digest: std::time::Duration::from_millis(3),
        witness_opening_path_parent_hash_retained_parent_checkpoint_prefix:
            std::time::Duration::from_millis(4),
        witness_opening_path_parent_hash_retained_parent_checkpoint_suffix:
            std::time::Duration::from_millis(5),
        witness_opening_path_parent_hash_row_count: 120,
        witness_opening_path_parent_hash_recomputed_row_count: 310,
        witness_opening_path_parent_hash_retained_leaf_digest_row_count: 352,
        witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_row_count: 396,
        witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_row_count: 442,
        witness_opening_path_parent_hash_launch_count: 12,
        witness_opening_path_parent_hash_recomputed_launch_count: 31,
        witness_opening_path_parent_hash_retained_leaf_digest_launch_count: 32,
        witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launch_count: 33,
        witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launch_count: 34,
        witness_opening_path_parent_hash_byte_count: 130,
        witness_opening_path_parent_hash_recomputed_byte_count: 620,
        witness_opening_path_parent_hash_retained_leaf_digest_byte_count: 704,
        witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_byte_count: 759,
        witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_byte_count: 816,
        witness_opening_query_count: 3,
        witness_opening_stage_count: 4,
        ..lzvm_prover::WitnessProofArtifactTiming::default()
    };

    let mut timings = TimingRecorder::new(true);
    record_proof_artifact_timing(&mut timings, &timing);

    let mut stdout = Vec::new();
    write_timing_summary(&mut stdout, &timings);
    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");

    assert!(stdout.contains("timing_finish_witness_opening_path_parent_hash_ms=9\n"));
    assert!(stdout.contains("timing_finish_witness_opening_path_parent_hash_recomputed_ms=2\n"));
    assert!(stdout
        .contains("timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_ms=3\n"));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=4\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=5\n"
    ));
    assert!(stdout.contains("timing_finish_witness_opening_path_parent_hash_recomputed_rows=310\n"));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_rows=352\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows=396\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows=442\n"
    ));
    assert!(
        stdout.contains("timing_finish_witness_opening_path_parent_hash_recomputed_bytes=620\n")
    );
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_bytes=704\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes=759\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes=816\n"
    ));
    assert!(
        stdout.contains("timing_finish_witness_opening_path_parent_hash_recomputed_launches=31\n")
    );
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_launches=32\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=33\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=34\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_recomputed_rows_per_launch=10\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_recomputed_bytes_per_launch=20\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_rows_per_launch=11\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_leaf_digest_bytes_per_launch=22\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows_per_launch=12\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes_per_launch=23\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows_per_launch=13\n"
    ));
    assert!(stdout.contains(
        "timing_finish_witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes_per_launch=24\n"
    ));
    assert!(stdout.contains("timing_finish_witness_opening_path_parent_hash_rows_per_query=40\n"));
    assert!(stdout.contains("timing_finish_witness_opening_path_parent_hash_rows_per_stage=30\n"));
    assert!(
        stdout.contains("timing_finish_witness_opening_path_parent_hash_launches_per_stage=3\n")
    );
}

#[test]
fn proof_artifact_timing_reports_external_source_rebuild_shape() {
    let timing = lzvm_prover::WitnessProofArtifactTiming {
        witness_external_source_descriptor_upload: std::time::Duration::from_millis(7),
        witness_external_source_descriptor_upload_byte_count: 88,
        witness_external_source_descriptor_upload_word_count: 11,
        witness_external_source_descriptor_upload_row_count: 2,
        witness_external_source_trace_expand: std::time::Duration::from_millis(11),
        ..lzvm_prover::WitnessProofArtifactTiming::default()
    };

    let mut timings = TimingRecorder::new(true);
    record_proof_artifact_timing(&mut timings, &timing);

    let mut stdout = Vec::new();
    write_timing_summary(&mut stdout, &timings);
    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");

    for expected in [
        "timing_finish_witness_external_source_descriptor_upload_ms=7\n",
        "timing_finish_witness_external_source_descriptor_upload_bytes=88\n",
        "timing_finish_witness_external_source_descriptor_upload_words=11\n",
        "timing_finish_witness_external_source_descriptor_upload_rows=2\n",
        "timing_finish_witness_external_source_trace_expand_ms=11\n",
    ] {
        assert!(stdout.contains(expected), "missing {expected} in {stdout}");
    }
}

#[test]
fn proof_artifact_timing_reports_contribution_work() {
    let timing = lzvm_prover::WitnessProofArtifactTiming {
        contribution_segment: std::time::Duration::from_millis(13),
        contribution_verify: std::time::Duration::from_millis(17),
        contribution_challenge: std::time::Duration::from_millis(19),
        ..lzvm_prover::WitnessProofArtifactTiming::default()
    };

    let mut timings = TimingRecorder::new(true);
    record_proof_artifact_timing(&mut timings, &timing);

    let mut stdout = Vec::new();
    write_timing_summary(&mut stdout, &timings);
    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");

    for expected in [
        "timing_finish_contribution_segment_ms=13\n",
        "timing_finish_contribution_verify_ms=17\n",
        "timing_finish_contribution_challenge_ms=19\n",
    ] {
        assert!(stdout.contains(expected), "missing {expected} in {stdout}");
    }
}

#[test]
fn proof_artifact_timing_reports_per_stage_opening_work_shape() {
    let timing =
        lzvm_prover::WitnessProofArtifactTiming {
            witness_opening_row_values_source_extend: std::time::Duration::from_millis(41),
            witness_opening_row_values_source_download: std::time::Duration::from_millis(42),
            witness_opening_row_values_device_download: std::time::Duration::from_millis(43),
            witness_opening_row_values_device_row_count: 34,
            witness_opening_row_values_device_download_batch_count: 39,
            witness_opening_row_values_source_row_count: 35,
            witness_opening_row_values_word_count: 36,
            witness_opening_row_values_byte_count: 37,
            witness_stage_opening_row_value_source_extend: vec![
                lzvm_prover::WitnessProofStageOpeningTiming {
                    stage_index: 7,
                    duration: std::time::Duration::from_millis(44),
                },
            ],
            witness_stage_opening_row_value_source_download: vec![
                lzvm_prover::WitnessProofStageOpeningTiming {
                    stage_index: 7,
                    duration: std::time::Duration::from_millis(45),
                },
            ],
            witness_stage_opening_row_value_device_download: vec![
                lzvm_prover::WitnessProofStageOpeningTiming {
                    stage_index: 7,
                    duration: std::time::Duration::from_millis(46),
                },
            ],
            witness_stage_opening_work: vec![lzvm_prover::WitnessProofStageOpeningWork {
                stage_index: 7,
                retained_source_count: 2,
                external_source_count: 3,
                embedded_source_count: 4,
                missing_source_count: 5,
                retained_leaf_digest_opening_count: 6,
                retained_leaf_digest_opening_row_count: 7,
                retained_parent_checkpoint_opening_count: 8,
                retained_parent_checkpoint_opening_row_count: 9,
                leaf_hash_row_count: 10,
                leaf_hash_byte_count: 11,
                leaf_hash_arity2_row_count: 26,
                leaf_hash_arity2_byte_count: 27,
                leaf_hash_arity4_row_count: 28,
                leaf_hash_arity4_byte_count: 29,
                leaf_coset_extend_call_count: 12,
                leaf_coset_extend_output_byte_count: 13,
                leaf_coset_extend_column_count: 14,
                leaf_coset_extend_max_column_count: 15,
                leaf_coset_extend_ntt_launch_count: 16,
                leaf_coset_extend_bit_reverse_launch_count: 17,
                leaf_coset_extend_ntt_stage_launch_count: 18,
                leaf_coset_extend_ntt_block_twiddle_launch_count: 19,
                leaf_coset_extend_normalize_launch_count: 20,
                leaf_coset_extend_pack_launch_count: 21,
                leaf_coset_extend_unpack_launch_count: 22,
                path_parent_hash: std::time::Duration::from_millis(47),
                path_parent_hash_recomputed: std::time::Duration::from_millis(48),
                path_parent_hash_retained_leaf_digest: std::time::Duration::from_millis(49),
                path_parent_hash_retained_parent_checkpoint_prefix:
                    std::time::Duration::from_millis(50),
                path_parent_hash_retained_parent_checkpoint_suffix:
                    std::time::Duration::from_millis(51),
                path_parent_hash_row_count: 23,
                path_parent_hash_recomputed_row_count: 600,
                path_parent_hash_retained_leaf_digest_row_count: 671,
                path_parent_hash_retained_parent_checkpoint_prefix_row_count: 744,
                path_parent_hash_retained_parent_checkpoint_suffix_row_count: 819,
                path_parent_hash_byte_count: 24,
                path_parent_hash_recomputed_byte_count: 1200,
                path_parent_hash_retained_leaf_digest_byte_count: 1342,
                path_parent_hash_retained_parent_checkpoint_prefix_byte_count: 1488,
                path_parent_hash_retained_parent_checkpoint_suffix_byte_count: 1638,
                path_parent_hash_launch_count: 25,
                path_parent_hash_recomputed_launch_count: 60,
                path_parent_hash_retained_leaf_digest_launch_count: 61,
                path_parent_hash_retained_parent_checkpoint_prefix_launch_count: 62,
                path_parent_hash_retained_parent_checkpoint_suffix_launch_count: 63,
                row_values_device_row_count: 30,
                row_values_device_download_batch_count: 40,
                row_values_source_row_count: 31,
                row_values_word_count: 32,
                row_values_byte_count: 33,
            }],
            ..lzvm_prover::WitnessProofArtifactTiming::default()
        };

    let mut timings = TimingRecorder::new(true);
    record_proof_artifact_timing(&mut timings, &timing);

    let mut stdout = Vec::new();
    write_timing_summary(&mut stdout, &timings);
    let stdout = String::from_utf8(stdout).expect("timing output should be utf-8");

    for expected in [
        "timing_finish_witness_opening_row_value_source_extend_ms=41\n",
        "timing_finish_witness_opening_row_value_source_download_ms=42\n",
        "timing_finish_witness_opening_row_value_device_download_ms=43\n",
        "timing_finish_witness_opening_row_values_device_rows=34\n",
        "timing_finish_witness_opening_row_values_device_download_batches=39\n",
        "timing_finish_witness_opening_row_values_source_rows=35\n",
        "timing_finish_witness_opening_row_values_words=36\n",
        "timing_finish_witness_opening_row_values_bytes=37\n",
        "timing_finish_witness_stage_7_opening_retained_source_count=2\n",
        "timing_finish_witness_stage_7_opening_external_source_count=3\n",
        "timing_finish_witness_stage_7_opening_embedded_source_count=4\n",
        "timing_finish_witness_stage_7_opening_missing_source_count=5\n",
        "timing_finish_witness_stage_7_opening_retained_leaf_digest_openings=6\n",
        "timing_finish_witness_stage_7_opening_retained_leaf_digest_rows=7\n",
        "timing_finish_witness_stage_7_opening_retained_parent_checkpoint_openings=8\n",
        "timing_finish_witness_stage_7_opening_retained_parent_checkpoint_rows=9\n",
        "timing_finish_witness_stage_7_opening_leaf_hash_rows=10\n",
        "timing_finish_witness_stage_7_opening_leaf_hash_bytes=11\n",
        "timing_finish_witness_stage_7_opening_leaf_hash_arity2_rows=26\n",
        "timing_finish_witness_stage_7_opening_leaf_hash_arity2_bytes=27\n",
        "timing_finish_witness_stage_7_opening_leaf_hash_arity4_rows=28\n",
        "timing_finish_witness_stage_7_opening_leaf_hash_arity4_bytes=29\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_calls=12\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_output_bytes=13\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_columns=14\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_max_columns=15\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_ntt_launches=16\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_bit_reverse_launches=17\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_ntt_stage_launches=18\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_ntt_block_twiddle_launches=19\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_normalize_launches=20\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_pack_launches=21\n",
        "timing_finish_witness_stage_7_opening_leaf_coset_extend_unpack_launches=22\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_ms=47\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_recomputed_ms=48\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_leaf_digest_ms=49\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_prefix_ms=50\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_suffix_ms=51\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_rows=23\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_bytes=24\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_launches=25\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_recomputed_rows=600\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_leaf_digest_rows=671\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows=744\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows=819\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_recomputed_bytes=1200\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_leaf_digest_bytes=1342\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes=1488\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes=1638\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_recomputed_launches=60\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_leaf_digest_launches=61\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_prefix_launches=62\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_suffix_launches=63\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_recomputed_rows_per_launch=10\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_recomputed_bytes_per_launch=20\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_leaf_digest_rows_per_launch=11\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_leaf_digest_bytes_per_launch=22\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_prefix_rows_per_launch=12\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_prefix_bytes_per_launch=24\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_suffix_rows_per_launch=13\n",
        "timing_finish_witness_stage_7_opening_path_parent_hash_retained_parent_checkpoint_suffix_bytes_per_launch=26\n",
        "timing_finish_witness_stage_7_opening_row_value_source_extend_ms=44\n",
        "timing_finish_witness_stage_7_opening_row_value_source_download_ms=45\n",
        "timing_finish_witness_stage_7_opening_row_value_device_download_ms=46\n",
        "timing_finish_witness_stage_7_opening_row_values_device_rows=30\n",
        "timing_finish_witness_stage_7_opening_row_values_device_download_batches=40\n",
        "timing_finish_witness_stage_7_opening_row_values_source_rows=31\n",
        "timing_finish_witness_stage_7_opening_row_values_words=32\n",
        "timing_finish_witness_stage_7_opening_row_values_bytes=33\n",
    ] {
        assert!(stdout.contains(expected), "missing {expected} in {stdout}");
    }
}

#[test]
fn guest_pc_trace_uses_parallel_witness_threads_by_default() {
    let result = parse_witness_args(&[
        "--guest-pc-trace",
        "64",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.run_args.request.gpu.witness_thread_pools, 32);
}

#[test]
fn guest_pc_trace_preserves_explicit_witness_thread_pools() {
    let result = parse_witness_args(&[
        "--guest-pc-trace",
        "64",
        "--witness-thread-pools",
        "6",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.run_args.request.gpu.witness_thread_pools, 6);
}

#[test]
fn parses_unit_index_option_for_single_unit_witness_args() {
    let result = parse_witness_args(&[
        "--unit-index",
        "24",
        "--guest-pc-trace",
        "64",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.unit_index, Some(24));
}

#[test]
fn rejects_unit_index_with_all_units_during_parse() {
    let result = parse_witness_args(&[
        "--unit-index",
        "24",
        "--all-units",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--unit-index requires a single-unit witness run"
    ));
}

#[test]
fn rejects_duplicate_unit_index_during_parse() {
    let result = parse_witness_args(&[
        "--unit-index",
        "24",
        "--unit-index",
        "25",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message)) if message == "duplicate --unit-index option"
    ));
}

#[test]
fn rejects_guest_pc_trace_with_trace_bytes_during_parse() {
    let result = parse_witness_args(&[
        "--guest-pc-trace",
        "64",
        "--trace-bytes",
        "trace.bin",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "cannot combine --guest-pc-trace with --trace-bytes or --trace-bundle"
    ));
}

#[test]
fn rejects_guest_pc_trace_with_all_units_during_parse() {
    let result = parse_witness_args(&[
        "--guest-pc-trace",
        "64",
        "--all-units",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--guest-pc-trace requires a single-unit witness run"
    ));
}

#[test]
fn rejects_evaluation_values_segment_without_all_units_during_parse() {
    let result = parse_witness_args(&[
        "--evaluation-values-segment",
        "evaluations.bin",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--evaluation-values-segment requires all-units mode"
    ));
}

#[test]
fn parses_eth_block_input_option_for_witness_args() {
    let result = parse_witness_args(&[
        "--eth-block-input",
        "block.input",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.eth_block_input, Some("block.input".into()));
}

#[test]
fn parses_eth_public_input_option_for_witness_args() {
    let result = parse_witness_args(&[
        "--eth-public-input",
        "public.bin",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.eth_public_input, Some("public.bin".into()));
}

#[test]
fn rejects_combined_eth_block_and_public_input_options() {
    let result = parse_witness_args(&[
        "--eth-block-input",
        "block.input",
        "--eth-public-input",
        "public.bin",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "cannot combine --eth-block-input and --eth-public-input"
    ));
}

#[test]
fn rejects_missing_eth_public_input_value_during_parse() {
    let result = parse_witness_args(&[
        "--eth-public-input",
        "--trace-bytes",
        "trace.bin",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message)) if message == "missing --eth-public-input value"
    ));
}

#[test]
fn writes_eth_public_input_option_as_block_input_artifact() {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-prove-witness-eth-public-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let input_path = dir.join("public.bin");
    let output_dir = dir.join("proof-out");
    fs::write(&input_path, sample_public_block_bytes_with_matching_roots())
        .expect("public input should be written");
    let parsed = parse_witness_args(&[
        "--eth-public-input",
        input_path.to_str().expect("input path should be utf-8"),
        "setup-dir",
        output_dir.to_str().expect("output path should be utf-8"),
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    let prepared =
        prepare_eth_block_input(&parsed).expect("public input should prepare block input");
    let summary = prepared
        .summary
        .expect("block input summary should be present");
    let output_path = output_dir.join("eth-block.input");
    let encoded = fs::read(&output_path).expect("block input should be written");
    let parsed_input = parse_eth_block_input(&encoded).expect("block input should parse");

    assert!(prepared.generated_from_public_input);
    assert_eq!(summary.path, output_path);
    assert_eq!(summary.byte_len, encoded.len() as u64);
    assert_eq!(summary.input, parsed_input);
    assert_eq!(summary.block_number, 42);
    assert_eq!(summary.transaction_preimage_count, 1);
    assert_eq!(summary.withdrawal_count, Some(1));
    fs::remove_dir_all(&dir).expect("temp dir should be removed");
}

#[test]
fn rejects_eth_public_input_with_trailing_bytes() {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-prove-witness-eth-public-trailing-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let input_path = dir.join("public.bin");
    let output_dir = dir.join("proof-out");
    let mut public_input = sample_public_block_bytes_with_matching_roots();
    public_input.extend_from_slice(b"tail");
    fs::write(&input_path, public_input).expect("public input should be written");
    let parsed = parse_witness_args(&[
        "--eth-public-input",
        input_path.to_str().expect("input path should be utf-8"),
        "setup-dir",
        output_dir.to_str().expect("output path should be utf-8"),
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    let result = prepare_eth_block_input(&parsed);
    let output_exists = output_dir.join("eth-block.input").exists();
    fs::remove_dir_all(&dir).expect("temp dir should be removed");

    assert!(matches!(
        result,
        Err(message)
            if message
                == format!(
                    "ETH public input failed: {}: unexpected trailing bytes in ETH public input: 4",
                    input_path.display()
                )
    ));
    assert!(!output_exists);
}

#[test]
fn writes_eth_public_input_with_allowed_trailing_bytes_as_block_input_artifact() {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-prove-witness-eth-public-allow-trailing-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let input_path = dir.join("public.bin");
    let output_dir = dir.join("proof-out");
    let mut public_input = sample_public_block_bytes_with_matching_roots();
    public_input.extend_from_slice(b"tail");
    fs::write(&input_path, public_input).expect("public input should be written");
    let parsed = parse_witness_args(&[
        "--eth-public-input",
        input_path.to_str().expect("input path should be utf-8"),
        "--eth-public-input-allow-trailing",
        "setup-dir",
        output_dir.to_str().expect("output path should be utf-8"),
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    let prepared =
        prepare_eth_block_input(&parsed).expect("public input should prepare block input");
    let summary = prepared
        .summary
        .expect("block input summary should be present");
    let output_path = output_dir.join("eth-block.input");
    let encoded = fs::read(&output_path).expect("block input should be written");
    let parsed_input = parse_eth_block_input(&encoded).expect("block input should parse");

    assert!(prepared.generated_from_public_input);
    assert_eq!(summary.path, output_path);
    assert_eq!(summary.byte_len, encoded.len() as u64);
    assert_eq!(summary.input, parsed_input);
    assert_eq!(summary.block_number, 42);
    assert_eq!(summary.transaction_preimage_count, 1);
    assert_eq!(summary.withdrawal_count, Some(1));
    fs::remove_dir_all(&dir).expect("temp dir should be removed");
}

#[test]
fn rejects_eth_public_input_allow_trailing_without_eth_public_input() {
    let result = parse_witness_args(&[
        "--eth-public-input-allow-trailing",
        "--trace-bytes",
        "trace.bin",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "cannot use --eth-public-input-allow-trailing without --eth-public-input"
    ));
}

fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&withdrawal_bytes());

    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let transaction_root = parsed.transactions_root();
    let ommers_hash = parsed.ommers_hash();
    let withdrawal_root = parsed
        .withdrawals_root()
        .expect("withdrawals root should be present");
    input[48..80].copy_from_slice(&ommers_hash);
    input[156..188].copy_from_slice(&transaction_root);
    input[237..269].copy_from_slice(&withdrawal_root);
    input
}

fn sample_public_header_bytes() -> Vec<u8> {
    let mut input = Vec::new();
    push_public_bytes(&mut input, &[1; 32]);
    push_public_bytes(&mut input, &[2; 32]);
    push_public_bytes(&mut input, &[3; 20]);
    push_public_bytes(&mut input, &[4; 32]);
    push_public_bytes(&mut input, &[5; 32]);
    push_public_bytes(&mut input, &[6; 32]);
    push_public_option_bytes(&mut input, Some(&[7; 32]));
    push_public_bytes(&mut input, &[8; 256]);
    push_public_bytes(&mut input, &u256_bytes(9));
    input.extend_from_slice(&42_u64.to_le_bytes());
    input.extend_from_slice(&100_u64.to_le_bytes());
    input.extend_from_slice(&90_u64.to_le_bytes());
    input.extend_from_slice(&77_u64.to_le_bytes());
    push_public_bytes(&mut input, &[10; 32]);
    push_public_bytes(&mut input, &[11; 8]);
    push_public_option_u64(&mut input, Some(123));
    push_public_option_u64(&mut input, Some(456));
    push_public_option_u64(&mut input, Some(789));
    push_public_option_bytes(&mut input, Some(&[12; 32]));
    push_public_option_bytes(&mut input, Some(&[13; 32]));
    push_public_bytes(&mut input, b"abc");
    input
}

fn eip1559_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_u256(&mut bytes, 0x11);
    push_public_u256(&mut bytes, 0x22);
    push_public_uint_u64(&mut bytes, 1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&21_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_public_option_bytes(&mut bytes, Some(&[9; 20]));
    push_public_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    push_public_bytes(&mut bytes, b"call-data");
    bytes
}

fn withdrawal_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_uint_u64(&mut bytes, 7);
    push_public_uint_u64(&mut bytes, 8);
    push_public_bytes(&mut bytes, &[6; 20]);
    push_public_uint_u64(&mut bytes, 9);
    bytes
}

fn push_public_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_public_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_public_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

fn push_public_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

fn push_public_u256(out: &mut Vec<u8>, value: u8) {
    let mut bytes = [0; 32];
    bytes[31] = value;
    push_public_bytes(out, &bytes);
}

fn push_public_uint_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&8_u64.to_le_bytes());
    out.extend_from_slice(&value.to_be_bytes());
}

fn u256_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}
