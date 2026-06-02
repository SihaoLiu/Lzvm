use crate::constant_tree::{
    expected_constant_tree_leaf_node_byte_counts, summarize_constant_tree_file, ConstantTreeError,
    ConstantTreeFileSummary,
};
use crate::constraint_program::{
    read_global_constraint_program_file, read_regular_constraint_program_file, ConstraintProgram,
    ConstraintProgramError, GlobalConstraintProgram,
};
use crate::expression_program::{
    read_expression_program_file, ExpressionProgram, ExpressionProgramError,
};
use crate::fixed::{expected_raw_fixed_column_byte_count, FixedColumnError};
use crate::global_info::{read_global_info_binary_file, GlobalInfo, GlobalInfoError};
use crate::hint_program::{
    read_global_hint_program_file, read_regular_hint_program_file,
    regular_hint_program_from_expression_info, HintProgram, HintProgramError,
};
use crate::metadata_bundle::{
    read_unit_metadata_bundle, MetadataBundleError, UnitMetadataBundle, UnitMetadataPaths,
};
use crate::pcs_material::{read_pcs_setup_material_file, PcsSetupMaterial, PcsSetupMaterialError};
use crate::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, read_pcs_setup_plan_file, PcsPlanError,
    PcsSetupPlan,
};
use crate::source_fixed_file_manifest::{
    read_source_fixed_file_manifest_file, SourceFixedFileManifest, SourceFixedFileManifestError,
    SOURCE_FIXED_FILE_MANIFEST_FILE,
};
use crate::source_program::{
    read_source_program_archive_file, SourceProgramArchive, SourceProgramArchiveError,
    SOURCE_PROGRAM_ARCHIVE_FILE,
};
use crate::verification_key::{
    read_verification_key_binary_file, VerificationKeyError, VerificationKeyRoot,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

mod digest;

pub use digest::{key_directory_catalog_digest, key_directory_catalog_digest_hex};

const GLOBAL_INFO_BIN_FILE: &str = "pilout.globalInfo.bin";
const GLOBAL_CONSTRAINTS_BIN_FILE: &str = "pilout.globalConstraints.bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDirectoryLayout {
    pub root: PathBuf,
    pub global_info: GlobalInfo,
    pub global_paths: GlobalKeyPaths,
    pub source_fixed_file_manifest: PathBuf,
    pub source_program_archive: PathBuf,
    pub units: Vec<KeyUnitPaths>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyDirectoryCatalog {
    pub layout: KeyDirectoryLayout,
    pub global_constraints: GlobalConstraintProgram,
    pub global_hints: HintProgram,
    pub source_fixed_file_manifest: Option<SourceFixedFileManifest>,
    pub source_program_archive: Option<SourceProgramArchive>,
    pub units: Vec<KeyUnitCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalKeyPaths {
    pub info: PathBuf,
    pub constraints_program: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyUnitKind {
    Basic,
    Compressor,
    RecursiveFirst,
    RecursiveSecond,
    FinalAggregation,
    FinalCircuit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUnitPaths {
    pub kind: KeyUnitKind,
    pub group_id: Option<usize>,
    pub unit_id: Option<usize>,
    pub group_name: Option<String>,
    pub unit_name: Option<String>,
    pub prefix: PathBuf,
    pub metadata_prefix: Option<PathBuf>,
    pub program_prefix: Option<PathBuf>,
    pub verification_key_prefix: PathBuf,
    pub fixed_columns: PathBuf,
    pub constant_tree: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyUnitCatalogEntry {
    pub paths: KeyUnitPaths,
    pub metadata: UnitMetadataBundle,
    pub pcs_plan: PcsSetupPlan,
    pub verification_key: VerificationKeyRoot,
    pub expression_program: ExpressionProgram,
    pub regular_constraints: ConstraintProgram,
    pub regular_hints: HintProgram,
    pub verifier_program: ExpressionProgram,
    pub expected_fixed_bytes: usize,
    pub actual_fixed_bytes: u64,
    pub constant_tree_present: bool,
    pub constant_tree_bytes: Option<u64>,
    pub constant_tree_root: Option<VerificationKeyRoot>,
    pub pcs_material_present: bool,
    pub pcs_material_bytes: Option<u64>,
    pub pcs_material: Option<PcsSetupMaterial>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredPath {
    pub role: &'static str,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyDirectoryError {
    GlobalInfo(GlobalInfoError),
    GlobalConstraints(ConstraintProgramError),
    GlobalHints(HintProgramError),
    RegularConstraints(ConstraintProgramError),
    RegularHints(HintProgramError),
    ConstantTree(ConstantTreeError),
    Metadata(MetadataBundleError),
    PcsPlan(PcsPlanError),
    PcsMaterial(PcsSetupMaterialError),
    ExpressionProgram(ExpressionProgramError),
    VerificationKey(VerificationKeyError),
    FixedColumns(FixedColumnError),
    SourceFixedFileManifest(SourceFixedFileManifestError),
    SourceProgramArchive(SourceProgramArchiveError),
    SourceFixedFileManifestGroupMismatch {
        entry_index: usize,
        group_id: u64,
        group_name: String,
    },
    SourceFixedFileManifestUnitMismatch {
        entry_index: usize,
        group_id: u64,
        unit_id: u64,
        unit_name: String,
    },
    SourceFixedFileManifestSourceMismatch {
        entry_index: usize,
        source_name: String,
    },
    SourceFixedFileManifestSpanMismatch {
        entry_index: usize,
        source_name: String,
        start: u64,
        end: u64,
        source_len: usize,
    },
    SourceFixedFileManifestUtf8BoundaryMismatch {
        entry_index: usize,
        source_name: String,
        start: u64,
        end: u64,
    },
    MissingPath {
        role: &'static str,
        path: PathBuf,
    },
    MissingDerivedPath {
        role: &'static str,
        unit: KeyUnitKind,
    },
    FixedByteCountMismatch {
        kind: KeyUnitKind,
        path: PathBuf,
        expected: usize,
        found: u64,
    },
    ConstantTreeRootMismatch {
        kind: KeyUnitKind,
        expected: VerificationKeyRoot,
        found: VerificationKeyRoot,
    },
    PcsPlanMismatch {
        kind: KeyUnitKind,
        path: PathBuf,
    },
    PcsMaterialMismatch {
        kind: KeyUnitKind,
        path: PathBuf,
    },
    RegularHintsMismatch {
        kind: KeyUnitKind,
        path: PathBuf,
    },
    Digest {
        message: String,
    },
    Io {
        role: &'static str,
        message: String,
    },
}

impl fmt::Display for KeyUnitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::Compressor => write!(f, "compressor"),
            Self::RecursiveFirst => write!(f, "recursive-first"),
            Self::RecursiveSecond => write!(f, "recursive-second"),
            Self::FinalAggregation => write!(f, "final-aggregation"),
            Self::FinalCircuit => write!(f, "final-circuit"),
        }
    }
}

impl fmt::Display for KeyDirectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GlobalInfo(error) => write!(f, "key-directory global metadata error: {error}"),
            Self::GlobalConstraints(error) => {
                write!(f, "key-directory global constraint program error: {error}")
            }
            Self::GlobalHints(error) => {
                write!(f, "key-directory global hint program error: {error}")
            }
            Self::RegularConstraints(error) => {
                write!(f, "key-directory regular constraint program error: {error}")
            }
            Self::RegularHints(error) => {
                write!(f, "key-directory regular hint program error: {error}")
            }
            Self::ConstantTree(error) => {
                write!(f, "key-directory constant-tree error: {error}")
            }
            Self::Metadata(error) => write!(f, "key-directory unit metadata error: {error}"),
            Self::PcsPlan(error) => write!(f, "key-directory PCS setup plan error: {error}"),
            Self::PcsMaterial(error) => {
                write!(f, "key-directory PCS setup material error: {error}")
            }
            Self::ExpressionProgram(error) => {
                write!(f, "key-directory expression program error: {error}")
            }
            Self::VerificationKey(error) => {
                write!(f, "key-directory verification-key error: {error}")
            }
            Self::FixedColumns(error) => write!(f, "key-directory fixed-column error: {error}"),
            Self::SourceFixedFileManifest(error) => {
                write!(f, "key-directory source fixed-file manifest error: {error}")
            }
            Self::SourceProgramArchive(error) => {
                write!(f, "key-directory source program archive error: {error}")
            }
            Self::SourceFixedFileManifestGroupMismatch {
                entry_index,
                group_id,
                group_name,
            } => write!(
                f,
                "key-directory source fixed-file manifest entry {entry_index} references group {group_id}:{group_name} outside setup layout"
            ),
            Self::SourceFixedFileManifestUnitMismatch {
                entry_index,
                group_id,
                unit_id,
                unit_name,
            } => write!(
                f,
                "key-directory source fixed-file manifest entry {entry_index} references unit {group_id}:{unit_id}:{unit_name} outside setup layout"
            ),
            Self::SourceFixedFileManifestSourceMismatch {
                entry_index,
                source_name,
            } => write!(
                f,
                "key-directory source fixed-file manifest entry {entry_index} references source {source_name} outside source program archive"
            ),
            Self::SourceFixedFileManifestSpanMismatch {
                entry_index,
                source_name,
                start,
                end,
                source_len,
            } => write!(
                f,
                "key-directory source fixed-file manifest entry {entry_index} span {start}..{end} exceeds source {source_name} length {source_len}"
            ),
            Self::SourceFixedFileManifestUtf8BoundaryMismatch {
                entry_index,
                source_name,
                start,
                end,
            } => write!(
                f,
                "key-directory source fixed-file manifest entry {entry_index} span {start}..{end} is not on UTF-8 boundary for source {source_name}"
            ),
            Self::MissingPath { role, path } => {
                write!(f, "missing key-directory {role}: {}", path.display())
            }
            Self::MissingDerivedPath { role, unit } => {
                write!(f, "missing derived key-directory {role} for {unit}")
            }
            Self::FixedByteCountMismatch {
                kind,
                path,
                expected,
                found,
            } => write!(
                f,
                "key-directory fixed-column byte count mismatch for {kind} at {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::ConstantTreeRootMismatch { kind, .. } => {
                write!(f, "key-directory constant-tree root mismatch for {kind}")
            }
            Self::PcsPlanMismatch { kind, path } => write!(
                f,
                "key-directory PCS setup plan mismatch for {kind} at {}",
                path.display()
            ),
            Self::PcsMaterialMismatch { kind, path } => write!(
                f,
                "key-directory PCS setup material mismatch for {kind} at {}",
                path.display()
            ),
            Self::RegularHintsMismatch { kind, path } => write!(
                f,
                "key-directory regular hint program mismatch for {kind} at {}",
                path.display()
            ),
            Self::Digest { message } => write!(f, "key-directory digest error: {message}"),
            Self::Io { role, message } => write!(f, "key-directory {role} io error: {message}"),
        }
    }
}

