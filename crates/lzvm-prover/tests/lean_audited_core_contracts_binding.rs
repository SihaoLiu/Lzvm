use std::path::{Path, PathBuf};

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_audited_core_contracts_use_direct_evidence_paths() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_root = crate_root.join("../../lean/Lzvm");
    let mut lean_files = Vec::new();
    collect_lean_files(&lean_root, &mut lean_files);
    lean_files.sort();

    let mut indirect_contracts = Vec::new();
    for path in lean_files {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("Lean source {} should read: {err}", path.display()));
        for theorem_name in lean_binding::theorem_names(&source) {
            let Some(stem) = theorem_name.strip_suffix("_audited_core_contract") else {
                continue;
            };
            let theorem_body = lean_binding::theorem_body(&source, &theorem_name);
            let aggregate_helper = format!("{stem}_core_and_sound");
            let uses_aggregate =
                lean_binding::visible_identifier_occurrence_count(&theorem_body, &aggregate_helper)
                    > 0;
            let uses_combined_evidence = lean_binding::visible_identifier_occurrence_count(
                &theorem_body,
                "assumption_bundle_carries_required_evidence",
            ) > 0;
            if uses_aggregate || uses_combined_evidence {
                indirect_contracts.push(format!(
                    "{}::{theorem_name}",
                    path.strip_prefix(&lean_root).unwrap_or(&path).display()
                ));
            }
        }
    }

    assert!(
        indirect_contracts.is_empty(),
        "Lean audited core contracts should use direct evidence paths:\n{}",
        indirect_contracts.join("\n")
    );
}

#[test]
fn lean_sources_avoid_combined_required_evidence_helper() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_root = crate_root.join("../../lean/Lzvm");
    let mut lean_files = Vec::new();
    collect_lean_files(&lean_root, &mut lean_files);
    lean_files.sort();

    let mut combined_evidence_users = Vec::new();
    for path in lean_files {
        if path.file_name().and_then(|name| name.to_str()) == Some("AssumptionAudit.lean") {
            continue;
        }
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("Lean source {} should read: {err}", path.display()));
        if lean_binding::visible_identifier_occurrence_count(
            &source,
            "assumption_bundle_carries_required_evidence",
        ) > 0
        {
            combined_evidence_users.push(
                path.strip_prefix(&lean_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        combined_evidence_users.is_empty(),
        "Lean sources should route required evidence through direct helpers:\n{}",
        combined_evidence_users.join("\n")
    );
}

fn collect_lean_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).expect("Lean source directory should read") {
        let path = entry.expect("Lean source entry should read").path();
        if path.is_dir() {
            collect_lean_files(&path, files);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("lean") {
            files.push(path);
        }
    }
}
