use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::{
    encode_expression_info, read_expression_info_binary_file, ExpressionInfo,
};
use lzvm_artifacts::expression_program::{encode_expression_program, read_expression_program_file};
use lzvm_artifacts::fixed::{read_fixed_columns_file, read_fixed_columns_file_for_setup};
use lzvm_artifacts::global_info::{encode_global_info, read_global_info_binary_file, GlobalInfo};
use lzvm_artifacts::key_directory::{
    read_key_directory_layout, KeyDirectoryError, KeyDirectoryLayout,
};
use lzvm_artifacts::regular_program::{
    encode_regular_program, read_regular_program_file, regular_program_from_expression_info,
    verifier_program_from_verifier_info,
};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, read_unit_setup_info_binary_file, UnitSetupInfo,
};
use lzvm_artifacts::verification_key::read_verification_key_binary_file;
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, read_verifier_info_binary_file, VerifierInfo,
};

mod backend;
mod constant_tree;
mod directory_manifest;
mod errors;
mod pcs;
mod program_image;
mod reports;
mod source_companions;
mod source_fixed_columns;
mod source_fixed_expression;
mod source_fixed_file_manifest;
mod source_fixed_sequence;
mod source_key_directory;
mod source_program_archive;
mod source_row_count;
mod source_scope;
mod source_static_values;
mod staging;

pub use backend::*;
pub use constant_tree::{
    build_constant_tree_from_fixed_columns, build_constant_tree_from_fixed_columns_with_backend,
    build_constant_tree_from_leaves, build_constant_tree_from_leaves_with_backend,
    extend_fixed_columns_for_constant_tree, extend_fixed_columns_for_constant_tree_with_backend,
    write_base_constant_tree, write_base_fixed_columns, write_constant_tree_from_fixed_columns,
    write_constant_tree_leaves, write_constant_tree_leaves_with_backend,
    write_verification_key_from_constant_tree,
};
pub use directory_manifest::{
    summarize_setup_directory, write_setup_directory_manifest, SetupDirectoryManifestWriteReport,
    SetupDirectorySummaryError,
};
pub use errors::*;
pub use pcs::{
    write_pcs_directory, write_pcs_directory_from_layout, write_pcs_material_directory,
    write_pcs_material_directory_from_layout, write_pcs_setup_material_file,
    write_pcs_setup_plan_file, PcsDirectoryWriteError, PcsDirectoryWriteReport, PcsFileWriteReport,
};
pub use program_image::{
    write_program_image_commitment_cache, write_program_image_commitment_cache_file,
    write_program_image_commitment_cache_file_for_setup_directory,
    ProgramImageCommitmentCacheFileRequest, ProgramImageCommitmentCacheForSetupDirectoryRequest,
    ProgramImageCommitmentCacheWriteError, ProgramImageCommitmentCacheWriteReport,
};
pub use reports::*;
pub use source_companions::{
    write_source_companions, SourceCompanionWriteError, SourceCompanionWriteReport,
    SourceCompanionWriteRequest,
};
pub use source_fixed_columns::{
    write_fixed_columns_from_source_directory, write_fixed_columns_from_source_file,
    SourceFixedColumnsDirectoryWriteError, SourceFixedColumnsDirectoryWriteReport,
    SourceFixedColumnsDirectoryWriteRequest, SourceFixedColumnsWriteError,
    SourceFixedColumnsWriteReport, SourceFixedColumnsWriteRequest,
};
pub use source_fixed_file_manifest::{
    source_fixed_file_manifest_from_resolved, write_source_fixed_file_manifest,
    SourceFixedFileManifestWriteError, SourceFixedFileManifestWriteReport,
    SourceFixedFileManifestWriteRequest,
};
pub use source_key_directory::{
    write_source_key_directory_metadata, SourceKeyDirectoryMetadataError,
    SourceKeyDirectoryMetadataReport, SourceKeyDirectoryMetadataRequest,
};
pub use source_program_archive::{
    write_source_program_archive, SourceProgramArchiveWriteError, SourceProgramArchiveWriteReport,
    SourceProgramArchiveWriteRequest,
};
pub(crate) use staging::{publish_staging_bytes, staging_path_for, write_staging_bytes};

