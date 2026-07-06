use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_guest_report_register_write_binding_exports_canonical_views() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/GuestReportRegisterWrite.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean guest register write source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.GuestReportRegisterWrite"),
        "top-level Lean module should import guest register write"
    );
    assert!(
        lean_source.contains("CompactGuestRegisterWriteCanonical")
            && lean_source.contains("reconstructGuestRegisterWrites"),
        "Lean guest register write module should expose compact reconstruction"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "compact_guest_register_write_none_empty",
            "compact_guest_register_write_register_singleton",
            "compact_guest_register_write_list_length_le_one",
            "compact_guest_register_write_canonical_none",
            "compact_guest_register_write_canonical_register",
            "compact_guest_register_write_register_value",
            "compact_guest_register_write_register_index",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "compact_guest_register_write_none_empty",
        &["reconstructGuestRegisterWrites GuestRegisterWriteDestination.none compact = []"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "compact_guest_register_write_register_singleton",
        &["[{ index := index, value := compact.value }]"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "compact_guest_register_write_list_length_le_one",
        &["cases destination", "simp [reconstructGuestRegisterWrites]"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "compact_guest_register_write_canonical_none",
        &["exact canonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "compact_guest_register_write_canonical_register",
        &["exact canonical"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "compact_guest_register_write_register_value",
        &["map GuestRegisterWrite.value = [compact.value]"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "compact_guest_register_write_register_index",
        &["map GuestRegisterWrite.index = [index]"],
    );
}
