use std::process::Command;

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
        "needs_cross_segment_root_pipeline",
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
        "profile,total_ms,root_count,materialization_groups,materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline",
        "single-root-groups,9050,23,23,1,1.000,yes",
        "batched-roots,9050,23,1,23,23.000,no",
    ] {
        assert!(
            stdout.contains(required),
            "prove timing root summary should print {required}"
        );
    }
}
