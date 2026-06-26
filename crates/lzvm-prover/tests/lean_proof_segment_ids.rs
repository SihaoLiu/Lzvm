use std::path::Path;

#[test]
fn lean_proof_segment_ids_track_runtime_allowlist() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("top-level Lean source should read");
    let lean_path = crate_root.join("../../lean/Lzvm/ProofSegmentIds.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean proof segment ID source should read");
    let rust_path = crate_root.join("src/proof_segment_ids.rs");
    let rust_source =
        std::fs::read_to_string(&rust_path).expect("Rust proof segment ID source should read");

    assert!(
        lean_root_source.contains("import Lzvm.ProofSegmentIds"),
        "top-level Lean module should import concrete proof segment ID allowlist"
    );
    assert!(
        lean_source.contains("def IsAllowedProofSegmentId")
            && lean_source.contains("def isFixedProofSegmentIdBool")
            && lean_source.contains("theorem witness_commitment_base_id_allowed")
            && lean_source.contains("theorem unknown_fixed_proof_segment_id_not_allowed"),
        "Lean should expose concrete allowed and rejected proof segment ID facts"
    );
    assert!(
        rust_source.contains(
            "(WITNESS_COMMITMENT_SEGMENT_BASE_ID..PCS_MATERIAL_MANIFEST_SEGMENT_ID).contains(&id)"
        ) && rust_source.contains("matches!("),
        "Rust proof segment ID helper should retain the witness range and fixed-ID allowlist"
    );

    for expected in EXPECTED_SEGMENT_IDS {
        assert!(
            lean_source.contains(&format!(
                "def {} : Nat := {}",
                expected.lean_name, expected.value
            )),
            "Lean proof segment ID {} should match runtime value {}",
            expected.lean_name,
            expected.value
        );
        assert!(
            rust_source.contains(expected.rust_name),
            "Rust proof segment ID helper should include {}",
            expected.rust_name
        );
    }
}

struct ExpectedSegmentId {
    lean_name: &'static str,
    rust_name: &'static str,
    value: &'static str,
}

const EXPECTED_SEGMENT_IDS: &[ExpectedSegmentId] = &[
    ExpectedSegmentId {
        lean_name: "witnessCommitmentSegmentBaseId",
        rust_name: "WITNESS_COMMITMENT_SEGMENT_BASE_ID",
        value: "100",
    },
    ExpectedSegmentId {
        lean_name: "pcsMaterialManifestSegmentId",
        rust_name: "PCS_MATERIAL_MANIFEST_SEGMENT_ID",
        value: "10000",
    },
    ExpectedSegmentId {
        lean_name: "pcsQueryPlanSegmentId",
        rust_name: "PCS_QUERY_PLAN_SEGMENT_ID",
        value: "10001",
    },
    ExpectedSegmentId {
        lean_name: "witnessOpeningSegmentId",
        rust_name: "WITNESS_OPENING_SEGMENT_ID",
        value: "10002",
    },
    ExpectedSegmentId {
        lean_name: "constantOpeningSegmentId",
        rust_name: "CONSTANT_OPENING_SEGMENT_ID",
        value: "10003",
    },
    ExpectedSegmentId {
        lean_name: "pcsFriOpeningSegmentId",
        rust_name: "PCS_FRI_OPENING_SEGMENT_ID",
        value: "10004",
    },
    ExpectedSegmentId {
        lean_name: "pcsQueryNonceSegmentId",
        rust_name: "PCS_QUERY_NONCE_SEGMENT_ID",
        value: "10005",
    },
    ExpectedSegmentId {
        lean_name: "pcsEvaluationSegmentId",
        rust_name: "PCS_EVALUATION_SEGMENT_ID",
        value: "10006",
    },
    ExpectedSegmentId {
        lean_name: "pcsProofValuesSegmentId",
        rust_name: "PCS_PROOF_VALUES_SEGMENT_ID",
        value: "10007",
    },
    ExpectedSegmentId {
        lean_name: "groupValuesSegmentId",
        rust_name: "GROUP_VALUES_SEGMENT_ID",
        value: "10008",
    },
    ExpectedSegmentId {
        lean_name: "unitValuesSegmentId",
        rust_name: "UNIT_VALUES_SEGMENT_ID",
        value: "10009",
    },
    ExpectedSegmentId {
        lean_name: "programImageCacheSegmentId",
        rust_name: "PROGRAM_IMAGE_CACHE_SEGMENT_ID",
        value: "10010",
    },
    ExpectedSegmentId {
        lean_name: "contributionSegmentId",
        rust_name: "CONTRIBUTION_SEGMENT_ID",
        value: "10011",
    },
    ExpectedSegmentId {
        lean_name: "challengeValuesSegmentId",
        rust_name: "CHALLENGE_VALUES_SEGMENT_ID",
        value: "10012",
    },
    ExpectedSegmentId {
        lean_name: "ethBlockInputSegmentId",
        rust_name: "ETH_BLOCK_INPUT_SEGMENT_ID",
        value: "10013",
    },
    ExpectedSegmentId {
        lean_name: "traceConstraintSegmentId",
        rust_name: "TRACE_CONSTRAINT_SEGMENT_ID",
        value: "10014",
    },
    ExpectedSegmentId {
        lean_name: "framedGuestInputSegmentId",
        rust_name: "FRAMED_GUEST_INPUT_SEGMENT_ID",
        value: "10015",
    },
];
