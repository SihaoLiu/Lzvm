use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub const GPU_RUNTIME_WRAPPER_SOURCE_PATH: &str = "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime.lean";
#[allow(dead_code)]
pub const GPU_RUNTIME_COMMON_SOURCE_PATH: &str =
    "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/Common.lean";
#[allow(dead_code)]
pub const GPU_RUNTIME_CORE_SOURCE_PATH: &str =
    "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/Core.lean";
#[allow(dead_code)]
pub const GPU_RUNTIME_TRACE_GATE_SOURCE_PATH: &str =
    "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/TraceGate.lean";
#[allow(dead_code)]
pub const GPU_RUNTIME_TRACE_SOURCE_PATH: &str =
    "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/Trace.lean";
#[allow(dead_code)]
pub const GPU_RUNTIME_RETAINED_BUDGET_SOURCE_PATH: &str =
    "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/RetainedBudget.lean";
#[allow(dead_code)]
pub const GPU_RUNTIME_FIXED_COLUMN_CACHE_SOURCE_PATH: &str =
    "../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/FixedColumnCache.lean";

#[allow(dead_code)]
pub const GPU_RUNTIME_SOURCE_PATHS: &[&str] = &[
    GPU_RUNTIME_WRAPPER_SOURCE_PATH,
    GPU_RUNTIME_COMMON_SOURCE_PATH,
    GPU_RUNTIME_CORE_SOURCE_PATH,
    GPU_RUNTIME_TRACE_GATE_SOURCE_PATH,
    GPU_RUNTIME_TRACE_SOURCE_PATH,
    GPU_RUNTIME_RETAINED_BUDGET_SOURCE_PATH,
    GPU_RUNTIME_FIXED_COLUMN_CACHE_SOURCE_PATH,
];

#[allow(dead_code)]
pub fn read_gpu_runtime_sources(crate_root: &Path) -> String {
    read_lean_sources(crate_root, GPU_RUNTIME_SOURCE_PATHS)
}

#[allow(dead_code)]
pub fn assert_gpu_runtime_source_paths_cover_directory(crate_root: &Path) {
    let runtime_dir = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/GpuRuntime");
    let mut expected = BTreeSet::from([canonical_lean_path(
        &crate_root.join(GPU_RUNTIME_WRAPPER_SOURCE_PATH),
    )]);
    collect_lean_source_paths(&runtime_dir, &mut expected);
    let actual = GPU_RUNTIME_SOURCE_PATHS
        .iter()
        .map(|relative_path| canonical_lean_path(&crate_root.join(relative_path)))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "Lean GPU runtime source list should cover the wrapper and every split module"
    );
}

fn collect_lean_source_paths(dir: &Path, paths: &mut BTreeSet<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("Lean GPU runtime directory should read") {
        let path = entry.expect("Lean GPU runtime entry should read").path();
        if path.is_dir() {
            collect_lean_source_paths(&path, paths);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("lean") {
            paths.insert(canonical_lean_path(&path));
        }
    }
}

fn canonical_lean_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|err| panic!("Lean source path {} should resolve: {err}", path.display()))
}

#[allow(dead_code)]
pub fn read_lean_source(crate_root: &Path, relative_path: &str) -> String {
    std::fs::read_to_string(crate_root.join(relative_path))
        .unwrap_or_else(|err| panic!("Lean source {relative_path} should read: {err}"))
}

#[allow(dead_code)]
pub fn read_lean_sources(crate_root: &Path, relative_paths: &[&str]) -> String {
    relative_paths
        .iter()
        .map(|relative_path| read_lean_source(crate_root, relative_path))
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(dead_code)]
pub fn contains_theorem_declaration(source: &str, name: &str) -> bool {
    find_theorem_declaration(&searchable_source(source), name).is_some()
}

