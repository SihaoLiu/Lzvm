use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

const CONTRACTS_SOURCE_PATH: &str = "../../lean/Lzvm/RuntimeSoundness/Contracts.lean";
const TOP_LEVEL_SOURCE_PATH: &str = "../../lean/Lzvm.lean";
const CONTRACTS_MODULE: &str = "Lzvm.RuntimeSoundness.Contracts";

const DIRECT_ASSUMPTION_BODY_SNIPPETS: &[&str] = &[
    "assumptions.crypto.transcript_binding",
    "assumptions.semantic.public_input_binding",
    "sound_witness_implies_verifier_core_contract",
];

const AUDITED_ARTIFACT_CORE_SNIPPETS: &[&str] = &[
    "RuntimeSoundnessCheckedAcceptance",
    "RuntimeArtifactEvidence",
    "RequiredCryptographicAssumptionStatements assumptions.crypto",
    "RequiredSemanticAssumptionStatements assumptions.semantic",
    "ProofSystemSound system",
    "system.accepts publicInput proof",
    "system.transcriptBound publicInput proof",
    "system.publicInputBound publicInput proof",
    "system.pcsOpeningsValid publicInput proof",
    "system.friQueriesValid publicInput proof",
    "RuntimeVerifierCoreContract system publicInput proof",
    "exists witness trace constraints",
    "system.traceConsistent publicInput proof trace",
    "system.constraintsSatisfied constraints trace",
    "system.witnessMatchesTrace witness trace",
    "SoundWitness system publicInput proof",
];

const REQUIRED_SOURCE_SNIPPETS: &[&str] = &[
    "(requiresExternalSource : Prop)",
    "ExternalSourceOpeningEvidence",
    "validation.sourceValidation",
];

const SEGMENT_ID_SNIPPETS: &[&str] = &[
    "proofContainerCanonical artifact publicInput proof",
    "proofSegmentsPresent artifact publicInput proof",
    "proofMetadataCanonical artifact publicInput proof",
    "proofSegmentPayloadsNonempty artifact publicInput proof",
    "proofSegmentIdsAllowed artifact publicInput proof",
    "proofSegmentIdsUnique artifact publicInput proof",
    "proofUnitValuesTraceIdentityCoverage",
];

fn read_contracts_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    lean_binding::read_lean_source(crate_root, CONTRACTS_SOURCE_PATH)
}

fn read_top_level_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    lean_binding::read_lean_source(crate_root, TOP_LEVEL_SOURCE_PATH)
}

fn assert_prefix_contains_groups(source: &str, theorem: &str, groups: &[&[&str]]) {
    for group in groups {
        lean_binding::assert_theorem_prefix_contains(source, theorem, group);
    }
}

fn assert_body_omits_direct_assumption_access(source: &str, theorem: &str) {
    lean_binding::assert_theorem_body_omits(source, theorem, DIRECT_ASSUMPTION_BODY_SNIPPETS);
}

fn assert_required_source_guard(source: &str, theorem: &str) {
    let prefix = lean_binding::theorem_prefix(source, theorem);
    assert!(
        prefix.matches("requiresExternalSource ->").count() >= 2,
        "Lean theorem {theorem} should require the external-source premise separately"
    );
}

#[test]
fn lean_runtime_soundness_contracts_exports_artifact_audited_segment_contract() {
    let lean_source = read_contracts_source();
    let top_level_source = read_top_level_source();

    assert!(
        lean_binding::contains_import(&top_level_source, CONTRACTS_MODULE),
        "top-level Lean module should import runtime soundness contracts"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract"],
    );
    assert_prefix_contains_groups(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
        &[AUDITED_ARTIFACT_CORE_SNIPPETS, SEGMENT_ID_SNIPPETS],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract",
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract",
        ],
    );
    assert_body_omits_direct_assumption_access(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
    );
}

#[test]
fn lean_runtime_soundness_contracts_exports_concrete_segment_contract() {
    let lean_source = read_contracts_source();

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract"],
    );
    assert_prefix_contains_groups(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract",
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            AUDITED_ARTIFACT_CORE_SNIPPETS,
            SEGMENT_ID_SNIPPETS,
            &["RuntimeProofArtifactConcreteSegmentIdsAllowed proof"],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    assert_body_omits_direct_assumption_access(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract",
    );
}

