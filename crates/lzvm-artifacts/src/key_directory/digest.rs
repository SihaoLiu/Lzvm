use crate::expression_program::encode_expression_program;
use crate::global_info::{CurveKind, GlobalInfo};
use crate::global_program::{encode_global_program, GlobalProgram};
use crate::pcs_material::PcsSetupMaterial;
use crate::pcs_plan::PcsSetupPlan;
use crate::regular_program::{encode_regular_program, RegularProgram};
use crate::source_fixed_file_manifest::{
    encode_source_fixed_file_manifest, SourceFixedFileManifest,
};
use crate::source_program::{encode_source_program_archive, SourceProgramArchive};
use crate::verification_key::VerificationKeyRoot;
use sha2::{Digest, Sha256};

use super::{KeyDirectoryCatalog, KeyDirectoryError, KeyUnitKind};

pub fn key_directory_catalog_digest(
    catalog: &KeyDirectoryCatalog,
) -> Result<[u8; 32], KeyDirectoryError> {
    let mut hasher = Sha256::new();
    hash_bytes(&mut hasher, b"lzvm-key-directory-catalog-v1");
    hash_global_info(&mut hasher, &catalog.layout.global_info);
    hash_bytes(
        &mut hasher,
        &encode_global_program(&GlobalProgram {
            constraints: catalog.global_constraints.clone(),
            hints: catalog.global_hints.clone(),
        })
        .map_err(|error| KeyDirectoryError::Digest {
            message: error.to_string(),
        })?,
    );
    hash_optional_source_fixed_file_manifest(
        &mut hasher,
        catalog.source_fixed_file_manifest.as_ref(),
    )?;
    hash_optional_source_program_archive(&mut hasher, catalog.source_program_archive.as_ref())?;
    hash_u64(&mut hasher, catalog.units.len() as u64);
    for unit in &catalog.units {
        hash_u8(&mut hasher, key_unit_kind_tag(unit.paths.kind));
        hash_optional_usize(&mut hasher, unit.paths.group_id);
        hash_optional_usize(&mut hasher, unit.paths.unit_id);
        hash_optional_string(&mut hasher, unit.paths.group_name.as_deref());
        hash_optional_string(&mut hasher, unit.paths.unit_name.as_deref());
        hash_pcs_setup_plan(&mut hasher, &unit.pcs_plan);
        hash_bytes(
            &mut hasher,
            &crate::setup_info::encode_unit_setup_info(&unit.metadata.setup).map_err(|error| {
                KeyDirectoryError::Digest {
                    message: error.to_string(),
                }
            })?,
        );
        hash_bytes(
            &mut hasher,
            &encode_regular_program(&RegularProgram {
                expressions: unit.expression_program.clone(),
                constraints: unit.regular_constraints.clone(),
                hints: unit.regular_hints.clone(),
            })
            .map_err(|error| KeyDirectoryError::Digest {
                message: error.to_string(),
            })?,
        );
        hash_bytes(
            &mut hasher,
            &encode_expression_program(&unit.verifier_program).map_err(|error| {
                KeyDirectoryError::Digest {
                    message: error.to_string(),
                }
            })?,
        );
        hash_root(&mut hasher, &unit.verification_key);
        hash_u64(&mut hasher, unit.expected_fixed_bytes as u64);
        hash_u64(&mut hasher, unit.actual_fixed_bytes);
        hash_bool(&mut hasher, unit.constant_tree_present);
        hash_optional_u64(&mut hasher, unit.constant_tree_bytes);
        hash_optional_root(&mut hasher, unit.constant_tree_root.as_ref());
        hash_bool(&mut hasher, unit.pcs_material_present);
        hash_optional_u64(&mut hasher, unit.pcs_material_bytes);
        hash_optional_pcs_setup_material(&mut hasher, unit.pcs_material.as_ref());
    }

    Ok(hasher.finalize().into())
}

pub fn key_directory_catalog_digest_hex(
    catalog: &KeyDirectoryCatalog,
) -> Result<String, KeyDirectoryError> {
    Ok(encode_digest_hex(&key_directory_catalog_digest(catalog)?))
}