pub fn write_fixed_columns_native_file(
    setup_info_path: impl AsRef<Path>,
    columns_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<FixedColumnWriteReport, NativeFileWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let columns = read_fixed_columns_file(columns_path)?;
    write_base_fixed_columns(output_path, &columns, &setup).map_err(Into::into)
}

pub fn write_base_native_files(
    setup_info_path: impl AsRef<Path>,
    columns_path: impl AsRef<Path>,
    fixed_output_path: impl AsRef<Path>,
    tree_output_path: impl AsRef<Path>,
    backend: FixedExtensionBackend,
) -> Result<BaseNativeWriteReport, NativeFileWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let columns = read_fixed_columns_file_for_setup(columns_path, &setup, "raw", "unit")?;
    let tree = build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)?;
    let fixed = write_base_fixed_columns(fixed_output_path, &columns, &setup)?;
    let tree = write_base_constant_tree(tree_output_path, &tree, &setup, None)?;
    Ok(BaseNativeWriteReport { fixed, tree })
}

pub fn write_verification_key_native_file(
    setup_info_path: impl AsRef<Path>,
    constant_tree_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<VerificationKeyWriteReport, NativeFileWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let tree = read_native_input_bytes(
        constant_tree_path.as_ref(),
        "read constant-tree input for verification-key write",
    )?;
    write_verification_key_from_constant_tree(output_path, &tree, &setup).map_err(Into::into)
}

pub fn write_constant_tree_file(
    setup_info_path: impl AsRef<Path>,
    tree_path: impl AsRef<Path>,
    root_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<ConstantTreeWriteReport, NativeFileWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let tree = read_native_input_bytes(tree_path.as_ref(), "read constant-tree input")?;
    let root = read_verification_key_binary_file(root_path).map_err(SetupError::from)?;
    write_base_constant_tree(output_path, &tree, &setup, Some(&root)).map_err(Into::into)
}

pub fn write_constant_tree_leaves_file(
    setup_info_path: impl AsRef<Path>,
    columns_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    backend: FixedExtensionBackend,
) -> Result<ConstantTreeLeavesWriteReport, NativeFileWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let columns = read_fixed_columns_file(columns_path)?;
    write_constant_tree_leaves_with_backend(output_path, &columns, &setup, backend)
        .map_err(Into::into)
}

pub fn write_constant_tree_native_file(
    setup_info_path: impl AsRef<Path>,
    columns_path: impl AsRef<Path>,
    root_path: Option<impl AsRef<Path>>,
    output_path: impl AsRef<Path>,
    backend: FixedExtensionBackend,
) -> Result<ConstantTreeWriteReport, NativeFileWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let columns = read_fixed_columns_file_for_setup(columns_path, &setup, "raw", "unit")?;
    let expected_root = match root_path {
        Some(path) => Some(read_verification_key_binary_file(path).map_err(SetupError::from)?),
        None => None,
    };
    let tree = build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)?;
    write_base_constant_tree(output_path, &tree, &setup, expected_root.as_ref()).map_err(Into::into)
}

fn read_native_input_bytes(
    path: &Path,
    role: &'static str,
) -> Result<Vec<u8>, NativeFileWriteError> {
    std::fs::read(path)
        .map_err(|error| SetupError::Io {
            role,
            path: path.to_path_buf(),
            message: error.to_string(),
        })
        .map_err(Into::into)
}

pub fn write_base_directory(
    root: impl AsRef<Path>,
    backend: FixedExtensionBackend,
    derive_verkey: bool,
) -> Result<BaseDirectoryWriteReport, BaseDirectoryWriteError> {
    let layout = read_key_directory_layout(root).map_err(BaseDirectoryWriteError::from)?;
    write_base_directory_from_layout(&layout, backend, derive_verkey)
}