#[test]
fn lean_runtime_soundness_contracts_exports_finalized_segment_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_checked_acceptance_artifact_audited_finalized_segment_ids_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            AUDITED_ARTIFACT_CORE_SNIPPETS,
            &["RuntimeProofArtifactFinalized"],
            SEGMENT_ID_SNIPPETS,
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
            "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_finalized_concrete_segment_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_checked_acceptance_artifact_audited_finalized_concrete_segment_ids_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            AUDITED_ARTIFACT_CORE_SNIPPETS,
            &["RuntimeProofArtifactFinalized"],
            SEGMENT_ID_SNIPPETS,
            &["RuntimeProofArtifactConcreteSegmentIdsAllowed proof"],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_finalized_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_finalized_concrete_core_components_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_core_components_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            &[
                "RequiredCryptographicAssumptionStatements assumptions.crypto",
                "RequiredSemanticAssumptionStatements assumptions.semantic",
                "RuntimeProofArtifactFinalized",
                "ProofSystemSound system",
                "system.accepts publicInput proof",
                "system.transcriptBound publicInput proof",
                "system.publicInputBound publicInput proof",
                "system.pcsOpeningsValid publicInput proof",
                "system.friQueriesValid publicInput proof",
                "RuntimeVerifierCoreContract system publicInput proof",
                "exists witness trace constraints",
                "SoundWitness system publicInput proof",
            ],
            &[
                "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
                "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
                "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
            ],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_finalized_concrete_segment_ids_contract",
            "finalizedSegmentContract",
            "concreteSegmentIdsAllowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_finalized_concrete_core_requirements_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_checked_acceptance_finalized_concrete_core_requirements_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            &[
                "RuntimeProofArtifactFinalized",
                "system.transcriptBound publicInput proof",
                "system.publicInputBound publicInput proof",
                "system.pcsOpeningsValid publicInput proof",
                "system.friQueriesValid publicInput proof",
                "RuntimeVerifierCoreContract system publicInput proof",
                "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
                "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
                "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
            ],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_core_components_contract",
            "concreteSegmentIdsAllowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_artifact_finalized_concrete_core_requirements_contract()
{
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_checked_acceptance_artifact_finalized_concrete_core_requirements_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            &[
                "RuntimeArtifactEvidence",
                "RuntimeProofArtifactFinalized",
                "system.transcriptBound publicInput proof",
                "system.publicInputBound publicInput proof",
                "system.pcsOpeningsValid publicInput proof",
                "system.friQueriesValid publicInput proof",
                "RuntimeVerifierCoreContract system publicInput proof",
                "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
                "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
                "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
            ],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_finalized_concrete_segment_ids_contract",
            "artifactEvidence",
            "concreteSegmentIdsAllowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_concrete_segment_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_required_external_source_artifact_audited_concrete_segment_ids_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_required_source_guard(&lean_source, theorem);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            AUDITED_ARTIFACT_CORE_SNIPPETS,
            REQUIRED_SOURCE_SNIPPETS,
            SEGMENT_ID_SNIPPETS,
            &["RuntimeProofArtifactConcreteSegmentIdsAllowed proof"],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_concrete_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_core_components_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_required_source_guard(&lean_source, theorem);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            REQUIRED_SOURCE_SNIPPETS,
            &[
                "RequiredCryptographicAssumptionStatements assumptions.crypto",
                "RequiredSemanticAssumptionStatements assumptions.semantic",
                "RuntimeProofArtifactFinalized",
                "ProofSystemSound system",
                "system.accepts publicInput proof",
                "ExternalSourceOpeningEvidence",
                "system.transcriptBound publicInput proof",
                "system.publicInputBound publicInput proof",
                "system.pcsOpeningsValid publicInput proof",
                "system.friQueriesValid publicInput proof",
                "RuntimeVerifierCoreContract system publicInput proof",
                "exists witness trace constraints",
                "SoundWitness system publicInput proof",
            ],
            &[
                "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
                "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
                "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
            ],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract",
            "finalizedSegmentContract",
            "concreteSegmentIdsAllowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_core_requirements_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_required_external_source_finalized_concrete_core_source_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_required_source_guard(&lean_source, theorem);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            REQUIRED_SOURCE_SNIPPETS,
            &[
                "RuntimeProofArtifactFinalized",
                "ExternalSourceOpeningEvidence",
                "system.transcriptBound publicInput proof",
                "system.publicInputBound publicInput proof",
                "system.pcsOpeningsValid publicInput proof",
                "system.friQueriesValid publicInput proof",
                "RuntimeVerifierCoreContract system publicInput proof",
                "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
                "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
                "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
            ],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_required_external_source_audited_finalized_concrete_segment_ids_core_components_contract",
            "concreteSegmentIdsAllowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_artifact_core_source_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_required_external_source_artifact_finalized_concrete_core_source_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_required_source_guard(&lean_source, theorem);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            REQUIRED_SOURCE_SNIPPETS,
            &[
                "RuntimeArtifactEvidence",
                "RuntimeProofArtifactFinalized",
                "ExternalSourceOpeningEvidence",
                "system.transcriptBound publicInput proof",
                "system.publicInputBound publicInput proof",
                "system.pcsOpeningsValid publicInput proof",
                "system.friQueriesValid publicInput proof",
                "RuntimeVerifierCoreContract system publicInput proof",
                "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
                "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
                "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
            ],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract",
            "externalSourceEvidence",
            "concreteSegmentIdsAllowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_finalized_segment_contract() {
    let lean_source = read_contracts_source();
    let theorem = "runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_required_source_guard(&lean_source, theorem);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            AUDITED_ARTIFACT_CORE_SNIPPETS,
            REQUIRED_SOURCE_SNIPPETS,
            &["RuntimeProofArtifactFinalized"],
            SEGMENT_ID_SNIPPETS,
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
            "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_finalized_concrete_segment_contract() {
    let lean_source = read_contracts_source();
    let theorem =
        "runtime_soundness_required_external_source_artifact_audited_finalized_concrete_segment_ids_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    assert_required_source_guard(&lean_source, theorem);
    assert_prefix_contains_groups(
        &lean_source,
        theorem,
        &[
            &["RuntimeProofArtifactConcreteSegmentIdBinding"],
            AUDITED_ARTIFACT_CORE_SNIPPETS,
            REQUIRED_SOURCE_SNIPPETS,
            &["RuntimeProofArtifactFinalized"],
            SEGMENT_ID_SNIPPETS,
            &["RuntimeProofArtifactConcreteSegmentIdsAllowed proof"],
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    assert_body_omits_direct_assumption_access(&lean_source, theorem);
}