fn key_unit_kind_tag(kind: KeyUnitKind) -> u8 {
    match kind {
        KeyUnitKind::Basic => 0,
        KeyUnitKind::Compressor => 1,
        KeyUnitKind::RecursiveFirst => 2,
        KeyUnitKind::RecursiveSecond => 3,
        KeyUnitKind::FinalAggregation => 4,
        KeyUnitKind::FinalCircuit => 5,
    }
}

fn hash_global_info(hasher: &mut Sha256, global: &GlobalInfo) {
    hash_string(hasher, &global.name);
    hash_string_vec(hasher, &global.air_groups);
    hash_u64(hasher, global.airs.len() as u64);
    for group in &global.airs {
        hash_u64(hasher, group.len() as u64);
        for unit in group {
            hash_string(hasher, &unit.name);
            hash_u64(hasher, unit.num_rows);
            hash_bool(hasher, unit.has_compressor);
        }
    }
    hash_u8(hasher, curve_kind_tag(&global.curve));
    hash_optional_u64(hasher, global.lattice_size);
    hash_u64(hasher, global.aggregation_types.len() as u64);
    for group in &global.aggregation_types {
        hash_u64(hasher, group.len() as u64);
        for entry in group {
            hash_u64(hasher, entry.aggregation_type);
        }
    }
    hash_u64(hasher, global.n_publics);
    hash_u64_vec(hasher, &global.num_challenges);
    hash_u64_vec(hasher, &global.num_proof_values);
    hash_u64(hasher, global.proof_values_map.len() as u64);
    for entry in &global.proof_values_map {
        hash_string(hasher, &entry.name);
        hash_u64(hasher, entry.stage);
        hash_optional_u64(hasher, entry.id);
        hash_u64_vec(hasher, &entry.lengths);
    }
    hash_u64(hasher, global.publics_map.len() as u64);
    for entry in &global.publics_map {
        hash_string(hasher, &entry.name);
        hash_u64(hasher, entry.stage);
        hash_u64_vec(hasher, &entry.lengths);
    }
    hash_u64(hasher, global.transcript_arity);
}

fn curve_kind_tag(curve: &CurveKind) -> u8 {
    match curve {
        CurveKind::None => 0,
        CurveKind::EcGfp5 => 1,
        CurveKind::EcMasFp5 => 2,
    }
}

fn hash_pcs_setup_plan(hasher: &mut Sha256, plan: &PcsSetupPlan) {
    hash_u32(hasher, plan.base_domain_bits);
    hash_u32(hasher, plan.extended_domain_bits);
    hash_u64(hasher, plan.base_domain_size);
    hash_u64(hasher, plan.extended_domain_size);
    hash_u64(hasher, plan.blowup_factor);
    hash_u32(hasher, plan.query_count);
    hash_u32(hasher, plan.proof_of_work_bits);
    hash_u32(hasher, plan.merkle_tree_arity);
    hash_optional_u32(hasher, plan.transcript_arity);
    hash_bool(hasher, plan.hash_commits);
    hash_u32(hasher, plan.constant_width);
    hash_u32_vec(hasher, &plan.stage_commit_widths);
    hash_i64_vec(hasher, &plan.opening_points);
    hash_u64(hasher, plan.fri_layers.len() as u64);
    for layer in &plan.fri_layers {
        hash_u32(hasher, layer.input_bits);
        hash_u32(hasher, layer.output_bits);
        hash_u64(hasher, layer.folding_factor);
    }
    hash_u32(hasher, plan.final_layer_bits);
}

fn hash_optional_pcs_setup_material(hasher: &mut Sha256, material: Option<&PcsSetupMaterial>) {
    match material {
        Some(material) => {
            hash_bool(hasher, true);
            hash_pcs_setup_material(hasher, material);
        }
        None => hash_bool(hasher, false),
    }
}

