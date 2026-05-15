use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::{
    parse_constant_tree_bytes, read_constant_tree_file, ConstantTreeError,
};
use lzvm_artifacts::expression_info::{
    encode_expression_info, read_expression_info_binary_file, ExpressionInfo, ExpressionInfoError,
};
use lzvm_artifacts::fixed::{
    encode_raw_fixed_columns, read_fixed_columns_file, read_fixed_columns_file_for_setup,
    read_raw_fixed_column_layout_file, write_raw_fixed_columns_file, FixedColumnError,
    FixedColumns,
};
use lzvm_artifacts::global_info::{encode_global_info, GlobalInfo, GlobalInfoError};
use lzvm_artifacts::hint_program::{
    encode_regular_hint_program, regular_hint_program_from_expression_info, HintProgramError,
};
use lzvm_artifacts::key_directory::{
    read_key_directory_layout, validate_key_directory_layout, KeyDirectoryError, KeyDirectoryLayout,
};
use lzvm_artifacts::pcs_material::{
    build_pcs_setup_material, encode_pcs_setup_material, PcsSetupMaterialError,
};
use lzvm_artifacts::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, read_pcs_setup_plan_file, PcsPlanError,
};
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, read_unit_setup_info_binary_file, SetupInfoError, UnitSetupInfo,
};
use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, read_verification_key_binary_file, VerificationKeyError,
    VerificationKeyRoot,
};
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, read_verifier_info_binary_file, VerifierInfo, VerifierInfoError,
};
use lzvm_field::{
    coset_extend_evaluations, poseidon2_hash_16, poseidon2_hash_8, DomainError, Felt, FieldError,
};

const WORD_BYTES: usize = 8;
const HASH_WORDS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstantTreeShape {
    arity: usize,
    row_count: usize,
    column_count: usize,
    expected_tree_len: usize,
}

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
pub struct PcsDirectoryWriteReport {
    pub unit_count: usize,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFileWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedExtensionBackend {
    Cpu,
    Cuda,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    FixedColumns(FixedColumnError),
    ConstantTree(ConstantTreeError),
    VerificationKey(VerificationKeyError),
    Domain(DomainError),
    Field(FieldError),
    CudaUnavailable,
    CudaBackend(String),
    ConstantTreeRootMismatch {
        expected: VerificationKeyRoot,
        found: VerificationKeyRoot,
    },
    InvalidConstantTreeLeafByteLength {
        expected: usize,
        found: usize,
    },
    UnsupportedConstantTreeArity {
        arity: u32,
    },
    UnsupportedConstantTreeHash {
        hash_type: Option<String>,
    },
    LengthOverflow,
    MissingParent {
        path: PathBuf,
    },
    Io {
        role: &'static str,
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FixedColumns(error) => write!(f, "setup fixed-column error: {error}"),
            Self::ConstantTree(error) => write!(f, "setup constant-tree error: {error}"),
            Self::VerificationKey(error) => write!(f, "setup verification-key error: {error}"),
            Self::Domain(error) => write!(f, "setup field-domain error: {error}"),
            Self::Field(error) => write!(f, "setup field error: {error}"),
            Self::CudaUnavailable => write!(f, "setup cuda backend is not enabled"),
            Self::CudaBackend(message) => write!(f, "setup cuda backend error: {message}"),
            Self::ConstantTreeRootMismatch { expected, found } => write!(
                f,
                "setup constant-tree root mismatch: expected {expected:?}, found {found:?}"
            ),
            Self::InvalidConstantTreeLeafByteLength { expected, found } => write!(
                f,
                "invalid constant-tree leaf byte length: expected {expected}, found {found}"
            ),
            Self::UnsupportedConstantTreeArity { arity } => {
                write!(f, "unsupported native constant-tree arity: {arity}")
            }
            Self::UnsupportedConstantTreeHash { hash_type } => {
                write!(f, "unsupported native constant-tree hash: {hash_type:?}")
            }
            Self::LengthOverflow => write!(f, "setup length overflow"),
            Self::MissingParent { path } => {
                write!(f, "setup output path has no parent: {}", path.display())
            }
            Self::Io {
                role,
                path,
                message,
            } => write!(f, "setup {role} io error at {}: {message}", path.display()),
        }
    }
}

impl std::error::Error for SetupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFileWriteError {
    SetupInfo(SetupInfoError),
    FixedColumns(FixedColumnError),
    Setup(SetupError),
}

