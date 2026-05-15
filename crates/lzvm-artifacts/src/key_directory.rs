use crate::constant_tree::{read_constant_tree_file, ConstantTreeError};
use crate::constraint_program::{
    encode_global_constraint_program, encode_regular_constraint_program,
    read_global_constraint_program_file, read_regular_constraint_program_file, ConstraintProgram,
    ConstraintProgramError, GlobalConstraintProgram,
};
use crate::expression_program::{
    encode_expression_program, read_expression_program_file, ExpressionProgram,
    ExpressionProgramError,
};
use crate::fixed::{expected_raw_fixed_column_byte_count, FixedColumnError};
use crate::global_info::{read_global_info_binary_file, CurveKind, GlobalInfo, GlobalInfoError};
use crate::metadata_bundle::{
    read_unit_metadata_bundle, MetadataBundleError, UnitMetadataBundle, UnitMetadataPaths,
};
use crate::pcs_material::{
    build_pcs_setup_material, read_pcs_setup_material_file, PcsSetupMaterial, PcsSetupMaterialError,
};
use crate::pcs_plan::{
    derive_pcs_setup_plan, read_pcs_setup_plan_file, PcsPlanError, PcsSetupPlan,
};
use crate::verification_key::{
    read_verification_key_binary_file, VerificationKeyError, VerificationKeyRoot,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

const GLOBAL_INFO_BIN_FILE: &str = "pilout.globalInfo.bin";
const GLOBAL_CONSTRAINTS_BIN_FILE: &str = "pilout.globalConstraints.bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDirectoryLayout {
    pub root: PathBuf,
    pub global_info: GlobalInfo,
    pub global_paths: GlobalKeyPaths,
    pub units: Vec<KeyUnitPaths>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyDirectoryCatalog {
    pub layout: KeyDirectoryLayout,
    pub global_constraints: GlobalConstraintProgram,
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
    RegularConstraints(ConstraintProgramError),
    ConstantTree(ConstantTreeError),
    Metadata(MetadataBundleError),
    PcsPlan(PcsPlanError),
    PcsMaterial(PcsSetupMaterialError),
    ExpressionProgram(ExpressionProgramError),
    VerificationKey(VerificationKeyError),
    FixedColumns(FixedColumnError),
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
            Self::RegularConstraints(error) => {
                write!(f, "key-directory regular constraint program error: {error}")
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
            .map(|prefix| append_suffix(prefix, ".pcs-plan"))
    }

    pub fn pcs_setup_material(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".pcs-material"))
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
    let units = derive_unit_paths(&root, &global_info);

    Ok(KeyDirectoryLayout {
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

pub fn read_key_directory_catalog_from_layout(
    layout: &KeyDirectoryLayout,
) -> Result<KeyDirectoryCatalog, KeyDirectoryError> {
    validate_key_directory_layout(layout)?;
    let global_constraints =
        read_global_constraint_program_file(&layout.global_paths.constraints_program)
            .map_err(KeyDirectoryError::GlobalConstraints)?;
    let mut units = Vec::with_capacity(layout.units.len());
    for unit in &layout.units {
        units.push(read_key_unit_catalog_entry(unit)?);
    }

    Ok(KeyDirectoryCatalog {
        layout: layout.clone(),
        global_constraints,
        units,
    })
}

pub fn key_directory_catalog_digest(
    catalog: &KeyDirectoryCatalog,
) -> Result<[u8; 32], KeyDirectoryError> {
    let mut hasher = Sha256::new();
    hash_bytes(&mut hasher, b"lzvm-key-directory-catalog-v1");
    hash_global_info(&mut hasher, &catalog.layout.global_info);
    hash_bytes(
        &mut hasher,
        &encode_global_constraint_program(&catalog.global_constraints).map_err(|error| {
            KeyDirectoryError::Digest {
                message: error.to_string(),
            }
        })?,
    );
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
            &encode_expression_program(&unit.expression_program).map_err(|error| {
                KeyDirectoryError::Digest {
                    message: error.to_string(),
                }
            })?,
        );
        hash_bytes(
            &mut hasher,
            &encode_regular_constraint_program(&unit.regular_constraints).map_err(|error| {
                KeyDirectoryError::Digest {
                    message: error.to_string(),
                }
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

fn read_key_unit_catalog_entry(
    paths: &KeyUnitPaths,
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

    let (constant_tree_present, constant_tree_bytes, constant_tree_root) =
        if paths.constant_tree.is_file() {
            let tree = read_constant_tree_file(&paths.constant_tree, &metadata.setup)?;
            let root = tree.root()?;
            if root != verification_key {
                return Err(KeyDirectoryError::ConstantTreeRootMismatch {
                    kind: paths.kind,
                    expected: verification_key.clone(),
                    found: root,
                });
            }
            (
                true,
                Some(u64::try_from(tree.bytes.len()).map_err(|_| {
                    KeyDirectoryError::ConstantTree(ConstantTreeError::LengthOverflow)
                })?),
                Some(root),
            )
        } else {
            (false, None, None)
        };
    let (pcs_material_present, pcs_material_bytes, pcs_material) =
        read_pcs_setup_material_companion(paths, &metadata.setup, &pcs_plan)?;

    Ok(KeyUnitCatalogEntry {
        paths: paths.clone(),
        metadata,
        pcs_plan,
        verification_key,
        expression_program,
        regular_constraints,
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
    if !path.is_file() {
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
    setup: &crate::setup_info::UnitSetupInfo,
    plan: &PcsSetupPlan,
) -> Result<(bool, Option<u64>, Option<PcsSetupMaterial>), KeyDirectoryError> {
    let Some(path) = paths.pcs_setup_material() else {
        return Ok((false, None, None));
    };
    if !path.is_file() {
        return Ok((false, None, None));
    }

    let found = read_pcs_setup_material_file(&path)?;
    let fixed_bytes =
        std::fs::read(&paths.fixed_columns).map_err(|error| KeyDirectoryError::Io {
            role: "fixed-column material input",
            message: error.to_string(),
        })?;
    let tree = read_constant_tree_file(&paths.constant_tree, setup)?;
    let expected = build_pcs_setup_material(plan, &fixed_bytes, &tree)?;
    if found != expected {
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

fn derive_unit_paths(root: &Path, global_info: &GlobalInfo) -> Vec<KeyUnitPaths> {
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
    if append_suffix(&final_circuit_prefix, ".starkinfo.bin").is_file() {
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

    units
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
