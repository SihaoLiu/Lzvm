use crate::{
    parse_include_directives, IncludeDirective, IncludeKind, IncludeVisibility, ParseError,
    SourceFile, SourceLoadError, SourceLoader, SourceLoaderConfig,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGraphEdge {
    pub from: String,
    pub to: String,
    pub request: String,
    pub kind: IncludeKind,
    pub visibility: IncludeVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGraph {
    pub sources: Vec<SourceFile>,
    pub edges: Vec<SourceGraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceGraphError {
    Load(SourceLoadError),
    Parse(ParseError),
}

impl std::fmt::Display for SourceGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load(error) => write!(f, "{error}"),
            Self::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SourceGraphError {}

pub struct SourceGraphLoader {
    loader: SourceLoader,
}

impl SourceGraphLoader {
    pub fn new(config: SourceLoaderConfig) -> Self {
        Self {
            loader: SourceLoader::new(config),
        }
    }

    pub fn load_main(
        &mut self,
        file_name: impl AsRef<Path>,
    ) -> Result<SourceGraph, SourceGraphError> {
        let main = self
            .loader
            .load_main(file_name)
            .map_err(SourceGraphError::Load)?;
        let mut graph = SourceGraph {
            sources: Vec::new(),
            edges: Vec::new(),
        };
        let mut expanded = HashSet::new();
        self.expand_source(main, &mut graph, &mut expanded)?;
        Ok(graph)
    }

    fn expand_source(
        &mut self,
        source: SourceFile,
        graph: &mut SourceGraph,
        expanded: &mut HashSet<PathBuf>,
    ) -> Result<(), SourceGraphError> {
        let source_name = source.source_name.clone();
        let source_dir = source.file_dir.clone();
        let source_path = source.full_path.clone();
        if !graph
            .sources
            .iter()
            .any(|existing| existing.full_path == source.full_path)
        {
            graph.sources.push(source.clone());
        }
        if !expanded.insert(source_path) {
            return Ok(());
        }

        for directive in collect_static_include_directives(&source)? {
            let child = match directive.kind {
                IncludeKind::Include => Some(
                    self.loader
                        .load_include(&directive.file, &source_dir)
                        .map_err(SourceGraphError::Load)?,
                ),
                IncludeKind::Require => self
                    .loader
                    .load_require(&directive.file, &source_dir)
                    .map_err(SourceGraphError::Load)?,
            };
            let Some(child) = child else {
                continue;
            };

            graph.edges.push(SourceGraphEdge {
                from: source_name.clone(),
                to: child.source_name.clone(),
                request: directive.file,
                kind: directive.kind,
                visibility: directive.visibility,
            });
            self.expand_source(child, graph, expanded)?;
        }

        Ok(())
    }
}

pub fn collect_static_include_directives(
    source: &SourceFile,
) -> Result<Vec<IncludeDirective>, SourceGraphError> {
    parse_include_directives(source).map_err(SourceGraphError::Parse)
}

#[cfg(test)]
mod tests {
    use super::{collect_static_include_directives, SourceGraphError, SourceGraphLoader};
    use crate::{IncludeKind, IncludeVisibility, ParseError, SourceFile, SourceLoaderConfig};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CASE_ID: AtomicUsize = AtomicUsize::new(0);

    fn case_dir(name: &str) -> PathBuf {
        let id = CASE_ID.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("lzvm-pil-graph-{}-{id}-{name}", std::process::id()));
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

    fn source(name: &str, contents: &str) -> SourceFile {
        SourceFile {
            contents: contents.to_owned(),
            file_dir: PathBuf::from("/case"),
            full_path: PathBuf::from("/case").join(name),
            source_name: name.to_owned(),
        }
    }

    #[test]
    fn collects_static_include_and_require_directives() {
        let source = source(
            "main.pil",
            "include \"a.pil\";\nprivate require \"b.pil\";\npublic include \"c.pil\";",
        );

        let directives =
            collect_static_include_directives(&source).expect("directives should parse");

        assert_eq!(directives.len(), 3);
        assert_eq!(directives[0].kind, IncludeKind::Include);
        assert_eq!(directives[0].visibility, IncludeVisibility::Public);
        assert_eq!(directives[0].file, "a.pil");
        assert_eq!(directives[1].kind, IncludeKind::Require);
        assert_eq!(directives[1].visibility, IncludeVisibility::Private);
        assert_eq!(directives[1].file, "b.pil");
        assert_eq!(directives[2].kind, IncludeKind::Include);
        assert_eq!(directives[2].visibility, IncludeVisibility::Public);
        assert_eq!(directives[2].file, "c.pil");
    }

    #[test]
    fn rejects_dynamic_template_include_paths() {
        let source = source("main.pil", "include `dynamic/${name}.pil`;");

        let error =
            collect_static_include_directives(&source).expect_err("template path should fail");

        assert!(matches!(
            error,
            SourceGraphError::Parse(ParseError::TemplatePath { source_name, .. })
                if source_name == "main.pil"
        ));
    }

    #[test]
    fn loads_constant_template_include_paths() {
        let root = case_dir("template-include");
        write_file(&root, "main.pil", "include `lib/${1 + 1}.pil`;");
        write_file(&root, "lib/2.pil", "constant B = 1;");
        let mut graph_loader = SourceGraphLoader::new(SourceLoaderConfig {
            working_dir: root,
            ..SourceLoaderConfig::default()
        });

        let graph = graph_loader
            .load_main("main.pil")
            .expect("graph should load");

        assert_eq!(
            graph
                .sources
                .iter()
                .map(|source| source.source_name.as_str())
                .collect::<Vec<_>>(),
            vec!["main.pil", "lib/2.pil"]
        );
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].request, "lib/2.pil");
    }

    #[test]
    fn loads_transitive_static_source_graphs_depth_first() {
        let root = case_dir("transitive");
        write_file(&root, "main.pil", "include \"a.pil\";");
        write_file(&root, "a.pil", "include \"b.pil\";");
        write_file(&root, "b.pil", "constant B = 1;");
        let mut graph_loader = SourceGraphLoader::new(SourceLoaderConfig {
            working_dir: root,
            ..SourceLoaderConfig::default()
        });

        let graph = graph_loader
            .load_main("main.pil")
            .expect("graph should load");

        assert_eq!(
            graph
                .sources
                .iter()
                .map(|source| source.source_name.as_str())
                .collect::<Vec<_>>(),
            vec!["main.pil", "a.pil", "b.pil"]
        );
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from, "main.pil");
        assert_eq!(graph.edges[0].to, "a.pil");
        assert_eq!(graph.edges[1].from, "a.pil");
        assert_eq!(graph.edges[1].to, "b.pil");
    }

    #[test]
    fn require_directives_are_loaded_once() {
        let root = case_dir("require-once");
        write_file(
            &root,
            "main.pil",
            "require \"shared.pil\";\nrequire \"shared.pil\";",
        );
        write_file(&root, "shared.pil", "constant X = 1;");
        let mut graph_loader = SourceGraphLoader::new(SourceLoaderConfig {
            working_dir: root,
            ..SourceLoaderConfig::default()
        });

        let graph = graph_loader
            .load_main("main.pil")
            .expect("graph should load");

        assert_eq!(
            graph
                .sources
                .iter()
                .map(|source| source.source_name.as_str())
                .collect::<Vec<_>>(),
            vec!["main.pil", "shared.pil"]
        );
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].kind, IncludeKind::Require);
    }

    #[test]
    fn include_path_order_is_applied_to_graph_edges() {
        let root = case_dir("include-path-order");
        let lib = root.join("lib");
        write_file(&root, "main.pil", "include \"shared.pil\";");
        write_file(&root, "shared.pil", "constant X = 1;");
        let selected = write_file(&lib, "shared.pil", "constant X = 2;");
        let mut graph_loader = SourceGraphLoader::new(SourceLoaderConfig {
            working_dir: root,
            include_paths: vec![lib],
            include_path_first: true,
        });

        let graph = graph_loader
            .load_main("main.pil")
            .expect("graph should load");

        assert_eq!(graph.sources[1].full_path, selected);
        assert_eq!(graph.edges[0].request, "shared.pil");
    }
}