impl fmt::Display for NativeFileWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::FixedColumns(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for NativeFileWriteError {}

impl From<SetupInfoError> for NativeFileWriteError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<FixedColumnError> for NativeFileWriteError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<SetupError> for NativeFileWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseDirectoryWriteError {
    KeyDirectory(KeyDirectoryError),
    GlobalInfo(GlobalInfoError),
    SetupInfo(SetupInfoError),
    ExpressionInfo(ExpressionInfoError),
    VerifierInfo(VerifierInfoError),
    FixedColumns(FixedColumnError),
    HintProgram(HintProgramError),
    VerificationKey(VerificationKeyError),
    Setup(SetupError),
    MissingUnitPath { role: &'static str },
    Message { message: String },
}

impl BaseDirectoryWriteError {
    fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

impl fmt::Display for BaseDirectoryWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDirectory(error) => write!(f, "{error}"),
            Self::GlobalInfo(error) => write!(f, "{error}"),
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::ExpressionInfo(error) => write!(f, "{error}"),
            Self::VerifierInfo(error) => write!(f, "{error}"),
            Self::FixedColumns(error) => write!(f, "{error}"),
            Self::HintProgram(error) => write!(f, "{error}"),
            Self::VerificationKey(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
            Self::MissingUnitPath { role } => write!(f, "missing unit {role}"),
            Self::Message { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for BaseDirectoryWriteError {}

impl From<KeyDirectoryError> for BaseDirectoryWriteError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

impl From<GlobalInfoError> for BaseDirectoryWriteError {
    fn from(error: GlobalInfoError) -> Self {
        Self::GlobalInfo(error)
    }
}

impl From<SetupInfoError> for BaseDirectoryWriteError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<ExpressionInfoError> for BaseDirectoryWriteError {
    fn from(error: ExpressionInfoError) -> Self {
        Self::ExpressionInfo(error)
    }
}

impl From<VerifierInfoError> for BaseDirectoryWriteError {
    fn from(error: VerifierInfoError) -> Self {
        Self::VerifierInfo(error)
    }
}

impl From<FixedColumnError> for BaseDirectoryWriteError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<HintProgramError> for BaseDirectoryWriteError {
    fn from(error: HintProgramError) -> Self {
        Self::HintProgram(error)
    }
}

impl From<VerificationKeyError> for BaseDirectoryWriteError {
    fn from(error: VerificationKeyError) -> Self {
        Self::VerificationKey(error)
    }
}

impl From<SetupError> for BaseDirectoryWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
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
            write_regular_hint_program_for_directory(&path, &expressions)?;
        }

        let verifier_path = require_base_unit_path(unit.verifier_info(), "verifier metadata path")?;
        let verifier = read_verifier_info_binary_file(&verifier_path)?;
        if let Some(path) = unit.verifier_info_binary() {
            write_verifier_info_binary_for_directory(&path, &verifier)?;
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

fn validate_base_directory_inputs(
    layout: &KeyDirectoryLayout,
    derive_verkey: bool,
) -> Result<(), BaseDirectoryWriteError> {
    if !derive_verkey {
        return validate_key_directory_layout(layout).map_err(BaseDirectoryWriteError::from);
    }

    let mut seen = BTreeSet::new();
    for required in layout.required_paths() {
        if matches!(
            required.role,
            "unit verification-key metadata" | "unit verification-key binary"
        ) {
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
    std::fs::write(path, &bytes).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "write global-info binary failed: {}: {error}",
            path.display()
        ))
    })?;
    Ok(bytes.len() as u64)
}

fn write_unit_setup_info_binary_for_directory(
    path: &Path,
    setup: &UnitSetupInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_unit_setup_info(setup)?;
    std::fs::write(path, &bytes).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "write setup metadata binary failed: {}: {error}",
            path.display()
        ))
    })?;
    Ok(bytes.len() as u64)
}

fn write_expression_info_binary_for_directory(
    path: &Path,
    expressions: &ExpressionInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_expression_info(expressions)?;
    std::fs::write(path, &bytes).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "write expression metadata binary failed: {}: {error}",
            path.display()
        ))
    })?;
    Ok(bytes.len() as u64)
}

fn write_regular_hint_program_for_directory(
    path: &Path,
    expressions: &ExpressionInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let program = regular_hint_program_from_expression_info(expressions)?;
    let hint_file = encode_regular_hint_program(&program)?;
    let hint_section = parse_sectioned_file(&hint_file, *b"chps", 1)
        .map_err(|error| BaseDirectoryWriteError::message(error.to_string()))?
        .sections
        .into_iter()
        .find(|section| section.id == 3)
        .ok_or_else(|| {
            BaseDirectoryWriteError::message("encoded hint program is missing hint section")
        })?;

    let existing = std::fs::read(path).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "read expression program for hint merge failed: {}: {error}",
            path.display()
        ))
    })?;
    let mut file = parse_sectioned_file(&existing, *b"chps", 1).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "parse expression program for hint merge failed: {}: {error}",
            path.display()
        ))
    })?;
    file.sections.retain(|section| section.id != 3);
    file.sections.push(hint_section);
    file.sections.sort_by_key(|section| section.id);
    let bytes = encode_sectioned_file(&file)
        .map_err(|error| BaseDirectoryWriteError::message(error.to_string()))?;
    std::fs::write(path, &bytes).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "write expression program hint section failed: {}: {error}",
            path.display()
        ))
    })?;
    Ok(bytes.len() as u64)
}