impl std::error::Error for KeyDirectoryError {}

impl From<GlobalInfoError> for KeyDirectoryError {
    fn from(error: GlobalInfoError) -> Self {
        Self::GlobalInfo(error)
    }
}

impl From<ConstraintProgramError> for KeyDirectoryError {
    fn from(error: ConstraintProgramError) -> Self {
        Self::GlobalConstraints(error)
    }
}

impl From<HintProgramError> for KeyDirectoryError {
    fn from(error: HintProgramError) -> Self {
        Self::RegularHints(error)
    }
}

impl From<ConstantTreeError> for KeyDirectoryError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

impl From<MetadataBundleError> for KeyDirectoryError {
    fn from(error: MetadataBundleError) -> Self {
        Self::Metadata(error)
    }
}

impl From<PcsPlanError> for KeyDirectoryError {
    fn from(error: PcsPlanError) -> Self {
        Self::PcsPlan(error)
    }
}

impl From<PcsSetupMaterialError> for KeyDirectoryError {
    fn from(error: PcsSetupMaterialError) -> Self {
        Self::PcsMaterial(error)
    }
}

impl From<ExpressionProgramError> for KeyDirectoryError {
    fn from(error: ExpressionProgramError) -> Self {
        Self::ExpressionProgram(error)
    }
}