pub fn write_base_directory_from_layout(
    layout: &KeyDirectoryLayout,
    backend: FixedExtensionBackend,
    derive_verkey: bool,
) -> Result<BaseDirectoryWriteReport, BaseDirectoryWriteError> {
    validate_base_directory_inputs(layout, derive_verkey)?;
    write_global_info_binary_for_directory(&layout.global_paths.info, &layout.global_info)?;

    let mut fixed_bytes = 0_u64;
    let mut tree_bytes = 0_u64;
    let mut verkey_bytes = 0_u64;
    for unit in &layout.units {
        let setup_path = require_base_unit_path(unit.setup_info(), "setup metadata path")?;
        let setup = read_unit_setup_info_binary_file(&setup_path)?;
        if let Some(path) = unit.setup_info_binary() {
            write_unit_setup_info_binary_for_directory(&path, &setup)?;
        }

        let expression_path =
            require_base_unit_path(unit.expression_info(), "expression metadata path")?;
        let expressions = read_expression_info_binary_file(&expression_path)?;
        if let Some(path) = unit.expression_info_binary() {
            write_expression_info_binary_for_directory(&path, &expressions)?;
        }
        if let Some(path) = unit.expression_program() {
            write_regular_program_for_directory(&path, &expressions, &setup)?;
        }

        let verifier_path = require_base_unit_path(unit.verifier_info(), "verifier metadata path")?;
        let verifier = read_verifier_info_binary_file(&verifier_path)?;
        if let Some(path) = unit.verifier_info_binary() {
            write_verifier_info_binary_for_directory(&path, &verifier)?;
        }
        if let Some(path) = unit.verifier_program() {
            write_verifier_program_for_directory(&path, &verifier, &setup, &layout.global_info)?;
        }

        let group_name = unit.group_name.as_deref().unwrap_or("raw");
        let unit_name = unit.unit_name.as_deref().unwrap_or("unit");
        let columns =
            read_fixed_columns_file_for_setup(&unit.fixed_columns, &setup, group_name, unit_name)?;
        let expected_root = if derive_verkey {
            None
        } else {
            Some(read_verification_key_binary_file(
                unit.verification_key_binary(),
            )?)
        };
        let tree = build_constant_tree_from_fixed_columns_with_backend(&columns, &setup, backend)?;
        let fixed_report = write_base_fixed_columns(&unit.fixed_columns, &columns, &setup)?;
        let tree_report =
            write_base_constant_tree(&unit.constant_tree, &tree, &setup, expected_root.as_ref())?;

        if derive_verkey {
            let key_report = write_verification_key_from_constant_tree(
                unit.verification_key_binary(),
                &tree,
                &setup,
            )?;
            verkey_bytes = verkey_bytes.saturating_add(key_report.binary_bytes);
        }

        fixed_bytes = fixed_bytes.saturating_add(fixed_report.bytes_written);
        tree_bytes = tree_bytes.saturating_add(tree_report.bytes_written);
    }

    Ok(BaseDirectoryWriteReport {
        unit_count: layout.units.len(),
        fixed_bytes,
        tree_bytes,
        verkey_bytes: if derive_verkey {
            Some(verkey_bytes)
        } else {
            None
        },
    })
}

pub fn write_key_directory(
    root: impl AsRef<Path>,
    backend: FixedExtensionBackend,
) -> Result<KeyDirectoryWriteReport, KeyDirectoryWriteError> {
    let layout = read_key_directory_layout(root).map_err(BaseDirectoryWriteError::from)?;
    write_key_directory_from_layout(&layout, backend)
}

pub fn write_key_directory_from_layout(
    layout: &KeyDirectoryLayout,
    backend: FixedExtensionBackend,
) -> Result<KeyDirectoryWriteReport, KeyDirectoryWriteError> {
    let base = write_base_directory_from_layout(layout, backend, true)?;
    let pcs_plan = write_pcs_directory_from_layout(layout)?;
    let pcs_material = write_pcs_material_directory_from_layout(layout)?;
    let manifest = directory_manifest::write_setup_directory_manifest_for_layout(layout)?;
    Ok(KeyDirectoryWriteReport {
        base,
        pcs_plan,
        pcs_material,
        manifest,
    })
}