fn write_verifier_info_binary_for_directory(
    path: &Path,
    verifier: &VerifierInfo,
) -> Result<u64, BaseDirectoryWriteError> {
    let bytes = encode_verifier_info(verifier)?;
    std::fs::write(path, &bytes).map_err(|error| {
        BaseDirectoryWriteError::message(format!(
            "write verifier metadata binary failed: {}: {error}",
            path.display()
        ))
    })?;
    Ok(bytes.len() as u64)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsDirectoryWriteError {
    KeyDirectory(KeyDirectoryError),
    SetupInfo(SetupInfoError),
    PcsPlan(PcsPlanError),
    ConstantTree(ConstantTreeError),
    PcsMaterial(PcsSetupMaterialError),
    MissingUnitPath { role: &'static str },
    PcsPlanMismatch,
    Io { message: String },
}

impl fmt::Display for PcsDirectoryWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDirectory(error) => write!(f, "{error}"),
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::PcsPlan(error) => write!(f, "{error}"),
            Self::ConstantTree(error) => write!(f, "{error}"),
            Self::PcsMaterial(error) => write!(f, "{error}"),
            Self::MissingUnitPath { role } => write!(f, "missing unit {role}"),
            Self::PcsPlanMismatch => write!(f, "PCS setup plan does not match setup metadata"),
            Self::Io { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PcsDirectoryWriteError {}

impl From<KeyDirectoryError> for PcsDirectoryWriteError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

impl From<SetupInfoError> for PcsDirectoryWriteError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<PcsPlanError> for PcsDirectoryWriteError {
    fn from(error: PcsPlanError) -> Self {
        Self::PcsPlan(error)
    }
}

impl From<ConstantTreeError> for PcsDirectoryWriteError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

impl From<PcsSetupMaterialError> for PcsDirectoryWriteError {
    fn from(error: PcsSetupMaterialError) -> Self {
        Self::PcsMaterial(error)
    }
}

pub fn write_pcs_setup_plan_file(
    setup_info_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<PcsFileWriteReport, PcsDirectoryWriteError> {
    let output_path = output_path.as_ref().to_path_buf();
    let bytes = encode_pcs_setup_plan_from_path(setup_info_path.as_ref())?;
    write_output_bytes(&output_path, &bytes)?;
    Ok(PcsFileWriteReport {
        path: output_path,
        bytes_written: bytes.len() as u64,
    })
}

pub fn write_pcs_setup_material_file(
    setup_info_path: impl AsRef<Path>,
    plan_path: impl AsRef<Path>,
    fixed_columns_path: impl AsRef<Path>,
    constant_tree_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<PcsFileWriteReport, PcsDirectoryWriteError> {
    let output_path = output_path.as_ref().to_path_buf();
    let bytes = encode_pcs_setup_material_from_paths(
        setup_info_path.as_ref(),
        plan_path.as_ref(),
        fixed_columns_path.as_ref(),
        constant_tree_path.as_ref(),
    )?;
    write_output_bytes(&output_path, &bytes)?;
    Ok(PcsFileWriteReport {
        path: output_path,
        bytes_written: bytes.len() as u64,
    })
}

pub fn write_pcs_directory(
    root: impl AsRef<Path>,
) -> Result<PcsDirectoryWriteReport, PcsDirectoryWriteError> {
    let layout = read_key_directory_layout(root).map_err(PcsDirectoryWriteError::from)?;
    write_pcs_directory_from_layout(&layout)
}

pub fn write_pcs_directory_from_layout(
    layout: &KeyDirectoryLayout,
) -> Result<PcsDirectoryWriteReport, PcsDirectoryWriteError> {
    let mut bytes_written = 0_u64;

    for unit in &layout.units {
        let setup_path = require_unit_path(unit.setup_info(), "setup metadata path")?;
        let output = require_unit_path(unit.pcs_setup_plan(), "PCS plan output path")?;

        let bytes = encode_pcs_setup_plan_from_path(&setup_path)?;
        write_output_bytes(&output, &bytes)?;
        bytes_written = bytes_written.saturating_add(bytes.len() as u64);
    }

    Ok(PcsDirectoryWriteReport {
        unit_count: layout.units.len(),
        bytes_written,
    })
}

pub fn write_pcs_material_directory(
    root: impl AsRef<Path>,
) -> Result<PcsDirectoryWriteReport, PcsDirectoryWriteError> {
    let layout = read_key_directory_layout(root).map_err(PcsDirectoryWriteError::from)?;
    write_pcs_material_directory_from_layout(&layout)
}

pub fn write_pcs_material_directory_from_layout(
    layout: &KeyDirectoryLayout,
) -> Result<PcsDirectoryWriteReport, PcsDirectoryWriteError> {
    let mut bytes_written = 0_u64;

    for unit in &layout.units {
        let setup_path = require_unit_path(unit.setup_info(), "setup metadata path")?;
        let plan_path = require_unit_path(unit.pcs_setup_plan(), "PCS plan path")?;
        let output = require_unit_path(unit.pcs_setup_material(), "PCS material output path")?;

        let bytes = encode_pcs_setup_material_from_paths(
            &setup_path,
            &plan_path,
            &unit.fixed_columns,
            &unit.constant_tree,
        )?;
        write_output_bytes(&output, &bytes)?;
        bytes_written = bytes_written.saturating_add(bytes.len() as u64);
    }

    Ok(PcsDirectoryWriteReport {
        unit_count: layout.units.len(),
        bytes_written,
    })
}

fn require_unit_path(
    path: Option<PathBuf>,
    role: &'static str,
) -> Result<PathBuf, PcsDirectoryWriteError> {
    path.ok_or(PcsDirectoryWriteError::MissingUnitPath { role })
}

fn encode_pcs_setup_plan_from_path(path: &Path) -> Result<Vec<u8>, PcsDirectoryWriteError> {
    let setup = read_unit_setup_info_binary_file(path)?;
    let plan = derive_pcs_setup_plan(&setup)?;
    encode_pcs_setup_plan(&plan).map_err(Into::into)
}

fn encode_pcs_setup_material_from_paths(
    setup_info_path: &Path,
    plan_path: &Path,
    fixed_columns_path: &Path,
    constant_tree_path: &Path,
) -> Result<Vec<u8>, PcsDirectoryWriteError> {
    let setup = read_unit_setup_info_binary_file(setup_info_path)?;
    let plan = read_pcs_setup_plan_file(plan_path)?;
    let expected_plan = derive_pcs_setup_plan(&setup)?;
    if plan != expected_plan {
        return Err(PcsDirectoryWriteError::PcsPlanMismatch);
    }

    let fixed_bytes =
        std::fs::read(fixed_columns_path).map_err(|error| PcsDirectoryWriteError::Io {
            message: error.to_string(),
        })?;
    let tree = read_constant_tree_file(constant_tree_path, &setup)?;
    let material = build_pcs_setup_material(&plan, &fixed_bytes, &tree)?;
    encode_pcs_setup_material(&material).map_err(Into::into)
}

fn write_output_bytes(path: &Path, bytes: &[u8]) -> Result<(), PcsDirectoryWriteError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| PcsDirectoryWriteError::Io {
            message: error.to_string(),
        })?;
    }
    std::fs::write(path, bytes).map_err(|error| PcsDirectoryWriteError::Io {
        message: error.to_string(),
    })?;
    Ok(())
}

impl From<FixedColumnError> for SetupError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<ConstantTreeError> for SetupError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

impl From<VerificationKeyError> for SetupError {
    fn from(error: VerificationKeyError) -> Self {
        Self::VerificationKey(error)
    }
}

impl From<DomainError> for SetupError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<FieldError> for SetupError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

pub fn extend_fixed_columns_for_constant_tree(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    extend_fixed_columns_for_constant_tree_with_backend(value, setup, FixedExtensionBackend::Cpu)
}

pub fn extend_fixed_columns_for_constant_tree_with_backend(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<Vec<u8>, SetupError> {
    let extended_row_count = checked_domain_len(setup.stark.n_bits_ext)?;
    let columns = fixed_columns_for_extension(value, setup)?;
    let extended_columns = match backend {
        FixedExtensionBackend::Cpu => extend_columns_on_cpu(&columns, setup)?,
        FixedExtensionBackend::Cuda => extend_columns_on_cuda(&columns, setup)?,
    };

    encode_extended_columns(&extended_columns, extended_row_count)
}

fn fixed_columns_for_extension(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    let raw = encode_raw_fixed_columns(value, setup)?;
    let row_count = checked_domain_len(setup.stark.n_bits)?;
    let column_count =
        usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?;
    let word_count = row_count
        .checked_mul(column_count)
        .ok_or(SetupError::LengthOverflow)?;
    if raw.len()
        != word_count
            .checked_mul(WORD_BYTES)
            .ok_or(SetupError::LengthOverflow)?
    {
        return Err(SetupError::LengthOverflow);
    }
    let mut extended_columns = Vec::with_capacity(column_count);
    for column in 0..column_count {
        let mut values = Vec::with_capacity(row_count);
        for row in 0..row_count {
            let word_index = row
                .checked_mul(column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(SetupError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(SetupError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                raw[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        extended_columns.push(values);
    }
    Ok(extended_columns)
}

fn extend_columns_on_cpu(
    columns: &[Vec<Felt>],
    setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    columns
        .iter()
        .map(|values| {
            coset_extend_evaluations(
                values,
                setup.stark.n_bits as usize,
                setup.stark.n_bits_ext as usize,
            )
            .map_err(Into::into)
        })
        .collect()
}

#[cfg(feature = "cuda")]
fn extend_columns_on_cuda(
    columns: &[Vec<Felt>],
    setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    columns
        .iter()
        .map(|values| {
            let source = values
                .iter()
                .map(|value| value.to_u64())
                .collect::<Vec<_>>();
            let extended = lzvm_accel::cuda_goldilocks_coset_extend(
                &source,
                setup.stark.n_bits as usize,
                setup.stark.n_bits_ext as usize,
            )
            .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
            extended
                .into_iter()
                .map(|value| Felt::from_canonical(value).map_err(Into::into))
                .collect()
        })
        .collect()
}

#[cfg(not(feature = "cuda"))]
fn extend_columns_on_cuda(
    _columns: &[Vec<Felt>],
    _setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    Err(SetupError::CudaUnavailable)
}

fn encode_extended_columns(
    extended_columns: &[Vec<Felt>],
    extended_row_count: usize,
) -> Result<Vec<u8>, SetupError> {
    let column_count = extended_columns.len();
    let byte_count = extended_row_count
        .checked_mul(column_count)
        .and_then(|count| count.checked_mul(WORD_BYTES))
        .ok_or(SetupError::LengthOverflow)?;
    for column_values in extended_columns {
        if column_values.len() != extended_row_count {
            return Err(SetupError::LengthOverflow);
        }
    }

    let mut out = Vec::with_capacity(byte_count);
    for row in 0..extended_row_count {
        for column_values in extended_columns {
            out.extend_from_slice(&column_values[row].to_le_bytes());
        }
    }
    Ok(out)
}

pub fn write_base_fixed_columns(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<FixedColumnWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let staging_path = staging_path_for(&path);
    write_raw_fixed_columns_file(&staging_path, value, setup)?;
    read_raw_fixed_column_layout_file(&staging_path, setup, &value.group_name, &value.unit_name)?;
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish fixed columns",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(FixedColumnWriteReport {
        path,
        bytes_written,
    })
}

pub fn write_constant_tree_leaves(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeLeavesWriteReport, SetupError> {
    write_constant_tree_leaves_with_backend(path, value, setup, FixedExtensionBackend::Cpu)
}

pub fn write_constant_tree_leaves_with_backend(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<ConstantTreeLeavesWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let leaves = extend_fixed_columns_for_constant_tree_with_backend(value, setup, backend)?;
    let expected_len = checked_domain_len(setup.stark.n_bits_ext)?
        .checked_mul(usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?)
        .and_then(|words| words.checked_mul(8))
        .ok_or(SetupError::LengthOverflow)?;
    if leaves.len() != expected_len {
        return Err(SetupError::LengthOverflow);
    }

    let staging_path = staging_path_for(&path);
    std::fs::write(&staging_path, &leaves).map_err(|error| SetupError::Io {
        role: "write constant-tree leaves staging file",
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    if bytes_written != u64::try_from(expected_len).map_err(|_| SetupError::LengthOverflow)? {
        return Err(SetupError::LengthOverflow);
    }
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish constant-tree leaves",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(ConstantTreeLeavesWriteReport {
        path,
        bytes_written,
        row_count: 1_u64
            .checked_shl(setup.stark.n_bits_ext)
            .ok_or(SetupError::LengthOverflow)?,
        column_count: setup.n_constants,
    })
}

pub fn build_constant_tree_from_fixed_columns(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    build_constant_tree_from_fixed_columns_with_backend(value, setup, FixedExtensionBackend::Cpu)
}

pub fn build_constant_tree_from_fixed_columns_with_backend(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<Vec<u8>, SetupError> {
    let leaves = extend_fixed_columns_for_constant_tree_with_backend(value, setup, backend)?;
    build_constant_tree_from_leaves_with_backend(&leaves, setup, backend)
}

pub fn build_constant_tree_from_leaves(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    build_constant_tree_from_leaves_with_backend(leaves, setup, FixedExtensionBackend::Cpu)
}

pub fn build_constant_tree_from_leaves_with_backend(
    leaves: &[u8],
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<Vec<u8>, SetupError> {
    match backend {
        FixedExtensionBackend::Cpu => build_constant_tree_from_leaves_on_cpu(leaves, setup),
        FixedExtensionBackend::Cuda => build_constant_tree_from_leaves_on_cuda(leaves, setup),
    }
}

fn build_constant_tree_from_leaves_on_cpu(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    let shape = constant_tree_shape(leaves, setup)?;
    let rows = read_constant_tree_leaf_rows(leaves, shape)?;

    let mut out = Vec::with_capacity(shape.expected_tree_len);
    out.extend_from_slice(leaves);

    let mut level = Vec::with_capacity(shape.row_count);
    for row in &rows {
        let digest = linear_hash(row, shape.arity)?;
        append_digest(&mut out, digest);
        level.push(digest);
    }

    while level.len() > 1 {
        let extra_zeros = (shape.arity - (level.len() % shape.arity)) % shape.arity;
        for _ in 0..extra_zeros {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
            level.push(zero);
        }

        let mut next = Vec::with_capacity(level.len() / shape.arity);
        for children in level.chunks_exact(shape.arity) {
            let digest = parent_hash(children, shape.arity)?;
            append_digest(&mut out, digest);
            next.push(digest);
        }
        level = next;
    }

    parse_constant_tree_bytes(out.clone(), setup)?;
    Ok(out)
}

#[cfg(feature = "cuda")]
fn build_constant_tree_from_leaves_on_cuda(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    let shape = constant_tree_shape(leaves, setup)?;
    let rows = read_constant_tree_leaf_rows(leaves, shape)?;

    let mut out = Vec::with_capacity(shape.expected_tree_len);
    out.extend_from_slice(leaves);

    let mut level = cuda_linear_hashes(&rows, shape.arity)?;
    for digest in &level {
        append_digest(&mut out, *digest);
    }

    while level.len() > 1 {
        let extra_zeros = (shape.arity - (level.len() % shape.arity)) % shape.arity;
        for _ in 0..extra_zeros {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
            level.push(zero);
        }

        let next = cuda_parent_hashes(&level, shape.arity)?;
        for digest in &next {
            append_digest(&mut out, *digest);
        }
        level = next;
    }

    parse_constant_tree_bytes(out.clone(), setup)?;
    Ok(out)
}

#[cfg(not(feature = "cuda"))]
fn build_constant_tree_from_leaves_on_cuda(
    _leaves: &[u8],
    _setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    Err(SetupError::CudaUnavailable)
}

pub fn write_constant_tree_from_fixed_columns(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeWriteReport, SetupError> {
    let tree = build_constant_tree_from_fixed_columns(value, setup)?;
    write_base_constant_tree(path, &tree, setup, None)
}

pub fn write_base_constant_tree(
    path: impl AsRef<Path>,
    value: &[u8],
    setup: &UnitSetupInfo,
    expected_root: Option<&VerificationKeyRoot>,
) -> Result<ConstantTreeWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let tree = parse_constant_tree_bytes(value.to_vec(), setup)?;
    let root = tree.root()?;
    if let Some(expected) = expected_root {
        if expected != &root {
            return Err(SetupError::ConstantTreeRootMismatch {
                expected: expected.clone(),
                found: root,
            });
        }
    }

    let staging_path = staging_path_for(&path);
    std::fs::write(&staging_path, value).map_err(|error| SetupError::Io {
        role: "write constant-tree staging file",
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    let staged_tree = read_constant_tree_file(&staging_path, setup)?;
    let staged_root = staged_tree.root()?;
    if staged_root != root {
        return Err(SetupError::ConstantTreeRootMismatch {
            expected: root,
            found: staged_root,
        });
    }
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish constant tree",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(ConstantTreeWriteReport {
        path,
        bytes_written,
        root,
    })
}

pub fn write_verification_key_from_constant_tree(
    binary_path: impl AsRef<Path>,
    tree_bytes: &[u8],
    setup: &UnitSetupInfo,
) -> Result<VerificationKeyWriteReport, SetupError> {
    let tree = parse_constant_tree_bytes(tree_bytes.to_vec(), setup)?;
    let root = tree.root()?;
    let binary_bytes = encode_verification_key_binary(&root)?;

    let binary_path = binary_path.as_ref().to_path_buf();
    let binary_staging =
        write_staging_bytes(&binary_path, &binary_bytes, "verification-key binary")?;

    let binary_root = read_verification_key_binary_file(&binary_staging)?;
    if binary_root != root {
        return Err(SetupError::ConstantTreeRootMismatch {
            expected: root.clone(),
            found: binary_root,
        });
    }

    let binary_size =
        publish_staging_bytes(&binary_staging, &binary_path, "verification-key binary")?;

    Ok(VerificationKeyWriteReport {
        binary_path,
        binary_bytes: binary_size,
        root,
    })
}

fn staging_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "fixed-columns".into());
    name.push(format!(".staging.{}", std::process::id()));
    path.with_file_name(name)
}

fn write_staging_bytes(
    path: &Path,
    bytes: &[u8],
    role: &'static str,
) -> Result<PathBuf, SetupError> {
    let parent = path.parent().ok_or_else(|| SetupError::MissingParent {
        path: path.to_path_buf(),
    })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let staging_path = staging_path_for(path);
    std::fs::write(&staging_path, bytes).map_err(|error| SetupError::Io {
        role,
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    Ok(staging_path)
}

fn publish_staging_bytes(
    staging_path: &Path,
    output_path: &Path,
    role: &'static str,
) -> Result<u64, SetupError> {
    let bytes_written = std::fs::metadata(staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.to_path_buf(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(staging_path, output_path).map_err(|error| SetupError::Io {
        role,
        path: output_path.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(bytes_written)
}

fn checked_domain_len(bits: u32) -> Result<usize, SetupError> {
    1_usize.checked_shl(bits).ok_or(SetupError::LengthOverflow)
}

fn validate_native_constant_tree_setup(setup: &UnitSetupInfo) -> Result<(), SetupError> {
    if !matches!(setup.stark.merkle_tree_arity, 2 | 4) {
        return Err(SetupError::UnsupportedConstantTreeArity {
            arity: setup.stark.merkle_tree_arity,
        });
    }
    match setup.stark.verification_hash_type.as_deref() {
        None | Some("GL") => Ok(()),
        _ => Err(SetupError::UnsupportedConstantTreeHash {
            hash_type: setup.stark.verification_hash_type.clone(),
        }),
    }
}

fn constant_tree_leaf_byte_count(
    row_count: usize,
    column_count: usize,
) -> Result<usize, SetupError> {
    row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(SetupError::LengthOverflow)
}

fn expected_constant_tree_byte_count_for_setup(setup: &UnitSetupInfo) -> Result<usize, SetupError> {
    lzvm_artifacts::constant_tree::expected_constant_tree_byte_count(setup).map_err(Into::into)
}

fn constant_tree_shape(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeShape, SetupError> {
    validate_native_constant_tree_setup(setup)?;
    let arity =
        usize::try_from(setup.stark.merkle_tree_arity).map_err(|_| SetupError::LengthOverflow)?;
    let row_count = checked_domain_len(setup.stark.n_bits_ext)?;
    let column_count =
        usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?;
    let expected_leaf_len = constant_tree_leaf_byte_count(row_count, column_count)?;
    if leaves.len() != expected_leaf_len {
        return Err(SetupError::InvalidConstantTreeLeafByteLength {
            expected: expected_leaf_len,
            found: leaves.len(),
        });
    }
    Ok(ConstantTreeShape {
        arity,
        row_count,
        column_count,
        expected_tree_len: expected_constant_tree_byte_count_for_setup(setup)?,
    })
}

fn read_constant_tree_leaf_rows(
    leaves: &[u8],
    shape: ConstantTreeShape,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    let mut rows = Vec::with_capacity(shape.row_count);
    for row in 0..shape.row_count {
        let mut values = Vec::with_capacity(shape.column_count);
        for column in 0..shape.column_count {
            let word_index = row
                .checked_mul(shape.column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(SetupError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(SetupError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                leaves[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        rows.push(values);
    }
    Ok(rows)
}

fn linear_hash(values: &[Felt], arity: usize) -> Result<[Felt; HASH_WORDS], SetupError> {
    match arity {
        2 => Ok(linear_hash_arity2(values)),
        4 => Ok(linear_hash_arity4(values)),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

fn linear_hash_arity2(values: &[Felt]) -> [Felt; HASH_WORDS] {
    if values.len() <= HASH_WORDS {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        digest[..values.len()].copy_from_slice(values);
        return digest;
    }

    let mut state = [Felt::ZERO; 8];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[4..].copy_from_slice(&capacity);
        state[..HASH_WORDS].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(HASH_WORDS);
        state[..chunk_len].copy_from_slice(&values[offset..offset + chunk_len]);
        state = poseidon2_hash_8(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn linear_hash_arity4(values: &[Felt]) -> [Felt; HASH_WORDS] {
    const RATE: usize = 12;

    if values.len() <= HASH_WORDS {
        let mut digest = [Felt::ZERO; HASH_WORDS];
        digest[..values.len()].copy_from_slice(values);
        return digest;
    }

    let mut state = [Felt::ZERO; 16];
    let mut offset = 0;
    while offset < values.len() {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[RATE..].copy_from_slice(&capacity);
        state[..RATE].fill(Felt::ZERO);

        let chunk_len = (values.len() - offset).min(RATE);
        state[..chunk_len].copy_from_slice(&values[offset..offset + chunk_len]);
        state = poseidon2_hash_16(state);
        offset += chunk_len;
    }

    [state[0], state[1], state[2], state[3]]
}

fn parent_hash(
    children: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], SetupError> {
    match arity {
        2 => Ok(parent_hash_arity2(children[0], children[1])),
        4 => Ok(parent_hash_arity4(children)),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

fn parent_hash_arity2(left: [Felt; HASH_WORDS], right: [Felt; HASH_WORDS]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn parent_hash_arity4(children: &[[Felt; HASH_WORDS]]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_16([
        children[0][0],
        children[0][1],
        children[0][2],
        children[0][3],
        children[1][0],
        children[1][1],
        children[1][2],
        children[1][3],
        children[2][0],
        children[2][1],
        children[2][2],
        children[2][3],
        children[3][0],
        children[3][1],
        children[3][2],
        children[3][3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes(
    rows: &[Vec<Felt>],
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    match arity {
        2 => cuda_linear_hashes_arity2(rows),
        4 => cuda_linear_hashes_arity4(rows),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_arity2(rows: &[Vec<Felt>]) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    const WIDTH: usize = 8;

    let value_count = rows.first().map_or(0, Vec::len);
    if value_count <= HASH_WORDS {
        return Ok(rows.iter().map(|row| padded_digest(row)).collect());
    }

    let mut states = vec![[Felt::ZERO; WIDTH]; rows.len()];
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(HASH_WORDS);
        let mut input = Vec::with_capacity(rows.len() * WIDTH);
        for (state, row) in states.iter().zip(rows) {
            let capacity = [state[0], state[1], state[2], state[3]];
            let mut next = [Felt::ZERO; WIDTH];
            next[..chunk_len].copy_from_slice(&row[offset..offset + chunk_len]);
            next[HASH_WORDS..].copy_from_slice(&capacity);
            push_felt_words(&mut input, &next);
        }

        let output = lzvm_accel::cuda_poseidon2_width8(&input)
            .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
        for (state, chunk) in states.iter_mut().zip(output.chunks_exact(WIDTH)) {
            *state = felt_array_from_words(chunk)?;
        }
        offset += chunk_len;
    }

    Ok(states
        .into_iter()
        .map(|state| [state[0], state[1], state[2], state[3]])
        .collect())
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_arity4(rows: &[Vec<Felt>]) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    const RATE: usize = 12;
    const WIDTH: usize = 16;

    let value_count = rows.first().map_or(0, Vec::len);
    if value_count <= HASH_WORDS {
        return Ok(rows.iter().map(|row| padded_digest(row)).collect());
    }

    let mut states = vec![[Felt::ZERO; WIDTH]; rows.len()];
    let mut offset = 0;
    while offset < value_count {
        let chunk_len = (value_count - offset).min(RATE);
        let mut input = Vec::with_capacity(rows.len() * WIDTH);
        for (state, row) in states.iter().zip(rows) {
            let capacity = [state[0], state[1], state[2], state[3]];
            let mut next = [Felt::ZERO; WIDTH];
            next[..chunk_len].copy_from_slice(&row[offset..offset + chunk_len]);
            next[RATE..].copy_from_slice(&capacity);
            push_felt_words(&mut input, &next);
        }

        let output = lzvm_accel::cuda_poseidon2_width16(&input)
            .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
        for (state, chunk) in states.iter_mut().zip(output.chunks_exact(WIDTH)) {
            *state = felt_array_from_words(chunk)?;
        }
        offset += chunk_len;
    }

    Ok(states
        .into_iter()
        .map(|state| [state[0], state[1], state[2], state[3]])
        .collect())
}

#[cfg(feature = "cuda")]
fn cuda_parent_hashes(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    match arity {
        2 => cuda_parent_hashes_arity2(level),
        4 => cuda_parent_hashes_arity4(level),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

#[cfg(feature = "cuda")]
fn cuda_parent_hashes_arity2(
    level: &[[Felt; HASH_WORDS]],
) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    const WIDTH: usize = 8;

    let mut input = Vec::with_capacity(level.len() * HASH_WORDS);
    for children in level.chunks_exact(2) {
        push_felt_words(&mut input, &children[0]);
        push_felt_words(&mut input, &children[1]);
    }

    let output = lzvm_accel::cuda_poseidon2_width8(&input)
        .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
    output
        .chunks_exact(WIDTH)
        .map(digest_from_state_words)
        .collect()
}

#[cfg(feature = "cuda")]
fn cuda_parent_hashes_arity4(
    level: &[[Felt; HASH_WORDS]],
) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    const WIDTH: usize = 16;

    let mut input = Vec::with_capacity(level.len() * HASH_WORDS);
    for children in level.chunks_exact(4) {
        for child in children {
            push_felt_words(&mut input, child);
        }
    }

    let output = lzvm_accel::cuda_poseidon2_width16(&input)
        .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
    output
        .chunks_exact(WIDTH)
        .map(digest_from_state_words)
        .collect()
}

#[cfg(feature = "cuda")]
fn padded_digest(values: &[Felt]) -> [Felt; HASH_WORDS] {
    let mut digest = [Felt::ZERO; HASH_WORDS];
    digest[..values.len()].copy_from_slice(values);
    digest
}

#[cfg(feature = "cuda")]
fn push_felt_words(out: &mut Vec<u64>, values: &[Felt]) {
    out.extend(values.iter().map(|value| value.to_u64()));
}

#[cfg(feature = "cuda")]
fn felt_array_from_words<const WIDTH: usize>(words: &[u64]) -> Result<[Felt; WIDTH], SetupError> {
    debug_assert_eq!(words.len(), WIDTH);
    let mut values = [Felt::ZERO; WIDTH];
    for (value, word) in values.iter_mut().zip(words) {
        *value = Felt::from_canonical(*word)?;
    }
    Ok(values)
}

#[cfg(feature = "cuda")]
fn digest_from_state_words(words: &[u64]) -> Result<[Felt; HASH_WORDS], SetupError> {
    debug_assert!(words.len() >= HASH_WORDS);
    let mut digest = [Felt::ZERO; HASH_WORDS];
    for (value, word) in digest.iter_mut().zip(words) {
        *value = Felt::from_canonical(*word)?;
    }
    Ok(digest)
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}
