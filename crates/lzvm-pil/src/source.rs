use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub contents: String,
    pub file_dir: PathBuf,
    pub full_path: PathBuf,
    pub source_name: String,
}

impl SourceFile {
    pub fn file_dir(&self) -> &Path {
        &self.file_dir
    }

    pub fn full_path(&self) -> &Path {
        &self.full_path
    }
}

#[derive(Debug, Clone)]
pub struct SourceLoaderConfig {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
}

impl Default for SourceLoaderConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            include_paths: Vec::new(),
            include_path_first: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoadError {
    pub requested: PathBuf,
    pub attempted_paths: Vec<PathBuf>,
}

impl std::fmt::Display for SourceLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source {:?} was not found", self.requested)
    }
}

impl std::error::Error for SourceLoadError {}

pub struct SourceLoader {
    config: SourceLoaderConfig,
    source_to_full_path: HashMap<String, PathBuf>,
    loaded_includes: HashSet<PathBuf>,
    base_path: Option<PathBuf>,
}

impl SourceLoader {
    pub fn new(config: SourceLoaderConfig) -> Self {
        Self {
            config,
            source_to_full_path: HashMap::new(),
            loaded_includes: HashSet::new(),
            base_path: None,
        }
    }

    pub fn load_main(
        &mut self,
        file_name: impl AsRef<Path>,
    ) -> Result<SourceFile, SourceLoadError> {
        self.load_source(
            file_name.as_ref(),
            true,
            &self.config.working_dir.clone(),
            &[],
        )
    }

    pub fn load_include(
        &mut self,
        file_name: impl AsRef<Path>,
        parent_dir: &Path,
    ) -> Result<SourceFile, SourceLoadError> {
        self.load_source(file_name.as_ref(), false, parent_dir, &[])
    }

    pub fn load_include_with_search_paths(
        &mut self,
        file_name: impl AsRef<Path>,
        parent_dir: &Path,
        search_paths: &[PathBuf],
    ) -> Result<SourceFile, SourceLoadError> {
        self.load_source(file_name.as_ref(), false, parent_dir, search_paths)
    }

    pub fn load_require(
        &mut self,
        file_name: impl AsRef<Path>,
        parent_dir: &Path,
    ) -> Result<Option<SourceFile>, SourceLoadError> {
        let file_name = file_name.as_ref();
        let (identity, _, _, _) = self.resolve_existing(file_name, parent_dir, &[])?;
        if self.loaded_includes.contains(&identity) {
            return Ok(None);
        }
        self.loaded_includes.insert(identity.clone());
        match self.load_source(file_name, false, parent_dir, &[]) {
            Ok(source) => Ok(Some(source)),
            Err(error) => {
                self.loaded_includes.remove(&identity);
                Err(error)
            }
        }
    }

    pub fn full_path_for_source(&self, source_name: &str) -> Option<&Path> {
        self.source_to_full_path
            .get(source_name)
            .map(PathBuf::as_path)
    }

    fn load_source(
        &mut self,
        file_name: &Path,
        is_main: bool,
        parent_dir: &Path,
        search_paths: &[PathBuf],
    ) -> Result<SourceFile, SourceLoadError> {
        let (candidate, found_search_index, direct_search_index, attempted_paths) =
            self.resolve_existing(file_name, parent_dir, search_paths)?;
        let file_dir = candidate
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let source_name = self.source_name(
            file_name,
            &candidate,
            &file_dir,
            is_main,
            found_search_index,
            direct_search_index,
        );
        let mut contents = std::fs::read_to_string(&candidate).map_err(|_| SourceLoadError {
            requested: file_name.to_path_buf(),
            attempted_paths: attempted_paths.clone(),
        })?;
        contents.push('\n');
        self.source_to_full_path
            .insert(source_name.clone(), candidate.clone());

        Ok(SourceFile {
            contents,
            file_dir,
            full_path: candidate,
            source_name,
        })
    }