#[allow(dead_code)]
pub fn theorem_names(source: &str) -> BTreeSet<String> {
    let source = searchable_source(source);
    let mut names = BTreeSet::new();
    let mut offset = 0;
    while let Some(start) = find_next_theorem_declaration(&source, offset) {
        if let Some(name) = theorem_name_at(&source, start) {
            names.insert(name.to_owned());
        }
        offset = start + "theorem".len();
    }
    names
}

#[allow(dead_code)]
pub fn assert_theorem_declarations(source: &str, names: &[&str]) {
    for name in names {
        assert!(
            contains_theorem_declaration(source, name),
            "Lean source should declare theorem {name}"
        );
    }
}

#[allow(dead_code)]
pub fn theorem_prefix(source: &str, name: &str) -> String {
    let visible_source = visible_source(source);
    let searchable_source = strip_string_literals(&visible_source);
    let theorem_start = find_theorem_declaration(&searchable_source, name)
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = find_proof_marker(&searchable_source, theorem_start, name);
    visible_source[theorem_start..theorem_start + proof_start].to_owned()
}

#[allow(dead_code)]
pub fn assert_theorem_prefix_contains(source: &str, name: &str, snippets: &[&str]) {
    let prefix = theorem_prefix(source, name);
    for snippet in snippets {
        assert!(
            prefix.contains(snippet),
            "Lean theorem {name} prefix should contain {snippet}"
        );
    }
}

#[allow(dead_code)]
pub fn assert_theorem_prefix_omits(source: &str, name: &str, snippets: &[&str]) {
    let prefix = theorem_prefix(source, name);
    for snippet in snippets {
        assert!(
            !prefix.contains(snippet),
            "Lean theorem {name} prefix should not contain {snippet}"
        );
    }
}

#[allow(dead_code)]
pub fn theorem_body(source: &str, name: &str) -> String {
    let visible_source = visible_source(source);
    let searchable_source = strip_string_literals(&visible_source);
    let theorem_start = find_theorem_declaration(&searchable_source, name)
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = find_proof_marker(&searchable_source, theorem_start, name);
    let body_start = theorem_start + proof_start + " := by".len();
    let body_end = find_next_theorem_declaration(&searchable_source, body_start)
        .unwrap_or(visible_source.len());
    visible_source[body_start..body_end].to_owned()
}

#[allow(dead_code)]
pub fn visible_occurrence_count(source: &str, snippet: &str) -> usize {
    searchable_source(source).matches(snippet).count()
}

#[allow(dead_code)]
pub fn visible_identifier_occurrence_count(source: &str, identifier: &str) -> usize {
    assert!(
        !identifier.is_empty(),
        "Lean identifier should not be empty"
    );
    let visible = searchable_source(source);
    visible
        .match_indices(identifier)
        .filter(|(start, _)| {
            let before = visible[..*start].chars().next_back();
            let after = visible[*start + identifier.len()..].chars().next();
            !is_lean_identifier_char(before) && !is_lean_identifier_char(after)
        })
        .count()
}

#[allow(dead_code)]
pub fn assert_theorem_body_contains_identifier(source: &str, name: &str, identifier: &str) {
    let body = theorem_body(source, name);
    assert!(
        visible_identifier_occurrence_count(&body, identifier) > 0,
        "Lean theorem {name} body should contain identifier {identifier}"
    );
}

#[allow(dead_code)]
pub fn assert_theorem_body_omits_identifier(source: &str, name: &str, identifier: &str) {
    let body = theorem_body(source, name);
    assert!(
        visible_identifier_occurrence_count(&body, identifier) == 0,
        "Lean theorem {name} body should not contain identifier {identifier}"
    );
}

#[allow(dead_code)]
pub fn assert_theorem_routes_accepted_evidence_by_split_helpers(source: &str, name: &str) {
    assert_theorem_declarations(source, &[name]);
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
        "abstract_verifier_sound_with_semantic_evidence",
    ] {
        assert_theorem_body_contains_identifier(source, name, identifier);
    }
    assert_theorem_body_omits_identifier(source, name, "abstract_verifier_sound");
}

fn visible_source(source: &str) -> String {
    strip_lean_comments(source)
}

