use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub fn contains_theorem_declaration(source: &str, name: &str) -> bool {
    find_theorem_declaration(&searchable_source(source), name).is_some()
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
    let proof_start = visible_source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
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
    let proof_start = visible_source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
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
        "Lean sources must not use uncontrolled proof placeholders: {violations:?}"
    );
}

#[allow(dead_code)]
pub fn assert_all_lean_modules_reachable_from_entrypoint(entrypoint: &Path, root: &Path) {
    let modules = collect_lean_modules(entrypoint, root);
    let entrypoint_module =
        lean_module_name(entrypoint, root).expect("Lean entrypoint should be inside Lean root");
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
fn collect_uncontrolled_lean_placeholders(path: &Path, violations: &mut Vec<String>) {
    for entry in std::fs::read_dir(path).expect("Lean source directory should read") {
        let entry = entry.expect("Lean source entry should read");
        let path = entry.path();
        if path.is_dir() {
            collect_uncontrolled_lean_placeholders(&path, violations);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("lean") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("Lean source should read");
        let visible = strip_string_literals(&visible_source(&source));
        for (line_index, line) in visible.lines().enumerate() {
            for token in ["sorry", "admit", "axiom", "opaque"] {
                if contains_identifier_token(line, token) {
                    violations.push(format!("{}:{}:{token}", path.display(), line_index + 1));
                }
            }
            if line.contains(".evidence") {
                violations.push(format!("{}:{}:.evidence", path.display(), line_index + 1));
            }
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
    let mut modules = BTreeMap::new();
    modules.insert(
        lean_module_name(entrypoint, root).expect("Lean entrypoint should be inside Lean root"),
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
    visible_source(&source)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("import ")
                .map(str::trim)
                .filter(|module| !module.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
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
    use super::{contains_identifier_token, strip_string_literals, theorem_body, visible_source};

    #[test]
    fn placeholder_tokens_match_lean_identifier_boundaries() {
        assert!(contains_identifier_token(
            "opaque hiddenProof : True",
            "opaque"
        ));
        assert!(contains_identifier_token("  admit", "admit"));
        assert!(!contains_identifier_token("opaqueName", "opaque"));
        assert!(!contains_identifier_token("admittedLemma", "admit"));
    }

    #[test]
    fn placeholder_scan_ignores_comments_and_string_literals() {
        let source = r#"
-- opaque line comment
/- opaque block comment -/
def label := "opaque string"
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
}