impl From<VerificationKeyError> for KeyDirectoryError {
    fn from(error: VerificationKeyError) -> Self {
        Self::VerificationKey(error)
    }
}

impl From<FixedColumnError> for KeyDirectoryError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<SourceFixedFileManifestError> for KeyDirectoryError {
    fn from(error: SourceFixedFileManifestError) -> Self {
        Self::SourceFixedFileManifest(error)
    }
}

impl From<SourceProgramArchiveError> for KeyDirectoryError {
    fn from(error: SourceProgramArchiveError) -> Self {
        Self::SourceProgramArchive(error)
    }
}

impl KeyDirectoryLayout {
    pub fn required_paths(&self) -> Vec<RequiredPath> {
        let mut paths = vec![
            RequiredPath {
                role: "global metadata",
                path: self.global_paths.info.clone(),
            },
            RequiredPath {
                role: "global constraints program",
                path: self.global_paths.constraints_program.clone(),
            },
        ];

        for unit in &self.units {
            paths.extend(unit.required_paths());
        }
        paths
    }
}

impl KeyUnitPaths {
    pub fn setup_info(&self) -> Option<PathBuf> {
        self.setup_info_binary()
    }

    pub fn setup_info_binary(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".starkinfo.bin"))
    }

    pub fn expression_info(&self) -> Option<PathBuf> {
        self.expression_info_binary()
    }

    pub fn expression_info_binary(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".expressionsinfo.bin"))
    }

    pub fn verifier_info(&self) -> Option<PathBuf> {
        self.verifier_info_binary()
    }

    pub fn verifier_info_binary(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".verifierinfo.bin"))
    }

    pub fn pcs_setup_plan(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|_| append_suffix(&self.prefix, ".pcs-plan"))
    }

    pub fn pcs_setup_material(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|_| append_suffix(&self.prefix, ".pcs-material"))
    }

    pub fn expression_program(&self) -> Option<PathBuf> {
        self.program_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".bin"))
    }

    pub fn verifier_program(&self) -> Option<PathBuf> {
        self.program_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".verifier.bin"))
    }

    pub fn verification_key_binary(&self) -> PathBuf {
        append_suffix(&self.verification_key_prefix, ".verkey.bin")
    }

    pub fn required_paths(&self) -> Vec<RequiredPath> {
        let mut paths = Vec::new();
        if let Some(path) = self.setup_info() {
            paths.push(RequiredPath {
                role: "unit setup metadata",
                path,
            });
        }
        if let Some(path) = self.expression_info() {
            paths.push(RequiredPath {
                role: "unit expression metadata",
                path,
            });
        }
        if let Some(path) = self.verifier_info() {
            paths.push(RequiredPath {
                role: "unit verifier metadata",
                path,
            });
        }
        if let Some(path) = self.expression_program() {
            paths.push(RequiredPath {
                role: "unit expression program",
                path,
            });
        }
        if let Some(path) = self.verifier_program() {
            paths.push(RequiredPath {
                role: "unit verifier program",
                path,
            });
        }
        paths.push(RequiredPath {
            role: "unit verification-key binary",
            path: self.verification_key_binary(),
        });
        paths.push(RequiredPath {
            role: "unit fixed columns",
            path: self.fixed_columns.clone(),
        });
        paths
    }
}