fn searchable_source(source: &str) -> String {
    strip_string_literals(&visible_source(source))
}

fn find_theorem_declaration(source: &str, name: &str) -> Option<usize> {
    source.match_indices("theorem").find_map(|(start, _)| {
        if start > 0 {
            let previous = source[..start].chars().next_back()?;
            if previous.is_alphanumeric() || previous == '_' {
                return None;
            }
        }
        let rest = &source[start + "theorem".len()..];
        let first = rest.chars().next()?;
        if !first.is_whitespace() {
            return None;
        }
        let rest = rest.trim_start();
        let after_name = rest.strip_prefix(name)?;
        if after_name
            .chars()
            .next()
            .map(|ch| ch.is_whitespace() || ch == ':')
            .unwrap_or(true)
        {
            Some(start)
        } else {
            None
        }
    })
}

fn theorem_name_at(source: &str, theorem_start: usize) -> Option<&str> {
    let rest = source[theorem_start + "theorem".len()..].trim_start();
    let name_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ':')
        .unwrap_or(rest.len());
    (name_end > 0).then_some(&rest[..name_end])
}

fn find_proof_marker(source: &str, theorem_start: usize, name: &str) -> usize {
    source[theorem_start..]
        .match_indices(" := by")
        .find_map(|(offset, marker)| {
            let after_by = source[theorem_start + offset + marker.len()..]
                .chars()
                .next();
            (!is_lean_identifier_char(after_by)).then_some(offset)
        })
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"))
}

fn find_next_theorem_declaration(source: &str, after: usize) -> Option<usize> {
    source[after..]
        .match_indices("theorem")
        .map(|(offset, _)| after + offset)
        .find(|&start| {
            if start > 0 {
                let Some(previous) = source[..start].chars().next_back() else {
                    return false;
                };
                if previous.is_alphanumeric() || previous == '_' {
                    return false;
                }
            }
            let rest = &source[start + "theorem".len()..];
            rest.chars()
                .next()
                .map(char::is_whitespace)
                .unwrap_or(false)
        })
}

#[allow(dead_code)]
pub fn assert_theorem_body_omits(source: &str, name: &str, snippets: &[&str]) {
    let body = theorem_body(source, name);
    for snippet in snippets {
        assert!(
            !body.contains(snippet),
            "Lean theorem {name} body should not contain {snippet}"
        );
    }
}

#[allow(dead_code)]
pub fn assert_theorem_body_contains(source: &str, name: &str, snippets: &[&str]) {
    let body = theorem_body(source, name);
    for snippet in snippets {
        assert!(
            body.contains(snippet),
            "Lean theorem {name} body should contain {snippet}"
        );
    }
}

#[allow(dead_code)]
pub fn structure_field_names(source: &str, start: &str, end: &str) -> Vec<String> {
    let source = strip_string_literals(&visible_source(source));
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    let after_start = &source[start_index..];
    let end_index = after_start
        .find(end)
        .unwrap_or_else(|| panic!("source should contain {end} after {start}"));
    let mut in_fields = false;
    let mut fields = Vec::new();
    for line in after_start[..end_index].lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if line == "where" || line.ends_with(" where") {
            in_fields = true;
            continue;
        }
        if !in_fields {
            continue;
        }
        if line.starts_with("structure ")
            || line.starts_with("namespace ")
            || line.starts_with("theorem ")
            || line.starts_with("def ")
            || line.starts_with("end ")
        {
            continue;
        }
        if let Some((name, _)) = line.split_once(':') {
            fields.push(name.trim().to_owned());
        }
    }
    fields
}

#[allow(dead_code)]
pub fn assert_no_uncontrolled_lean_placeholders(root: &Path) {
    let mut violations = Vec::new();
    collect_uncontrolled_lean_placeholders(root, &mut violations);
    assert!(
        violations.is_empty(),
        "Lean sources must not use uncontrolled proof placeholders or unsafe declarations: {violations:?}"
    );
}