    fn resolve_existing(
        &self,
        file_name: &Path,
        parent_dir: &Path,
        search_paths: &[PathBuf],
    ) -> Result<(PathBuf, usize, usize, Vec<PathBuf>), SourceLoadError> {
        let (roots, direct_search_index) = self.search_roots(parent_dir, search_paths);
        let attempted_paths = roots
            .iter()
            .map(|root| resolve_against(root, file_name))
            .collect::<Vec<_>>();
        if let Some((found_search_index, candidate)) = attempted_paths
            .iter()
            .enumerate()
            .find(|(_, path)| path.exists())
        {
            return Ok((
                candidate.clone(),
                found_search_index,
                direct_search_index,
                attempted_paths,
            ));
        }

        Err(SourceLoadError {
            requested: file_name.to_path_buf(),
            attempted_paths,
        })
    }

    fn search_roots(&self, parent_dir: &Path, search_paths: &[PathBuf]) -> (Vec<PathBuf>, usize) {
        let parent_dir = normalize_path(parent_dir);
        let mut roots = search_paths.iter().map(normalize_path).collect::<Vec<_>>();

        let direct_search_index;
        if self.config.include_path_first {
            roots.extend(self.config.include_paths.iter().map(normalize_path));
            direct_search_index = roots.len();
            roots.push(parent_dir);
        } else {
            direct_search_index = roots.len();
            roots.push(parent_dir);
            roots.extend(self.config.include_paths.iter().map(normalize_path));
        }

        (roots, direct_search_index)
    }

    fn source_name(
        &mut self,
        requested: &Path,
        full_path: &Path,
        file_dir: &Path,
        is_main: bool,
        found_search_index: usize,
        direct_search_index: usize,
    ) -> String {
        if found_search_index != direct_search_index {
            return path_to_source_name(requested);
        }

        if is_main {
            self.base_path = Some(file_dir.to_path_buf());
            return full_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path_to_source_name(full_path));
        }

        if let Some(base_path) = &self.base_path {
            if let Ok(relative) = full_path.strip_prefix(base_path) {
                return path_to_source_name(relative);
            }
        }

        path_to_source_name(full_path)
    }
}