pub fn read_key_directory_layout(
    root: impl AsRef<Path>,
) -> Result<KeyDirectoryLayout, KeyDirectoryError> {
    let root = root.as_ref().to_path_buf();
    let global_paths = GlobalKeyPaths::from_root(&root);
    let global_info = read_global_info_path(&global_paths)?;
    let units = derive_unit_paths(&root, &global_info)?;

    Ok(KeyDirectoryLayout {
        source_fixed_file_manifest: root.join(SOURCE_FIXED_FILE_MANIFEST_FILE),
        source_program_archive: root.join(SOURCE_PROGRAM_ARCHIVE_FILE),
        root,
        global_info,
        global_paths,
        units,
    })
}

pub fn validate_key_directory_layout(layout: &KeyDirectoryLayout) -> Result<(), KeyDirectoryError> {
    let mut seen = BTreeSet::new();
    for required in layout.required_paths() {
        if !seen.insert(required.path.clone()) {
            continue;
        }
        if !required.path.is_file() {
            return Err(KeyDirectoryError::MissingPath {
                role: required.role,
                path: required.path,
            });
        }
    }
    Ok(())
}

pub fn read_key_directory_catalog(
    root: impl AsRef<Path>,
) -> Result<KeyDirectoryCatalog, KeyDirectoryError> {
    let layout = read_key_directory_layout(root)?;
    read_key_directory_catalog_from_layout(&layout)
}

pub fn read_key_directory_catalog_trusting_pcs_material_digests(
    root: impl AsRef<Path>,
) -> Result<KeyDirectoryCatalog, KeyDirectoryError> {
    let layout = read_key_directory_layout(root)?;
    read_key_directory_catalog_from_layout_with_material_digest_check(
        &layout,
        PcsMaterialDigestCheck::TrustStored,
    )
}

pub fn read_key_directory_catalog_from_layout(
    layout: &KeyDirectoryLayout,
) -> Result<KeyDirectoryCatalog, KeyDirectoryError> {
    read_key_directory_catalog_from_layout_with_material_digest_check(
        layout,
        PcsMaterialDigestCheck::Recompute,
    )
}

