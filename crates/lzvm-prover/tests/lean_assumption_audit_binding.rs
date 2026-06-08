use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_assumption_audit_exports_runtime_soundness_coverage() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let audit_path = crate_root.join("../../lean/Lzvm/AssumptionAudit.lean");
    let audit_source =
        std::fs::read_to_string(&audit_path).expect("Lean assumption audit source should read");
    let runtime_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let runtime_source =
        std::fs::read_to_string(&runtime_path).expect("Lean runtime soundness source should read");
    let soundness_path = crate_root.join("../../lean/Lzvm/Soundness.lean");
    let soundness_source =
        std::fs::read_to_string(&soundness_path).expect("Lean soundness source should read");

    assert!(
        runtime_source.contains("import Lzvm.AssumptionAudit"),
        "runtime soundness should import the centralized assumption audit"
    );
    assert!(
        runtime_source.contains("assumption_bundle_carries_required_crypto_evidence"),
        "runtime soundness should use the audited assumption bundle projection"
    );
    lean_binding::assert_theorem_declarations(
        &audit_source,
        &[
            "cryptographic_assumptions_carry_required_evidence",
            "assumption_bundle_carries_required_crypto_evidence",
        ],
    );
    lean_binding::assert_theorem_declarations(
        &runtime_source,
        &["runtime_soundness_checked_acceptance_audited_assumptions"],
    );
    assert!(
        soundness_source.contains("import Lzvm.AssumptionAudit"),
        "abstract soundness should import the centralized assumption audit"
    );
    assert!(
        soundness_source.contains("assumption_bundle_carries_required_crypto_evidence"),
        "abstract soundness should use the audited assumption bundle projection"
    );
    lean_binding::assert_theorem_declarations(
        &soundness_source,
        &["abstract_verifier_sound_with_audited_assumptions"],
    );
}