#[allow(dead_code)]
pub fn assert_all_lean_modules_reachable_from_entrypoint(entrypoint: &Path, root: &Path) {
    let modules = collect_lean_modules(entrypoint, root);
    let lean_workspace = root
        .parent()
        .expect("Lean source root should have a workspace parent");
    assert_module_reachability(entrypoint, lean_workspace, &modules);
}

#[allow(dead_code)]
pub fn assert_all_workspace_lean_modules_reachable_from_entrypoint(
    entrypoint: &Path,
    workspace_root: &Path,
) {
    let modules = collect_workspace_lean_modules(entrypoint, workspace_root);
    assert_module_reachability(entrypoint, workspace_root, &modules);
}

fn assert_module_reachability(
    entrypoint: &Path,
    lean_workspace: &Path,
    modules: &BTreeMap<String, PathBuf>,
) {
    let entrypoint_module = lean_module_name_from_root(entrypoint, lean_workspace)
        .expect("Lean entrypoint should be inside Lean root");
    let mut reachable = BTreeSet::new();
    let mut pending = vec![entrypoint_module];
    while let Some(module) = pending.pop() {
        if !reachable.insert(module.clone()) {
            continue;
        }
        let Some(path) = modules.get(&module) else {
            continue;
        };
        for imported in lean_imports(path) {
            if modules.contains_key(&imported) {
                pending.push(imported);
            }
        }
    }
    let missing = modules
        .keys()
        .filter(|module| !reachable.contains(*module))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "Lean sources should be reachable from {}: {missing:?}",
        entrypoint.display()
    );
}

#[allow(dead_code)]
pub fn contains_import(source: impl AsRef<str>, module: &str) -> bool {
    lean_imports_from_source(source.as_ref())
        .iter()
        .any(|imported| imported == module)
}

#[allow(dead_code)]
pub fn assert_imports(source: impl AsRef<str>, modules: &[&str]) {
    let source = source.as_ref();
    for module in modules {
        assert!(
            contains_import(source, module),
            "Lean source should import {module}"
        );
    }
}

#[allow(dead_code)]
fn collect_uncontrolled_lean_placeholders(path: &Path, violations: &mut Vec<String>) {
    if path.is_file() {
        collect_uncontrolled_lean_placeholders_from_file(path, violations);
        return;
    }
    for entry in std::fs::read_dir(path).expect("Lean source directory should read") {
        let entry = entry.expect("Lean source entry should read");
        let path = entry.path();
        if path.is_dir() {
            if should_skip_lean_workspace_dir(&path) {
                continue;
            }
            collect_uncontrolled_lean_placeholders(&path, violations);
            continue;
        }
        collect_uncontrolled_lean_placeholders_from_file(&path, violations);
    }
}

#[allow(dead_code)]
fn collect_uncontrolled_lean_placeholders_from_file(path: &Path, violations: &mut Vec<String>) {
    if path.extension().and_then(|extension| extension.to_str()) != Some("lean") {
        return;
    }
    let source = std::fs::read_to_string(path).expect("Lean source should read");
    let visible = strip_string_literals(&visible_source(&source));
    for (line_index, line) in visible.lines().enumerate() {
        for token in ["sorry", "admit", "axiom", "opaque", "unsafe"] {
            if contains_identifier_token(line, token) {
                violations.push(format!("{}:{}:{token}", path.display(), line_index + 1));
            }
        }
        if line.contains(".evidence") {
            violations.push(format!("{}:{}:.evidence", path.display(), line_index + 1));
        }
    }
}

#[allow(dead_code)]
fn contains_identifier_token(line: &str, token: &str) -> bool {
    line.match_indices(token).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + token.len()..].chars().next();
        !is_lean_identifier_char(before) && !is_lean_identifier_char(after)
    })
}

#[allow(dead_code)]
fn collect_lean_modules(entrypoint: &Path, root: &Path) -> BTreeMap<String, PathBuf> {
    let lean_workspace = root
        .parent()
        .expect("Lean source root should have a workspace parent");
    collect_lean_modules_with_module_root(entrypoint, root, lean_workspace)
}