fn read_key_directory_catalog_from_layout_with_material_digest_check(
    layout: &KeyDirectoryLayout,
    digest_check: PcsMaterialDigestCheck,
) -> Result<KeyDirectoryCatalog, KeyDirectoryError> {
    validate_key_directory_layout(layout)?;
    let global_constraints =
        read_global_constraint_program_file(&layout.global_paths.constraints_program)
            .map_err(KeyDirectoryError::GlobalConstraints)?;
    let global_hints = read_global_hint_program_file(&layout.global_paths.constraints_program)
        .map_err(KeyDirectoryError::GlobalHints)?;
    let source_fixed_file_manifest =
        read_source_fixed_file_manifest_if_present(&layout.source_fixed_file_manifest)?;
    if let Some(manifest) = source_fixed_file_manifest.as_ref() {
        validate_source_fixed_file_manifest_layout(&layout.global_info, manifest)?;
    }
    let source_program_archive =
        read_source_program_archive_if_present(&layout.source_program_archive)?;
    if let (Some(manifest), Some(archive)) = (
        source_fixed_file_manifest.as_ref(),
        source_program_archive.as_ref(),
    ) {
        validate_source_fixed_file_manifest_archive(manifest, archive)?;
    }
    let mut units = Vec::with_capacity(layout.units.len());
    for unit in &layout.units {
        units.push(read_key_unit_catalog_entry(unit, digest_check)?);
    }

    Ok(KeyDirectoryCatalog {
        layout: layout.clone(),
        global_constraints,
        global_hints,
        source_fixed_file_manifest,
        source_program_archive,
        units,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PcsMaterialDigestCheck {
    Recompute,
    TrustStored,
}

impl GlobalKeyPaths {
    pub fn from_root(root: &Path) -> Self {
        Self {
            info: root.join(GLOBAL_INFO_BIN_FILE),
            constraints_program: root.join(GLOBAL_CONSTRAINTS_BIN_FILE),
        }
    }
}

fn read_global_info_path(paths: &GlobalKeyPaths) -> Result<GlobalInfo, KeyDirectoryError> {
    read_global_info_binary_file(&paths.info).map_err(KeyDirectoryError::GlobalInfo)
}

fn optional_path_exists(path: &Path, role: &'static str) -> Result<bool, KeyDirectoryError> {
    path.try_exists().map_err(|error| KeyDirectoryError::Io {
        role,
        message: error.to_string(),
    })
}

fn read_source_fixed_file_manifest_if_present(
    path: &Path,
) -> Result<Option<SourceFixedFileManifest>, KeyDirectoryError> {
    if !optional_path_exists(path, "source fixed-file manifest")? {
        return Ok(None);
    }
    read_source_fixed_file_manifest_file(path)
        .map(Some)
        .map_err(KeyDirectoryError::SourceFixedFileManifest)
}

fn read_source_program_archive_if_present(
    path: &Path,
) -> Result<Option<SourceProgramArchive>, KeyDirectoryError> {
    if !optional_path_exists(path, "source program archive")? {
        return Ok(None);
    }
    read_source_program_archive_file(path)
        .map(Some)
        .map_err(KeyDirectoryError::SourceProgramArchive)
}

fn validate_source_fixed_file_manifest_layout(
    global_info: &GlobalInfo,
    manifest: &SourceFixedFileManifest,
) -> Result<(), KeyDirectoryError> {
    for (entry_index, entry) in manifest.entries.iter().enumerate() {
        let Some(group_name) = usize::try_from(entry.group_id)
            .ok()
            .and_then(|group_id| global_info.air_groups.get(group_id))
        else {
            return Err(KeyDirectoryError::SourceFixedFileManifestGroupMismatch {
                entry_index,
                group_id: entry.group_id,
                group_name: entry.group_name.clone(),
            });
        };
        if group_name != &entry.group_name {
            return Err(KeyDirectoryError::SourceFixedFileManifestGroupMismatch {
                entry_index,
                group_id: entry.group_id,
                group_name: entry.group_name.clone(),
            });
        }
        if entry.virtual_instance {
            continue;
        }
        let Some(unit) = usize::try_from(entry.group_id)
            .ok()
            .and_then(|group_id| global_info.airs.get(group_id))
            .and_then(|group| {
                usize::try_from(entry.unit_id)
                    .ok()
                    .and_then(|unit_id| group.get(unit_id))
            })
        else {
            return Err(KeyDirectoryError::SourceFixedFileManifestUnitMismatch {
                entry_index,
                group_id: entry.group_id,
                unit_id: entry.unit_id,
                unit_name: entry.unit_name.clone(),
            });
        };
        if unit.name != entry.unit_name {
            return Err(KeyDirectoryError::SourceFixedFileManifestUnitMismatch {
                entry_index,
                group_id: entry.group_id,
                unit_id: entry.unit_id,
                unit_name: entry.unit_name.clone(),
            });
        }
    }
    Ok(())
}

fn validate_source_fixed_file_manifest_archive(
    manifest: &SourceFixedFileManifest,
    archive: &SourceProgramArchive,
) -> Result<(), KeyDirectoryError> {
    let sources: BTreeMap<&str, &str> = archive
        .sources
        .iter()
        .map(|source| (source.source_name.as_str(), source.contents.as_str()))
        .collect();
    for (entry_index, entry) in manifest.entries.iter().enumerate() {
        let Some(source) = sources.get(entry.source_name.as_str()).copied() else {
            return Err(KeyDirectoryError::SourceFixedFileManifestSourceMismatch {
                entry_index,
                source_name: entry.source_name.clone(),
            });
        };
        let source_len = source.len();
        if entry.end > source_len as u64 {
            return Err(KeyDirectoryError::SourceFixedFileManifestSpanMismatch {
                entry_index,
                source_name: entry.source_name.clone(),
                start: entry.start,
                end: entry.end,
                source_len,
            });
        }
        if !source.is_char_boundary(entry.start as usize)
            || !source.is_char_boundary(entry.end as usize)
        {
            return Err(
                KeyDirectoryError::SourceFixedFileManifestUtf8BoundaryMismatch {
                    entry_index,
                    source_name: entry.source_name.clone(),
                    start: entry.start,
                    end: entry.end,
                },
            );
        }
    }
    Ok(())
}

fn read_key_unit_catalog_entry(
    paths: &KeyUnitPaths,
    digest_check: PcsMaterialDigestCheck,
) -> Result<KeyUnitCatalogEntry, KeyDirectoryError> {
    let metadata_paths = UnitMetadataPaths::new(
        paths
            .setup_info()
            .ok_or(KeyDirectoryError::MissingDerivedPath {
                role: "setup metadata",
                unit: paths.kind,
            })?,
        paths
            .expression_info()
            .ok_or(KeyDirectoryError::MissingDerivedPath {
                role: "expression metadata",
                unit: paths.kind,
            })?,
        paths
            .verifier_info()
            .ok_or(KeyDirectoryError::MissingDerivedPath {
                role: "verifier metadata",
                unit: paths.kind,
            })?,
    );
    let metadata = read_unit_metadata_bundle(&metadata_paths)?;
    let pcs_plan = derive_pcs_setup_plan(&metadata.setup)?;
    validate_pcs_setup_plan_companion(paths, &pcs_plan)?;

    let binary_root = read_verification_key_binary_file(paths.verification_key_binary())?;
    let verification_key = binary_root;

    let expression_program_path =
        paths
            .expression_program()
            .ok_or(KeyDirectoryError::MissingDerivedPath {
                role: "expression program",
                unit: paths.kind,
            })?;
    let expression_program = read_expression_program_file(&expression_program_path)?;
    let regular_constraints = read_regular_constraint_program_file(&expression_program_path)
        .map_err(KeyDirectoryError::RegularConstraints)?;
    let regular_hints = read_regular_hint_program_file(&expression_program_path)
        .map_err(KeyDirectoryError::RegularHints)?;
    let expected_regular_hints = regular_hint_program_from_expression_info(&metadata.expressions)?;
    if regular_hints != expected_regular_hints {
        return Err(KeyDirectoryError::RegularHintsMismatch {
            kind: paths.kind,
            path: expression_program_path,
        });
    }
    let verifier_program = read_expression_program_file(paths.verifier_program().ok_or(
        KeyDirectoryError::MissingDerivedPath {
            role: "verifier program",
            unit: paths.kind,
        },
    )?)?;

    let expected_fixed_bytes = expected_raw_fixed_column_byte_count(&metadata.setup)?;
    let actual_fixed_bytes = std::fs::metadata(&paths.fixed_columns)
        .map_err(|error| KeyDirectoryError::Io {
            role: "fixed-column metadata",
            message: error.to_string(),
        })?
        .len();
    if actual_fixed_bytes
        != u64::try_from(expected_fixed_bytes)
            .map_err(|_| KeyDirectoryError::FixedColumns(FixedColumnError::LengthOverflow))?
    {
        return Err(KeyDirectoryError::FixedByteCountMismatch {
            kind: paths.kind,
            path: paths.fixed_columns.clone(),
            expected: expected_fixed_bytes,
            found: actual_fixed_bytes,
        });
    }

    if digest_check == PcsMaterialDigestCheck::TrustStored {
        let material_path_present = paths
            .pcs_setup_material()
            .map(|path| optional_path_exists(&path, "PCS setup material"))
            .transpose()?
            .unwrap_or(false);
        if material_path_present {
            let constant_tree_present =
                optional_path_exists(&paths.constant_tree, "constant tree")?;
            let constant_tree_bytes = if constant_tree_present {
                Some(
                    std::fs::metadata(&paths.constant_tree)
                        .map_err(|error| KeyDirectoryError::Io {
                            role: "constant tree metadata",
                            message: error.to_string(),
                        })?
                        .len(),
                )
            } else {
                None
            };
            let (pcs_material_present, pcs_material_bytes, pcs_material) =
                read_pcs_setup_material_companion_trusting_digests(
                    paths,
                    &metadata.setup,
                    &pcs_plan,
                    &verification_key,
                    constant_tree_present,
                    constant_tree_bytes,
                    actual_fixed_bytes,
                )?;
            let constant_tree_root = pcs_material.as_ref().map(|material| {
                VerificationKeyRoot::FieldElements(material.constant_tree_root.to_vec())
            });
            return Ok(KeyUnitCatalogEntry {
                paths: paths.clone(),
                metadata,
                pcs_plan,
                verification_key,
                expression_program,
                regular_constraints,
                regular_hints,
                verifier_program,
                expected_fixed_bytes,
                actual_fixed_bytes,
                constant_tree_present,
                constant_tree_bytes,
                constant_tree_root,
                pcs_material_present,
                pcs_material_bytes,
                pcs_material,
            });
        }
    }

    let constant_tree_summary = if optional_path_exists(&paths.constant_tree, "constant tree")? {
        let summary = summarize_constant_tree_file(&paths.constant_tree, &metadata.setup)?;
        if summary.root != verification_key {
            return Err(KeyDirectoryError::ConstantTreeRootMismatch {
                kind: paths.kind,
                expected: verification_key.clone(),
                found: summary.root,
            });
        }
        Some(summary)
    } else {
        None
    };
    let (constant_tree_present, constant_tree_bytes, constant_tree_root) =
        if let Some(summary) = constant_tree_summary.as_ref() {
            (true, Some(summary.byte_count), Some(summary.root.clone()))
        } else {
            (false, None, None)
        };
    let (pcs_material_present, pcs_material_bytes, pcs_material) =
        read_pcs_setup_material_companion(
            paths,
            &pcs_plan,
            constant_tree_summary.as_ref(),
            actual_fixed_bytes,
        )?;

    Ok(KeyUnitCatalogEntry {
        paths: paths.clone(),
        metadata,
        pcs_plan,
        verification_key,
        expression_program,
        regular_constraints,
        regular_hints,
        verifier_program,
        expected_fixed_bytes,
        actual_fixed_bytes,
        constant_tree_present,
        constant_tree_bytes,
        constant_tree_root,
        pcs_material_present,
        pcs_material_bytes,
        pcs_material,
    })
}

fn validate_pcs_setup_plan_companion(
    paths: &KeyUnitPaths,
    expected: &PcsSetupPlan,
) -> Result<(), KeyDirectoryError> {
    let Some(path) = paths.pcs_setup_plan() else {
        return Ok(());
    };
    if !optional_path_exists(&path, "PCS setup plan")? {
        return Ok(());
    }
    let found = read_pcs_setup_plan_file(&path)?;
    if &found != expected {
        return Err(KeyDirectoryError::PcsPlanMismatch {
            kind: paths.kind,
            path,
        });
    }
    Ok(())
}

fn read_pcs_setup_material_companion(
    paths: &KeyUnitPaths,
    plan: &PcsSetupPlan,
    constant_tree: Option<&ConstantTreeFileSummary>,
    fixed_byte_count: u64,
) -> Result<(bool, Option<u64>, Option<PcsSetupMaterial>), KeyDirectoryError> {
    let Some(path) = paths.pcs_setup_material() else {
        return Ok((false, None, None));
    };
    if !optional_path_exists(&path, "PCS setup material")? {
        return Ok((false, None, None));
    }

    let found = read_pcs_setup_material_file(&path)?;
    let Some(constant_tree) = constant_tree else {
        return Err(KeyDirectoryError::PcsMaterialMismatch {
            kind: paths.kind,
            path,
        });
    };
    let plan_digest: [u8; 32] = Sha256::digest(encode_pcs_setup_plan(plan)?).into();
    let fixed_digest = sha256_file(&paths.fixed_columns, "fixed-column material input")?;
    let root_matches = match &constant_tree.root {
        VerificationKeyRoot::FieldElements(values) => {
            values.as_slice() == found.constant_tree_root.as_slice()
        }
    };
    if found.plan_digest != plan_digest
        || found.fixed_column_digest != fixed_digest
        || found.constant_tree_digest != constant_tree.digest
        || !root_matches
        || found.fixed_byte_count != fixed_byte_count
        || found.constant_tree_byte_count != constant_tree.byte_count
        || found.leaf_byte_count
            != u64::try_from(constant_tree.leaf_byte_count)
                .map_err(|_| KeyDirectoryError::ConstantTree(ConstantTreeError::LengthOverflow))?
        || found.node_byte_count
            != u64::try_from(constant_tree.node_byte_count)
                .map_err(|_| KeyDirectoryError::ConstantTree(ConstantTreeError::LengthOverflow))?
    {
        return Err(KeyDirectoryError::PcsMaterialMismatch {
            kind: paths.kind,
            path,
        });
    }
    let bytes = std::fs::metadata(&path)
        .map_err(|error| KeyDirectoryError::Io {
            role: "PCS setup material metadata",
            message: error.to_string(),
        })?
        .len();
    Ok((true, Some(bytes), Some(found)))
}

fn read_pcs_setup_material_companion_trusting_digests(
    paths: &KeyUnitPaths,
    setup: &crate::setup_info::UnitSetupInfo,
    plan: &PcsSetupPlan,
    verification_key: &VerificationKeyRoot,
    constant_tree_present: bool,
    constant_tree_bytes: Option<u64>,
    fixed_byte_count: u64,
) -> Result<(bool, Option<u64>, Option<PcsSetupMaterial>), KeyDirectoryError> {
    let path = paths
        .pcs_setup_material()
        .ok_or(KeyDirectoryError::MissingDerivedPath {
            role: "PCS setup material",
            unit: paths.kind,
        })?;
    let found = read_pcs_setup_material_file(&path)?;
    if !constant_tree_present {
        return Err(KeyDirectoryError::PcsMaterialMismatch {
            kind: paths.kind,
            path,
        });
    }
    let plan_digest: [u8; 32] = Sha256::digest(encode_pcs_setup_plan(plan)?).into();
    let (expected_leaf_byte_count, expected_node_byte_count) =
        expected_constant_tree_leaf_node_byte_counts(setup).map_err(KeyDirectoryError::from)?;
    let expected_leaf_byte_count = u64::try_from(expected_leaf_byte_count)
        .map_err(|_| KeyDirectoryError::ConstantTree(ConstantTreeError::LengthOverflow))?;
    let expected_node_byte_count = u64::try_from(expected_node_byte_count)
        .map_err(|_| KeyDirectoryError::ConstantTree(ConstantTreeError::LengthOverflow))?;
    let root_matches = match verification_key {
        VerificationKeyRoot::FieldElements(values) => {
            values.as_slice() == found.constant_tree_root.as_slice()
        }
    };
    if found.plan_digest != plan_digest
        || !root_matches
        || found.fixed_byte_count != fixed_byte_count
        || Some(found.constant_tree_byte_count) != constant_tree_bytes
        || found.leaf_byte_count != expected_leaf_byte_count
        || found.node_byte_count != expected_node_byte_count
    {
        return Err(KeyDirectoryError::PcsMaterialMismatch {
            kind: paths.kind,
            path,
        });
    }
    let bytes = std::fs::metadata(&path)
        .map_err(|error| KeyDirectoryError::Io {
            role: "PCS setup material metadata",
            message: error.to_string(),
        })?
        .len();
    Ok((true, Some(bytes), Some(found)))
}

fn sha256_file(path: &Path, role: &'static str) -> Result<[u8; 32], KeyDirectoryError> {
    let mut file = File::open(path).map_err(|error| KeyDirectoryError::Io {
        role,
        message: error.to_string(),
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| KeyDirectoryError::Io {
                role,
                message: error.to_string(),
            })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().into())
}

fn derive_unit_paths(
    root: &Path,
    global_info: &GlobalInfo,
) -> Result<Vec<KeyUnitPaths>, KeyDirectoryError> {
    let mut units = Vec::new();
    let program_root = root.join(&global_info.name);

    for (group_id, group) in global_info.airs.iter().enumerate() {
        let group_name = &global_info.air_groups[group_id];
        let group_root = program_root.join(group_name);
        let recursive_second_prefix = group_root.join("recursive2").join("recursive2");

        for (unit_id, unit) in group.iter().enumerate() {
            let unit_root = group_root.join("airs").join(&unit.name);
            let basic_prefix = unit_root.join("air").join(&unit.name);
            units.push(KeyUnitPaths::from_prefix(KeyUnitPathSpec {
                kind: KeyUnitKind::Basic,
                group_id: Some(group_id),
                unit_id: Some(unit_id),
                group_name: Some(group_name.clone()),
                unit_name: Some(unit.name.clone()),
                prefix: basic_prefix.clone(),
                metadata_prefix: Some(basic_prefix.clone()),
                program_prefix: Some(basic_prefix.clone()),
                verification_key_prefix: basic_prefix,
            }));

            if unit.has_compressor {
                let compressor_prefix = unit_root.join("compressor").join("compressor");
                units.push(KeyUnitPaths::from_prefix(KeyUnitPathSpec {
                    kind: KeyUnitKind::Compressor,
                    group_id: Some(group_id),
                    unit_id: Some(unit_id),
                    group_name: Some(group_name.clone()),
                    unit_name: Some(unit.name.clone()),
                    prefix: compressor_prefix.clone(),
                    metadata_prefix: Some(compressor_prefix.clone()),
                    program_prefix: Some(compressor_prefix.clone()),
                    verification_key_prefix: compressor_prefix,
                }));
            }

            let recursive_first_prefix = unit_root.join("recursive1").join("recursive1");
            units.push(KeyUnitPaths::from_prefix(KeyUnitPathSpec {
                kind: KeyUnitKind::RecursiveFirst,
                group_id: Some(group_id),
                unit_id: Some(unit_id),
                group_name: Some(group_name.clone()),
                unit_name: Some(unit.name.clone()),
                prefix: recursive_first_prefix.clone(),
                metadata_prefix: Some(recursive_second_prefix.clone()),
                program_prefix: Some(recursive_second_prefix.clone()),
                verification_key_prefix: recursive_first_prefix,
            }));
        }

        units.push(KeyUnitPaths::from_prefix(KeyUnitPathSpec {
            kind: KeyUnitKind::RecursiveSecond,
            group_id: Some(group_id),
            unit_id: None,
            group_name: Some(group_name.clone()),
            unit_name: None,
            prefix: recursive_second_prefix.clone(),
            metadata_prefix: Some(recursive_second_prefix.clone()),
            program_prefix: Some(recursive_second_prefix.clone()),
            verification_key_prefix: recursive_second_prefix,
        }));
    }

    let final_prefix = program_root.join("vadcop_final").join("vadcop_final");
    units.push(KeyUnitPaths::from_prefix(KeyUnitPathSpec {
        kind: KeyUnitKind::FinalAggregation,
        group_id: None,
        unit_id: None,
        group_name: None,
        unit_name: None,
        prefix: final_prefix.clone(),
        metadata_prefix: Some(final_prefix.clone()),
        program_prefix: Some(final_prefix.clone()),
        verification_key_prefix: final_prefix,
    }));

    let final_circuit_prefix = program_root.join("recursivef").join("recursivef");
    let final_circuit_setup = append_suffix(&final_circuit_prefix, ".starkinfo.bin");
    if optional_path_exists(&final_circuit_setup, "final circuit setup metadata")? {
        units.push(KeyUnitPaths::from_prefix(KeyUnitPathSpec {
            kind: KeyUnitKind::FinalCircuit,
            group_id: None,
            unit_id: None,
            group_name: None,
            unit_name: None,
            prefix: final_circuit_prefix.clone(),
            metadata_prefix: Some(final_circuit_prefix.clone()),
            program_prefix: Some(final_circuit_prefix.clone()),
            verification_key_prefix: final_circuit_prefix,
        }));
    }

    Ok(units)
}

struct KeyUnitPathSpec {
    kind: KeyUnitKind,
    group_id: Option<usize>,
    unit_id: Option<usize>,
    group_name: Option<String>,
    unit_name: Option<String>,
    prefix: PathBuf,
    metadata_prefix: Option<PathBuf>,
    program_prefix: Option<PathBuf>,
    verification_key_prefix: PathBuf,
}

impl KeyUnitPaths {
    fn from_prefix(spec: KeyUnitPathSpec) -> Self {
        Self {
            kind: spec.kind,
            group_id: spec.group_id,
            unit_id: spec.unit_id,
            group_name: spec.group_name,
            unit_name: spec.unit_name,
            fixed_columns: append_suffix(&spec.prefix, ".const"),
            constant_tree: append_suffix(&spec.prefix, ".consttree"),
            prefix: spec.prefix,
            metadata_prefix: spec.metadata_prefix,
            program_prefix: spec.program_prefix,
            verification_key_prefix: spec.verification_key_prefix,
        }
    }
}

fn append_suffix(prefix: &Path, suffix: &str) -> PathBuf {
    let mut value = prefix.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}
