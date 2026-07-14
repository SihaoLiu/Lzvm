use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

const ACCEPTS_SOURCE_PATH: &str = "../../lean/Lzvm/PipelineBinding/Accepts.lean";
const AUDITED_SOURCE_PATH: &str = "../../lean/Lzvm/PipelineBinding/Audited.lean";
const CONTRACTS_CORE_AUDITED_SOURCE_PATH: &str =
    "../../lean/Lzvm/PipelineBinding/Contracts/Core/Audited.lean";
const CONTRACTS_CORE_BASE_SOURCE_PATH: &str =
    "../../lean/Lzvm/PipelineBinding/Contracts/Core/Base.lean";
const CONTRACTS_EXTERNAL_SOURCE_PATH: &str =
    "../../lean/Lzvm/PipelineBinding/Contracts/ExternalSource.lean";
const CORE_DERIVED_SOURCE_PATH: &str = "../../lean/Lzvm/PipelineBinding/Core/Derived.lean";
const CORE_OBLIGATIONS_SOURCE_PATH: &str = "../../lean/Lzvm/PipelineBinding/Obligations/Core.lean";
const SOUNDNESS_OBLIGATIONS_SOURCE_PATH: &str =
    "../../lean/Lzvm/PipelineBinding/Obligations/Soundness.lean";
const EXTERNAL_SOURCE_CONTRACTS_PATH: &str =
    "../../lean/Lzvm/PipelineBinding/ExternalSourceContracts.lean";
const SEGMENT_IDS_BASE_SOURCE_PATH: &str = "../../lean/Lzvm/PipelineBinding/SegmentIds/Base.lean";
const SEGMENT_IDS_EXTERNAL_SOURCE_PATH: &str =
    "../../lean/Lzvm/PipelineBinding/SegmentIds/ExternalSource.lean";

fn assert_routes_required_evidence_directly(lean_source: &str, theorem: &str) {
    lean_binding::assert_theorem_declarations(lean_source, &[theorem]);
    lean_binding::assert_theorem_body_contains(
        lean_source,
        theorem,
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        lean_source,
        theorem,
        &["assumption_bundle_carries_required_evidence"],
    );
}

fn assert_routes_accepted_evidence_by_split_helpers(lean_source: &str, theorem: &str) {
    lean_binding::assert_theorem_routes_accepted_evidence_by_split_helpers(lean_source, theorem);
    for identifier in [
        "accepted_proof_audited_core_execution_and_sound_witness",
        "assumption_bundle_carries_required_evidence",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(lean_source, theorem, identifier);
    }
}

fn assert_routes_proof_system_by_split_helper(lean_source: &str, theorem: &str) {
    lean_binding::assert_theorem_declarations(lean_source, &[theorem]);
    lean_binding::assert_theorem_body_contains_identifier(
        lean_source,
        theorem,
        "abstract_verifier_sound_with_semantic_evidence",
    );
    lean_binding::assert_theorem_body_omits_identifier(
        lean_source,
        theorem,
        "abstract_verifier_sound",
    );
}

#[test]
fn lean_pipeline_core_derived_routes_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, CORE_DERIVED_SOURCE_PATH);

    assert_routes_required_evidence_directly(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_audited_soundness_obligations",
    );
}

