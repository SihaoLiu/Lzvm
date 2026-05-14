use crate::global_info::{read_global_info_file, GlobalInfo, GlobalInfoError};
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

const GLOBAL_INFO_FILE: &str = "pilout.globalInfo.json";
const GLOBAL_CONSTRAINTS_JSON_FILE: &str = "pilout.globalConstraints.json";
const GLOBAL_CONSTRAINTS_BIN_FILE: &str = "pilout.globalConstraints.bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDirectoryLayout {
    pub root: PathBuf,
    pub global_info: GlobalInfo,
    pub global_paths: GlobalKeyPaths,
    pub units: Vec<KeyUnitPaths>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalKeyPaths {
    pub info: PathBuf,
    pub constraints_json: PathBuf,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredPath {
    pub role: &'static str,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyDirectoryError {
    GlobalInfo(GlobalInfoError),
    MissingPath { role: &'static str, path: PathBuf },
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
            Self::MissingPath { role, path } => {
                write!(f, "missing key-directory {role}: {}", path.display())
            }
        }
    }
}

impl std::error::Error for KeyDirectoryError {}

impl From<GlobalInfoError> for KeyDirectoryError {
    fn from(error: GlobalInfoError) -> Self {
        Self::GlobalInfo(error)
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
                role: "global constraints metadata",
                path: self.global_paths.constraints_json.clone(),
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
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".starkinfo.json"))
    }

    pub fn expression_info(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".expressionsinfo.json"))
    }

    pub fn verifier_info(&self) -> Option<PathBuf> {
        self.metadata_prefix
            .as_ref()
            .map(|prefix| append_suffix(prefix, ".verifierinfo.json"))
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

    pub fn verification_key_json(&self) -> PathBuf {
        append_suffix(&self.verification_key_prefix, ".verkey.json")
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
            role: "unit verification-key metadata",
            path: self.verification_key_json(),
        });
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
    let global_info = read_global_info_file(&global_paths.info)?;
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

impl GlobalKeyPaths {
    pub fn from_root(root: &Path) -> Self {
        Self {
            info: root.join(GLOBAL_INFO_FILE),
            constraints_json: root.join(GLOBAL_CONSTRAINTS_JSON_FILE),
            constraints_program: root.join(GLOBAL_CONSTRAINTS_BIN_FILE),
        }
    }
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
            units.push(KeyUnitPaths::from_prefix(
                KeyUnitKind::Basic,
                Some(group_id),
                Some(unit_id),
                Some(group_name.clone()),
                Some(unit.name.clone()),
                basic_prefix.clone(),
                Some(basic_prefix.clone()),
                Some(basic_prefix.clone()),
                basic_prefix,
            ));

            if unit.has_compressor {
                let compressor_prefix = unit_root.join("compressor").join("compressor");
                units.push(KeyUnitPaths::from_prefix(
                    KeyUnitKind::Compressor,
                    Some(group_id),
                    Some(unit_id),
                    Some(group_name.clone()),
                    Some(unit.name.clone()),
                    compressor_prefix.clone(),
                    Some(compressor_prefix.clone()),
                    Some(compressor_prefix.clone()),
                    compressor_prefix,
                ));
            }

            let recursive_first_prefix = unit_root.join("recursive1").join("recursive1");
            units.push(KeyUnitPaths::from_prefix(
                KeyUnitKind::RecursiveFirst,
                Some(group_id),
                Some(unit_id),
                Some(group_name.clone()),
                Some(unit.name.clone()),
                recursive_first_prefix.clone(),
                Some(recursive_second_prefix.clone()),
                Some(recursive_second_prefix.clone()),
                recursive_first_prefix,
            ));
        }

        units.push(KeyUnitPaths::from_prefix(
            KeyUnitKind::RecursiveSecond,
            Some(group_id),
            None,
            Some(group_name.clone()),
            None,
            recursive_second_prefix.clone(),
            Some(recursive_second_prefix.clone()),
            Some(recursive_second_prefix.clone()),
            recursive_second_prefix,
        ));
    }

    let final_prefix = program_root.join("vadcop_final").join("vadcop_final");
    units.push(KeyUnitPaths::from_prefix(
        KeyUnitKind::FinalAggregation,
        None,
        None,
        None,
        None,
        final_prefix.clone(),
        Some(final_prefix.clone()),
        Some(final_prefix.clone()),
        final_prefix,
    ));

    let final_circuit_prefix = program_root.join("recursivef").join("recursivef");
    if append_suffix(&final_circuit_prefix, ".starkinfo.json").is_file() {
        units.push(KeyUnitPaths::from_prefix(
            KeyUnitKind::FinalCircuit,
            None,
            None,
            None,
            None,
            final_circuit_prefix.clone(),
            Some(final_circuit_prefix.clone()),
            Some(final_circuit_prefix.clone()),
            final_circuit_prefix,
        ));
    }

    units
}

impl KeyUnitPaths {
    fn from_prefix(
        kind: KeyUnitKind,
        group_id: Option<usize>,
        unit_id: Option<usize>,
        group_name: Option<String>,
        unit_name: Option<String>,
        prefix: PathBuf,
        metadata_prefix: Option<PathBuf>,
        program_prefix: Option<PathBuf>,
        verification_key_prefix: PathBuf,
    ) -> Self {
        Self {
            kind,
            group_id,
            unit_id,
            group_name,
            unit_name,
            fixed_columns: append_suffix(&prefix, ".const"),
            constant_tree: append_suffix(&prefix, ".consttree"),
            prefix,
            metadata_prefix,
            program_prefix,
            verification_key_prefix,
        }
    }
}

fn append_suffix(prefix: &Path, suffix: &str) -> PathBuf {
    let mut value = prefix.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}
