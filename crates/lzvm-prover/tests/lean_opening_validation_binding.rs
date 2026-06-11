use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_opening_validation_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OpeningValidation.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean opening validation should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.OpeningValidation"),
        "top-level Lean module should import opening validation"
    );
    assert!(
        lean_source.contains("RuntimeOpeningValidation")
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean opening validation should expose checked and required-source verifier core projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_opening_evidence_implies_external_source_requirement",
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_verifier_core_contract",
            "runtime_opening_required_external_source_sound",
            "runtime_opening_required_external_source_verifier_core_contract",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_opening_evidence_implies_external_source_requirement"
        )
        .contains("RuntimeOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_evidence_implies_external_source_requirement"
            )
            .contains("ExternalSourceOpeningRequirement")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_evidence_implies_external_source_requirement"
            )
            .contains("validation.runtimeSoundnessValidation.sourceValidation"),
        "opening evidence should expose the external-source requirement carried by runtime soundness evidence"
    );
}

fn theorem_prefix(source: &str, name: &str) -> String {
    let theorem_start = source
        .find(&format!("theorem {name}"))
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
    source[theorem_start..theorem_start + proof_start].to_owned()
}