#[allow(dead_code)]
fn collect_workspace_lean_modules(
    entrypoint: &Path,
    workspace_root: &Path,
) -> BTreeMap<String, PathBuf> {
    collect_lean_modules_with_module_root(entrypoint, workspace_root, workspace_root)
}

#[allow(dead_code)]
fn collect_lean_modules_with_module_root(
    entrypoint: &Path,
    root: &Path,
    lean_workspace: &Path,
) -> BTreeMap<String, PathBuf> {
    let mut modules = BTreeMap::new();
    modules.insert(
        lean_module_name_from_root(entrypoint, lean_workspace)
            .expect("Lean entrypoint should be inside Lean root"),
        entrypoint.to_path_buf(),
    );
    collect_lean_modules_from_dir(root, lean_workspace, &mut modules);
    modules
}

#[allow(dead_code)]
fn collect_lean_modules_from_dir(
    path: &Path,
    lean_workspace: &Path,
    modules: &mut BTreeMap<String, PathBuf>,
) {
    for entry in std::fs::read_dir(path).expect("Lean source directory should read") {
        let entry = entry.expect("Lean source entry should read");
        let path = entry.path();
        if path.is_dir() {
            if should_skip_lean_workspace_dir(&path) {
                continue;
            }
            collect_lean_modules_from_dir(&path, lean_workspace, modules);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("lean") {
            continue;
        }
        let module = path
            .strip_prefix(lean_workspace)
            .expect("Lean source should be under workspace root")
            .with_extension("")
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join(".");
        modules.insert(module, path);
    }
}

#[allow(dead_code)]
fn lean_module_name(path: &Path, root: &Path) -> Option<String> {
    let lean_workspace = root.parent()?;
    lean_module_name_from_root(path, lean_workspace)
}

#[allow(dead_code)]
fn lean_module_name_from_root(path: &Path, lean_workspace: &Path) -> Option<String> {
    let relative = path.strip_prefix(lean_workspace).ok()?.with_extension("");
    Some(
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("."),
    )
}

#[allow(dead_code)]
fn lean_imports(path: &Path) -> Vec<String> {
    let source = std::fs::read_to_string(path).expect("Lean source should read");
    lean_imports_from_source(&source)
}

#[allow(dead_code)]
fn lean_imports_from_source(source: &str) -> Vec<String> {
    strip_string_literals(&visible_source(source))
        .lines()
        .flat_map(|line| {
            let line = line.trim();
            line.strip_prefix("import ")
                .map(str::trim)
                .filter(|module| !module.is_empty())
                .map(|modules| {
                    modules
                        .split_whitespace()
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect()
}

fn should_skip_lean_workspace_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

#[allow(dead_code)]
fn is_lean_identifier_char(ch: Option<char>) -> bool {
    ch.map(|ch| ch.is_alphanumeric() || ch == '_' || ch == '\'')
        .unwrap_or(false)
}

#[allow(dead_code)]
fn strip_string_literals(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '\\' {
                if chars.next().is_some() {
                    out.push(' ');
                    out.push(' ');
                } else {
                    out.push(' ');
                }
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            out.push(if ch == '\n' { '\n' } else { ' ' });
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    out
}

fn strip_lean_comments(source: &str) -> String {
    let mut visible = String::with_capacity(source.len());
    let mut block_depth = 0usize;
    let mut in_string = false;
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if block_depth > 0 {
            if ch == '\n' {
                visible.push('\n');
                continue;
            }
            if ch == '-' && chars.peek() == Some(&'/') {
                chars.next();
                block_depth -= 1;
                continue;
            }
            if ch == '/' && chars.peek() == Some(&'-') {
                chars.next();
                block_depth += 1;
                continue;
            }
            continue;
        }

        if in_string {
            visible.push(ch);
            if ch == '\\' {
                if let Some(escaped) = chars.next() {
                    visible.push(escaped);
                }
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            visible.push(ch);
            continue;
        }
        if ch == '-' && chars.peek() == Some(&'-') {
            chars.next();
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    visible.push('\n');
                    break;
                }
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'-') {
            chars.next();
            block_depth += 1;
            continue;
        }
        visible.push(ch);
    }

    visible
}

#[cfg(test)]
mod tests {
    use super::{
        contains_identifier_token, contains_import, strip_string_literals, theorem_body,
        theorem_names, visible_source,
    };

    #[test]
    fn placeholder_tokens_match_lean_identifier_boundaries() {
        assert!(contains_identifier_token(
            "opaque hiddenProof : True",
            "opaque"
        ));
        assert!(contains_identifier_token("  admit", "admit"));
        assert!(contains_identifier_token(
            "unsafe def uncheckedProof := True",
            "unsafe"
        ));
        assert!(!contains_identifier_token("opaqueName", "opaque"));
        assert!(!contains_identifier_token("admittedLemma", "admit"));
        assert!(!contains_identifier_token("unsafeLemma", "unsafe"));
    }

    #[test]
    fn placeholder_scan_ignores_comments_and_string_literals() {
        let source = r#"
-- opaque line comment
-- unsafe line comment
/- opaque block comment -/
/- unsafe block comment -/
def label := "opaque unsafe string"
"#;

        let visible = strip_string_literals(&visible_source(source));

        assert!(
            !visible
                .lines()
                .any(|line| contains_identifier_token(line, "opaque")),
            "Lean placeholder scan should ignore comments and string literals: {visible}"
        );
    }

    #[test]
    fn placeholder_scan_ignores_nested_block_comments() {
        let hidden = ["op", "aque"].concat();
        let source = format!(
            "/- outer {hidden}\n/- inner {hidden} -/\n{hidden} -/\ntheorem visible_fact : True := by\n  trivial\n"
        );

        let visible = strip_string_literals(&visible_source(&source));

        assert!(
            !visible
                .lines()
                .any(|line| contains_identifier_token(line, &hidden)),
            "Lean placeholder scan should ignore nested comments: {visible}"
        );
        assert!(visible.contains("theorem visible_fact"));
    }

    #[test]
    fn import_matching_ignores_comments_strings_and_longer_modules() {
        let source = r#"
-- import Lzvm.Commented
/- import Lzvm.BlockCommented -/
def label := "import Lzvm.StringLiteral"
def multiline := "
import Lzvm.MultilineString
"
import Lzvm.Real
import Lzvm.Real.Extra
"#;

        assert!(contains_import(source, "Lzvm.Real"));
        assert!(contains_import(source, "Lzvm.Real.Extra"));
        assert!(!contains_import(source, "Lzvm.Commented"));
        assert!(!contains_import(source, "Lzvm.BlockCommented"));
        assert!(!contains_import(source, "Lzvm.StringLiteral"));
        assert!(!contains_import(source, "Lzvm.MultilineString"));
        assert!(!contains_import(source, "Lzvm.Re"));
    }

    #[test]
    fn theorem_body_stops_before_following_private_theorem() {
        let source = r#"
private theorem first : True := by
  exact True.intro

private theorem second : True := by
  exact True.intro
"#;

        let body = theorem_body(source, "first");

        assert!(body.contains("exact True.intro"));
        assert!(
            !body.contains("private theorem second"),
            "Lean theorem body extraction should stop before a following private theorem"
        );
    }

    #[test]
    fn theorem_names_ignore_comments_strings_and_split_signatures() {
        let source = r#"
-- theorem commented_out : True := by
/- theorem block_commented : True := by -/
def label := "theorem string_literal : True := by"

private theorem private_visible
    : True := by
  exact True.intro

theorem public_visible
    : True := by
  exact True.intro
"#;

        let names = theorem_names(source);

        assert!(names.contains("private_visible"));
        assert!(names.contains("public_visible"));
        assert!(!names.contains("commented_out"));
        assert!(!names.contains("block_commented"));
        assert!(!names.contains("string_literal"));
    }
}