fn resolve_against(root: &Path, file_name: &Path) -> PathBuf {
    if file_name.is_absolute() {
        normalize_path(file_name)
    } else {
        normalize_path(root.join(file_name))
    }
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

fn path_to_source_name(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{SourceLoader, SourceLoaderConfig};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CASE_ID: AtomicUsize = AtomicUsize::new(0);

    fn case_dir(name: &str) -> PathBuf {
        let id = CASE_ID.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "lzvm-pil-source-{}-{id}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("case directory should be created");
        dir
    }

    fn write_file(root: &Path, name: &str, contents: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(path.parent().expect("test file should have a parent"))
            .expect("parent directory should be created");
        fs::write(&path, contents).expect("test file should be written");
        path
    }

    #[test]
    fn loads_main_source_and_sets_base_path() {
        let root = case_dir("main");
        let main = write_file(&root, "main.pil", "constant N = 4;");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root.clone(),
            ..SourceLoaderConfig::default()
        });

        let source = loader
            .load_main("main.pil")
            .expect("main source should load");

        assert_eq!(source.full_path, main);
        assert_eq!(source.file_dir, root);
        assert_eq!(source.source_name, "main.pil");
        assert_eq!(source.contents, "constant N = 4;\n");
        assert_eq!(
            loader.full_path_for_source("main.pil"),
            Some(source.full_path())
        );
    }

    #[test]
    fn resolves_include_from_parent_before_include_paths_by_default() {
        let root = case_dir("default-order");
        let lib = root.join("lib");
        write_file(&root, "main.pil", "include \"shared.pil\";");
        let local = write_file(&root, "shared.pil", "constant X = 1;");
        write_file(&lib, "shared.pil", "constant X = 2;");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root.clone(),
            include_paths: vec![lib],
            ..SourceLoaderConfig::default()
        });
        let main = loader.load_main("main.pil").expect("main should load");

        let include = loader
            .load_include("shared.pil", main.file_dir())
            .expect("include should load");

        assert_eq!(include.full_path, local);
        assert_eq!(include.source_name, "shared.pil");
        assert_eq!(include.contents, "constant X = 1;\n");
    }

    #[test]
    fn can_resolve_include_paths_before_parent_directory() {
        let root = case_dir("path-first");
        let lib = root.join("lib");
        write_file(&root, "main.pil", "include \"shared.pil\";");
        write_file(&root, "shared.pil", "constant X = 1;");
        let library = write_file(&lib, "shared.pil", "constant X = 2;");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root.clone(),
            include_paths: vec![lib],
            include_path_first: true,
        });
        let main = loader.load_main("main.pil").expect("main should load");

        let include = loader
            .load_include("shared.pil", main.file_dir())
            .expect("include should load");

        assert_eq!(include.full_path, library);
        assert_eq!(include.source_name, "shared.pil");
        assert_eq!(include.contents, "constant X = 2;\n");
    }

    #[test]
    fn records_relative_source_names_inside_the_main_tree() {
        let root = case_dir("relative-source");
        write_file(&root, "main.pil", "include \"nested/child.pil\";");
        let child = write_file(&root, "nested/child.pil", "constant CHILD = 1;");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root,
            ..SourceLoaderConfig::default()
        });
        let main = loader.load_main("main.pil").expect("main should load");

        let include = loader
            .load_include("nested/child.pil", main.file_dir())
            .expect("include should load");

        assert_eq!(include.full_path, child);
        assert_eq!(include.source_name, "nested/child.pil");
        assert_eq!(
            loader.full_path_for_source("nested/child.pil"),
            Some(include.full_path())
        );
    }

    #[test]
    fn require_once_skips_repeated_include_identity() {
        let root = case_dir("require-once");
        write_file(&root, "main.pil", "require \"shared.pil\";");
        write_file(&root, "shared.pil", "constant X = 1;");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root,
            ..SourceLoaderConfig::default()
        });
        let main = loader.load_main("main.pil").expect("main should load");

        let first = loader
            .load_require("shared.pil", main.file_dir())
            .expect("first require should load");
        let second = loader
            .load_require("shared.pil", main.file_dir())
            .expect("second require should not fail");

        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn extra_search_paths_are_checked_before_the_parent_directory() {
        let root = case_dir("extra-paths");
        let active = root.join("active");
        write_file(&root, "main.pil", "include \"dynamic.pil\";");
        write_file(&root, "dynamic.pil", "constant X = 1;");
        let selected = write_file(&active, "dynamic.pil", "constant X = 2;");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root,
            ..SourceLoaderConfig::default()
        });
        let main = loader.load_main("main.pil").expect("main should load");

        let include = loader
            .load_include_with_search_paths(
                "dynamic.pil",
                main.file_dir(),
                std::slice::from_ref(&active),
            )
            .expect("include should load");

        assert_eq!(include.full_path, selected);
        assert_eq!(include.source_name, "dynamic.pil");
    }

    #[test]
    fn missing_source_errors_report_attempted_paths() {
        let root = case_dir("missing");
        let lib = root.join("lib");
        fs::create_dir_all(&lib).expect("library directory should be created");
        let mut loader = SourceLoader::new(SourceLoaderConfig {
            working_dir: root.clone(),
            include_paths: vec![lib.clone()],
            ..SourceLoaderConfig::default()
        });

        let error = loader
            .load_main("missing.pil")
            .expect_err("missing source should fail");

        assert_eq!(error.requested, PathBuf::from("missing.pil"));
        assert_eq!(
            error.attempted_paths,
            vec![root.join("missing.pil"), lib.join("missing.pil")]
        );
    }
}