#[test]
fn lean_pipeline_checked_acceptance_derives_soundness_without_semantic_assumptions() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let derived_source = lean_binding::read_lean_source(crate_root, CORE_DERIVED_SOURCE_PATH);
    let core_source = lean_binding::read_lean_source(crate_root, CORE_OBLIGATIONS_SOURCE_PATH);
    let soundness_source =
        lean_binding::read_lean_source(crate_root, SOUNDNESS_OBLIGATIONS_SOURCE_PATH);

    lean_binding::assert_theorem_declarations(
        &derived_source,
        &["runtime_pipeline_binding_checked_acceptance_public_input_bound_without_assumptions"],
    );
    lean_binding::assert_theorem_prefix_omits(
        &derived_source,
        "runtime_pipeline_binding_checked_acceptance_public_input_bound_without_assumptions",
        &[
            "AssumptionBundle",
            "SemanticAssumptions",
            "CryptographicAssumptions",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &derived_source,
        "runtime_pipeline_binding_checked_acceptance_public_input_bound_without_assumptions",
        &[
            "runtime_pipeline_binding_checked_acceptance_proof_artifact_evidence",
            "runtime_proof_artifact_binding_evidence_implies_runtime_evidence",
            "runtime_artifact_evidence_implies_public_input_bound",
        ],
    );

    lean_binding::assert_theorem_declarations(
        &core_source,
        &["runtime_pipeline_binding_checked_acceptance_core_obligations_without_assumptions"],
    );
    lean_binding::assert_theorem_prefix_omits(
        &core_source,
        "runtime_pipeline_binding_checked_acceptance_core_obligations_without_assumptions",
        &[
            "AssumptionBundle",
            "SemanticAssumptions",
            "CryptographicAssumptions",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &core_source,
        "runtime_pipeline_binding_checked_acceptance_core_obligations_without_assumptions",
        &[
            "runtime_pipeline_binding_checked_acceptance_transcript_bound_without_assumptions",
            "runtime_pipeline_binding_checked_acceptance_public_input_bound_without_assumptions",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri_without_assumptions",
        ],
    );

    for theorem in [
        "runtime_pipeline_binding_checked_acceptance_sound_witness_without_assumptions",
        "runtime_pipeline_binding_checked_acceptance_direct_soundness_contract",
    ] {
        lean_binding::assert_theorem_declarations(&soundness_source, &[theorem]);
        lean_binding::assert_theorem_prefix_omits(
            &soundness_source,
            theorem,
            &[
                "AssumptionBundle",
                "SemanticAssumptions",
                "CryptographicAssumptions",
            ],
        );
        lean_binding::assert_theorem_body_omits(
            &soundness_source,
            theorem,
            &[
                "runtime_pipeline_binding_checked_acceptance_sound\n",
                "abstract_verifier_sound",
                "assumptions.semantic",
                "semanticAssumptions.public_input_binding",
            ],
        );
    }
    lean_binding::assert_theorem_body_contains(
        &soundness_source,
        "runtime_pipeline_binding_checked_acceptance_sound_witness_without_assumptions",
        &[
            "runtime_pipeline_binding_checked_acceptance_core_obligations_without_assumptions",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint",
            "runtime_trace_constraint_checked_acceptance_trace_witness_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &soundness_source,
        "runtime_pipeline_binding_checked_acceptance_direct_soundness_contract",
        &[
            "system.accepts publicInput proof",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "RuntimeTraceConstraintSemanticEvidenceComplete",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &soundness_source,
        "runtime_pipeline_binding_checked_acceptance_direct_soundness_contract",
        &[
            "runtime_pipeline_binding_checked_acceptance_verifier_accepts",
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence",
            "runtime_pipeline_binding_checked_acceptance_core_obligations_without_assumptions",
            "runtime_pipeline_binding_checked_acceptance_trace_semantic_evidence_complete",
            "runtime_pipeline_binding_checked_acceptance_sound_witness_without_assumptions",
        ],
    );
}

#[test]
fn lean_pipeline_accepts_routes_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, ACCEPTS_SOURCE_PATH);

    for theorem in [
        "runtime_pipeline_binding_checked_acceptance_audited_soundness_accepts_contract",
        "runtime_pipeline_binding_checked_acceptance_audited_full_soundness_contract",
    ] {
        assert_routes_required_evidence_directly(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_accepts_routes_proof_system_by_split_helper() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, ACCEPTS_SOURCE_PATH);

    for theorem in [
        "runtime_pipeline_binding_checked_acceptance_proof_system_sound",
        "runtime_pipeline_binding_checked_acceptance_audited_full_soundness_contract",
    ] {
        assert_routes_proof_system_by_split_helper(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_audited_routes_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, AUDITED_SOURCE_PATH);

    for theorem in [
        "runtime_pipeline_binding_required_external_source_audited_soundness_proof_system_contract",
        "runtime_pipeline_binding_required_external_source_audited_soundness_pcs_fri_core_witness_contract",
        "runtime_pipeline_binding_required_external_source_audited_sound_proof_system_core_contract",
    ] {
        assert_routes_required_evidence_directly(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_contracts_core_base_routes_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, CONTRACTS_CORE_BASE_SOURCE_PATH);

    assert_routes_required_evidence_directly(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_audited_assumption_full_contract",
    );
}

#[test]
fn lean_pipeline_contracts_core_base_routes_proof_system_by_split_helper() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, CONTRACTS_CORE_BASE_SOURCE_PATH);

    assert_routes_proof_system_by_split_helper(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract",
    );
}

#[test]
fn lean_pipeline_contracts_core_audited_routes_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source =
        lean_binding::read_lean_source(crate_root, CONTRACTS_CORE_AUDITED_SOURCE_PATH);

    for theorem in [
        "runtime_pipeline_binding_evidence_audited_soundness_core_contract",
        "runtime_pipeline_binding_checked_acceptance_audited_soundness_pcs_fri_core_witness_contract",
        "runtime_pipeline_checked_acceptance_concrete_opening_audited_soundness_core_contract",
        "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_audited_soundness_core_contract",
        "runtime_pipeline_binding_checked_acceptance_contracts_audited_soundness_core_contract",
    ] {
        assert_routes_required_evidence_directly(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_contracts_core_audited_routes_proof_system_by_split_helper() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source =
        lean_binding::read_lean_source(crate_root, CONTRACTS_CORE_AUDITED_SOURCE_PATH);

    for theorem in [
        "runtime_pipeline_binding_checked_acceptance_audited_query_opening_core_sound_contract",
        "runtime_pipeline_binding_checked_acceptance_audited_concrete_opening_contract",
        "runtime_pipeline_binding_checked_acceptance_hash_concrete_opening_core_contract",
    ] {
        assert_routes_proof_system_by_split_helper(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_contracts_external_route_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, CONTRACTS_EXTERNAL_SOURCE_PATH);

    for theorem in [
        "runtime_pipeline_required_external_source_concrete_audited_soundness_core_contract",
        "runtime_pipeline_required_external_source_hash_concrete_audited_soundness_core_contract",
    ] {
        assert_routes_required_evidence_directly(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_external_source_contracts_route_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, EXTERNAL_SOURCE_CONTRACTS_PATH);

    for theorem in [
        "runtime_pipeline_binding_required_external_source_contracts_audited_soundness_core_contract",
        "runtime_pipeline_binding_required_external_source_artifact_audited_soundness_core_contract",
    ] {
        assert_routes_required_evidence_directly(&lean_source, theorem);
    }
}

#[test]
fn lean_pipeline_segment_ids_route_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_source = lean_binding::read_lean_source(crate_root, SEGMENT_IDS_BASE_SOURCE_PATH);
    let external_source =
        lean_binding::read_lean_source(crate_root, SEGMENT_IDS_EXTERNAL_SOURCE_PATH);

    assert_routes_required_evidence_directly(
        &base_source,
        "runtime_pipeline_binding_checked_acceptance_audited_soundness_segment_ids_contract",
    );
    assert_routes_required_evidence_directly(
        &external_source,
        "runtime_pipeline_binding_required_external_source_audited_soundness_segment_ids_contract",
    );
}

#[test]
fn lean_pipeline_audited_routes_accepted_evidence_by_split_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, AUDITED_SOURCE_PATH);

    assert_routes_accepted_evidence_by_split_helpers(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_direct_finalized_core_sound_witness_contract",
    );
}
