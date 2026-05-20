use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use lzvm_artifacts::constant_tree::{read_constant_tree_file, ConstantTreeError};
use lzvm_artifacts::key_directory::{
    read_key_directory_layout, KeyDirectoryError, KeyDirectoryLayout,
};
use lzvm_artifacts::pcs_material::{
    build_pcs_setup_material, encode_pcs_setup_material, read_pcs_setup_material_file,
    PcsSetupMaterialError,
};
use lzvm_artifacts::pcs_plan::{
    derive_pcs_setup_plan, encode_pcs_setup_plan, read_pcs_setup_plan_file, PcsPlanError,
};
use lzvm_artifacts::setup_info::{read_unit_setup_info_binary_file, SetupInfoError};

use crate::{publish_staging_bytes, write_staging_bytes, SetupError};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsDirectoryWriteError {
    KeyDirectory(KeyDirectoryError),
    SetupInfo(SetupInfoError),
    PcsPlan(PcsPlanError),
    ConstantTree(ConstantTreeError),
    PcsMaterial(PcsSetupMaterialError),
    Setup(SetupError),
    MissingUnitPath { role: &'static str },
    PcsPlanMismatch,
    Io { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PcsOutputKind {
    Plan,
    Material,
}

impl fmt::Display for PcsDirectoryWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDirectory(error) => write!(f, "{error}"),
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::PcsPlan(error) => write!(f, "{error}"),
            Self::ConstantTree(error) => write!(f, "{error}"),
            Self::PcsMaterial(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
            Self::MissingUnitPath { role } => write!(f, "missing unit {role}"),
            Self::PcsPlanMismatch => write!(f, "PCS setup plan does not match setup metadata"),
            Self::Io { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PcsDirectoryWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyDirectory(error) => Some(error),
            Self::SetupInfo(error) => Some(error),
            Self::PcsPlan(error) => Some(error),
            Self::ConstantTree(error) => Some(error),
            Self::PcsMaterial(error) => Some(error),
            Self::Setup(error) => Some(error),
            Self::MissingUnitPath { .. } | Self::PcsPlanMismatch | Self::Io { .. } => None,
        }
    }
}

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

impl From<SetupError> for PcsDirectoryWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

pub fn write_pcs_setup_plan_file(
    setup_info_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<PcsFileWriteReport, PcsDirectoryWriteError> {
    let output_path = output_path.as_ref().to_path_buf();
    let bytes = encode_pcs_setup_plan_from_path(setup_info_path.as_ref())?;
    write_output_bytes(&output_path, &bytes, PcsOutputKind::Plan)?;
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
    write_output_bytes(&output_path, &bytes, PcsOutputKind::Material)?;
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
        write_output_bytes(&output, &bytes, PcsOutputKind::Plan)?;
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
    let reports = run_pcs_material_jobs(layout)?;
    let bytes_written = reports
        .iter()
        .fold(0_u64, |acc, bytes| acc.saturating_add(*bytes));

    Ok(PcsDirectoryWriteReport {
        unit_count: layout.units.len(),
        bytes_written,
    })
}

fn run_pcs_material_jobs(layout: &KeyDirectoryLayout) -> Result<Vec<u64>, PcsDirectoryWriteError> {
    let unit_count = layout.units.len();
    if unit_count == 0 {
        return Ok(Vec::new());
    }

    let parallelism = pcs_material_parallelism(unit_count);
    let next_unit = AtomicUsize::new(0);
    let cancelled = AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel();
    let mut reports = vec![None; unit_count];
    let mut first_error = None;

    thread::scope(|scope| {
        let handles = (0..parallelism)
            .map(|_| {
                let sender = sender.clone();
                let next_unit = &next_unit;
                let cancelled = &cancelled;
                scope.spawn(move || {
                    while !cancelled.load(Ordering::Acquire) {
                        let index = next_unit.fetch_add(1, Ordering::AcqRel);
                        if index >= unit_count {
                            break;
                        }
                        if sender
                            .send((index, write_pcs_material_unit(layout, index)))
                            .is_err()
                        {
                            break;
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        drop(sender);

        for (index, result) in receiver {
            match result {
                Ok(bytes) => reports[index] = Some(bytes),
                Err(error) => {
                    cancelled.store(true, Ordering::Release);
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        for handle in handles {
            if let Err(error) = handle.join() {
                std::panic::resume_unwind(error);
            }
        }
    });

    if let Some(error) = first_error {
        return Err(error);
    }

    reports
        .into_iter()
        .map(|bytes| {
            bytes.ok_or_else(|| PcsDirectoryWriteError::Io {
                message: "PCS material worker stopped before reporting".to_owned(),
            })
        })
        .collect()
}

fn write_pcs_material_unit(
    layout: &KeyDirectoryLayout,
    index: usize,
) -> Result<u64, PcsDirectoryWriteError> {
    let unit = &layout.units[index];
    let setup_path = require_unit_path(unit.setup_info(), "setup metadata path")?;
    let plan_path = require_unit_path(unit.pcs_setup_plan(), "PCS plan path")?;
    let output = require_unit_path(unit.pcs_setup_material(), "PCS material output path")?;

    let bytes = encode_pcs_setup_material_from_paths(
        &setup_path,
        &plan_path,
        &unit.fixed_columns,
        &unit.constant_tree,
    )?;
    write_output_bytes(&output, &bytes, PcsOutputKind::Material)?;
    Ok(bytes.len() as u64)
}

fn pcs_material_parallelism(unit_count: usize) -> usize {
    if unit_count <= 1 {
        return 1;
    }
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, 4)
        .min(unit_count)
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

fn write_output_bytes(
    path: &Path,
    bytes: &[u8],
    kind: PcsOutputKind,
) -> Result<(), PcsDirectoryWriteError> {
    let staging_path = write_staging_bytes(path, bytes, "write PCS setup artifact staging file")?;
    match kind {
        PcsOutputKind::Plan => {
            read_pcs_setup_plan_file(&staging_path)?;
        }
        PcsOutputKind::Material => {
            read_pcs_setup_material_file(&staging_path)?;
        }
    }
    publish_staging_bytes(&staging_path, path, "publish PCS setup artifact")?;
    Ok(())
}
