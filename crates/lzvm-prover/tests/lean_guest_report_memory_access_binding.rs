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
            "low_bytes_full_width",
            "compact_guest_memory_one_reconstructs",
            "compact_guest_memory_owned_one_preserves_access",
            "compact_guest_memory_pair_preserves_accesses",
            "compact_guest_memory_many_preserves_accesses",
            "compact_guest_memory_precompile_preserves_views",
            "compact_guest_memory_precompile_empty_matches_normal_view",
            "compact_single_guest_memory_access_preserves_view",
            "compact_single_guest_memory_access_uses_one_on_exact_match",
            "compact_single_guest_memory_access_falls_back_on_mismatch",
        ],
    );
    assert!(
        lean_source.contains("inductive CompactSingleAccessShape where")
            && lean_source.contains("def CompactSingleAccessShape.reconstruct")
            && lean_source.contains("inductive CompactGuestMemoryAccessStorage where")
            && lean_source.contains("| ownedOne (access : GuestMemoryAccess)")
            && lean_source.contains("| pair (first second : GuestMemoryAccess)")
            && lean_source.contains("def CompactGuestMemoryAccessStorage.normalAccesses")
            && lean_source.contains("def CompactGuestMemoryAccessStorage.normalIsEmpty")
            && lean_source.contains("def compactSingleGuestMemoryAccess"),
        "Lean compact report storage should expose reconstruction, fallback, and logical views"
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
    assert!(
        runtime_source.contains("struct GuestReportMemoryAccessList")
            && runtime_source.contains("tagged: usize")
            && runtime_source.contains("enum GuestReportMemoryAccessRef")
            && runtime_source.contains("One(u64),")
            && runtime_source.contains("OwnedOne(&'a GuestMemoryAccess),")
            && runtime_source.contains("Pair(&'a [GuestMemoryAccess; 2]),")
            && runtime_source.contains("Many(&'a Vec<GuestMemoryAccess>),")
            && runtime_source.contains("const TAG_MASK: usize = 0b111;")
            && runtime_source.contains("const OWNED_ONE_TAG: usize = 2;")
            && runtime_source.contains("const PRECOMPILE_TAG: usize = 5;")
            && runtime_source.contains("fn compact_single_memory_access(")
            && runtime_source.contains("== Some(access)")
            && runtime_source.contains("access.address <= (usize::MAX >> 3) as u64")
            && runtime_source.contains("Self::from_box(Box::new(access), Self::OWNED_ONE_TAG)")
            && runtime_source.contains("impl Drop for GuestReportMemoryAccessList")
            && runtime_source.contains("effects.normal_memory_accesses.is_empty()"),
        "runtime compact report storage should match the Lean reconstruction and fallback model"
    );
}
