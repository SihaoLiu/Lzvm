use std::path::PathBuf;

use lzvm_artifacts::verification_key::VerificationKeyRoot;

use crate::directory_manifest::SetupDirectoryManifestWriteReport;
use crate::pcs::PcsDirectoryWriteReport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedColumnWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTreeWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
    pub root: VerificationKeyRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantTreeLeavesWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
    pub row_count: u64,
    pub column_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationKeyWriteReport {
    pub binary_path: PathBuf,
    pub binary_bytes: u64,
    pub root: VerificationKeyRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseNativeWriteReport {
    pub fixed: FixedColumnWriteReport,
    pub tree: ConstantTreeWriteReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseDirectoryWriteReport {
    pub unit_count: usize,
    pub fixed_bytes: u64,
    pub tree_bytes: u64,
    pub verkey_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDirectoryWriteReport {
    pub base: BaseDirectoryWriteReport,
    pub pcs_plan: PcsDirectoryWriteReport,
    pub pcs_material: PcsDirectoryWriteReport,
    pub manifest: SetupDirectoryManifestWriteReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupDirectorySummaryReport {
    pub unit_count: usize,
    pub global_constraint_count: usize,
    pub fixed_bytes: u64,
    pub pcs_material_unit_count: usize,
    pub pcs_material_bytes: u64,
    pub source_fixed_file_manifest_present: bool,
    pub source_fixed_file_manifest_entry_count: usize,
    pub fingerprint: String,
}
