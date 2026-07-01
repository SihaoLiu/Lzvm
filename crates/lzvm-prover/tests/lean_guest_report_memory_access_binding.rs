use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_guest_report_memory_access_binding_tracks_runtime_storage_shape() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source =
        lean_binding::read_lean_source(crate_root, "../../lean/Lzvm/GuestReportMemoryAccess.lean");
    let top_level_source = lean_binding::read_lean_source(crate_root, "../../lean/Lzvm.lean");
    let runtime_source = std::fs::read_to_string(crate_root.join("src/guest_machine/mod.rs"))
        .expect("guest machine source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.GuestReportMemoryAccess"),
        "top-level Lean module should import guest report memory access"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &lean_source,
            "structure GuestMemoryAccess where",
            "structure GuestPrecompileReportEffects where",
        ),
        vec!["kind", "address", "byteLen", "value"],
        "Lean guest memory access should mirror the runtime memory access payload"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &lean_source,
            "structure GuestPrecompileReportEffects where",
            "inductive GuestMemoryAccessStorage where",
        ),
        vec!["normalMemoryAccesses", "precompileMemoryAccesses", "result",],
        "Lean precompile effects should retain separate normal and precompile views"
    );
    assert!(
        lean_source.contains("def GuestMemoryAccessStorage.normalAccesses")
            && lean_source.contains("def GuestMemoryAccessStorage.precompileAccesses")
            && lean_source.contains("def GuestMemoryAccessStorage.precompileResult")
            && lean_source.contains("def FoldedGuestMemoryEffectsCanonical")
            && lean_source.contains("| empty")
            && lean_source.contains("| one (access : GuestMemoryAccess)")
            && lean_source.contains("| many (accesses : List GuestMemoryAccess)")
            && lean_source.contains("| precompile (effects : GuestPrecompileReportEffects)"),
        "Lean guest memory storage should expose folded storage and logical views"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "folded_guest_memory_empty_views",
            "folded_guest_memory_one_normal_view",
            "folded_guest_memory_many_normal_view",
            "folded_guest_memory_precompile_views",
            "folded_guest_memory_nonprecompile_has_no_precompile_accesses",
            "folded_guest_memory_nonprecompile_has_no_precompile_result",
            "folded_guest_memory_precompile_preserves_normal_accesses",
            "folded_guest_memory_precompile_preserves_precompile_accesses",
            "folded_guest_memory_precompile_preserves_result",
        ],
    );
    assert!(
        runtime_source.contains("pub struct GuestMemoryAccess")
            && runtime_source.contains("pub enum GuestMemoryAccessKind")
            && runtime_source.contains("pub struct GuestMemoryAccessList")
            && runtime_source.contains("enum GuestMemoryAccessEntries")
            && runtime_source.contains("Empty,")
            && runtime_source.contains("One(GuestMemoryAccess),")
            && runtime_source.contains("Many(Box<[GuestMemoryAccess]>),")
            && runtime_source.contains("Precompile(Box<GuestPrecompileReportEffects>),")
            && runtime_source.contains("pub struct GuestPrecompileReportEffects")
            && runtime_source.contains("normal_memory_accesses")
            && runtime_source.contains("fn with_precompile_effects")
            && runtime_source.contains("fn precompile_memory_accesses(&self)")
            && runtime_source.contains("fn precompile_result(&self)"),
        "runtime guest memory storage should keep the folded shape represented in Lean"
    );
}