fn validate_base_directory_inputs(
    layout: &KeyDirectoryLayout,
    derive_verkey: bool,
) -> Result<(), BaseDirectoryWriteError> {
    let mut seen = BTreeSet::new();
    for required in layout.required_paths() {
        if matches!(
            required.role,
            "unit expression program" | "unit verifier program"
        ) {
            continue;
        }
        if matches!(
            required.role,
            "unit verification-key metadata" | "unit verification-key binary"
        ) && derive_verkey
        {
            continue;
        }
        if !seen.insert(required.path.clone()) {
            continue;
        }
        if !required.path.is_file() {
            return Err(KeyDirectoryError::MissingPath {
                role: required.role,
                path: required.path,
            }
            .into());
        }
    }
    Ok(())
}

fn require_base_unit_path(
    path: Option<PathBuf>,
    role: &'static str,
) -> Result<PathBuf, BaseDirectoryWriteError> {
    path.ok_or(BaseDirectoryWriteError::MissingUnitPath { role })
}

fn write_global_info_binary_for_directory(
    path: &Path,
    global_info: &GlobalInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_global_info(global_info)?;
    write_validated_base_directory_bytes(
        path,
        &bytes,
        "write global-info binary staging file",
        "publish global-info binary",
        |staging_path| {
            read_global_info_binary_file(staging_path)?;
            Ok(())
        },
    )
}

fn write_unit_setup_info_binary_for_directory(
    path: &Path,
    setup: &UnitSetupInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_unit_setup_info(setup)?;
    write_validated_base_directory_bytes(
        path,
        &bytes,
        "write setup metadata binary staging file",
        "publish setup metadata binary",
        |staging_path| {
            read_unit_setup_info_binary_file(staging_path)?;
            Ok(())
        },
    )
}

fn write_expression_info_binary_for_directory(
    path: &Path,
    expressions: &ExpressionInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_expression_info(expressions)?;
    write_validated_base_directory_bytes(
        path,
        &bytes,
        "write expression metadata binary staging file",
        "publish expression metadata binary",
        |staging_path| {
            read_expression_info_binary_file(staging_path)?;
            Ok(())
        },
    )
}

fn write_regular_program_for_directory(
    path: &Path,
    expressions: &ExpressionInfo,
    setup: &UnitSetupInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let program = regular_program_from_expression_info(expressions, setup)?;
    let bytes = encode_regular_program(&program)?;
    write_validated_base_directory_bytes(
        path,
        &bytes,
        "write regular program staging file",
        "publish regular program",
        |staging_path| {
            read_regular_program_file(staging_path)?;
            Ok(())
        },
    )
}

fn write_verifier_info_binary_for_directory(
    path: &Path,
    verifier: &VerifierInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_verifier_info(verifier)?;
    write_validated_base_directory_bytes(
        path,
        &bytes,
        "write verifier metadata binary staging file",
        "publish verifier metadata binary",
        |staging_path| {
            read_verifier_info_binary_file(staging_path)?;
            Ok(())
        },
    )
}

fn write_verifier_program_for_directory(
    path: &Path,
    verifier: &VerifierInfo,
    setup: &UnitSetupInfo,
    global: &GlobalInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let program = verifier_program_from_verifier_info(verifier, setup, global)?;
    let bytes = encode_expression_program(&program)?;
    write_validated_base_directory_bytes(
        path,
        &bytes,
        "write verifier program staging file",
        "publish verifier program",
        |staging_path| {
            read_expression_program_file(staging_path)?;
            Ok(())
        },
    )
}

fn write_validated_base_directory_bytes(
    path: &Path,
    bytes: &[u8],
    write_role: &'static str,
    publish_role: &'static str,
    validate: impl FnOnce(&Path) -> Result<(), BaseDirectoryWriteError>,
) -> Result<u64, BaseDirectoryWriteError> {
    let staging_path = write_staging_bytes(path, bytes, write_role)?;
    validate(&staging_path)?;
    publish_staging_bytes(&staging_path, path, publish_role).map_err(Into::into)
}