fn hash_pcs_setup_material(hasher: &mut Sha256, material: &PcsSetupMaterial) {
    hash_bytes(hasher, &material.plan_digest);
    hash_bytes(hasher, &material.fixed_column_digest);
    hash_bytes(hasher, &material.constant_tree_digest);
    for value in material.constant_tree_root {
        hash_u64(hasher, value);
    }
    hash_u64(hasher, material.fixed_byte_count);
    hash_u64(hasher, material.constant_tree_byte_count);
    hash_u64(hasher, material.leaf_byte_count);
    hash_u64(hasher, material.node_byte_count);
}

fn hash_optional_source_fixed_file_manifest(
    hasher: &mut Sha256,
    value: Option<&SourceFixedFileManifest>,
) -> Result<(), KeyDirectoryError> {
    match value {
        Some(value) => {
            hash_bool(hasher, true);
            let bytes = encode_source_fixed_file_manifest(value)?;
            hash_bytes(hasher, &bytes);
        }
        None => hash_bool(hasher, false),
    }
    Ok(())
}

fn hash_optional_source_program_archive(
    hasher: &mut Sha256,
    value: Option<&SourceProgramArchive>,
) -> Result<(), KeyDirectoryError> {
    match value {
        Some(value) => {
            hash_bool(hasher, true);
            let bytes = encode_source_program_archive(value)?;
            hash_bytes(hasher, &bytes);
        }
        None => hash_bool(hasher, false),
    }
    Ok(())
}

fn hash_u8(hasher: &mut Sha256, value: u8) {
    hasher.update([value]);
}

fn hash_bool(hasher: &mut Sha256, value: bool) {
    hash_u8(hasher, u8::from(value));
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn hash_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_le_bytes());
}

fn hash_i64(hasher: &mut Sha256, value: i64) {
    hasher.update(value.to_le_bytes());
}

fn hash_optional_u32(hasher: &mut Sha256, value: Option<u32>) {
    match value {
        Some(value) => {
            hash_bool(hasher, true);
            hash_u32(hasher, value);
        }
        None => hash_bool(hasher, false),
    }
}

fn hash_optional_u64(hasher: &mut Sha256, value: Option<u64>) {
    match value {
        Some(value) => {
            hash_bool(hasher, true);
            hash_u64(hasher, value);
        }
        None => hash_bool(hasher, false),
    }
}

fn hash_optional_usize(hasher: &mut Sha256, value: Option<usize>) {
    hash_optional_u64(hasher, value.map(|value| value as u64));
}

fn hash_bytes(hasher: &mut Sha256, value: &[u8]) {
    hash_u64(hasher, value.len() as u64);
    hasher.update(value);
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

fn hash_string_vec(hasher: &mut Sha256, values: &[String]) {
    hash_u64(hasher, values.len() as u64);
    for value in values {
        hash_string(hasher, value);
    }
}

fn hash_u64_vec(hasher: &mut Sha256, values: &[u64]) {
    hash_u64(hasher, values.len() as u64);
    for value in values {
        hash_u64(hasher, *value);
    }
}

fn hash_u32_vec(hasher: &mut Sha256, values: &[u32]) {
    hash_u64(hasher, values.len() as u64);
    for value in values {
        hash_u32(hasher, *value);
    }
}

fn hash_i64_vec(hasher: &mut Sha256, values: &[i64]) {
    hash_u64(hasher, values.len() as u64);
    for value in values {
        hash_i64(hasher, *value);
    }
}

fn hash_optional_string(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            hash_bool(hasher, true);
            hash_string(hasher, value);
        }
        None => hash_bool(hasher, false),
    }
}

fn hash_root(hasher: &mut Sha256, root: &VerificationKeyRoot) {
    match root {
        VerificationKeyRoot::FieldElements(values) => {
            hash_u8(hasher, 1);
            hash_u64(hasher, values.len() as u64);
            for value in values {
                hash_u64(hasher, *value);
            }
        }
    }
}

fn hash_optional_root(hasher: &mut Sha256, root: Option<&VerificationKeyRoot>) {
    match root {
        Some(root) => {
            hash_bool(hasher, true);
            hash_root(hasher, root);
        }
        None => hash_bool(hasher, false),
    }
}

fn encode_digest_hex(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
