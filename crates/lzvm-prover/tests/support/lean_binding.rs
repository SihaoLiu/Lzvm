use std::path::Path;

pub fn contains_theorem_declaration(source: &str, name: &str) -> bool {
    find_theorem_declaration(&visible_source(source), name).is_some()
}

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
    let theorem_start = find_theorem_declaration(&visible_source, name)
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
    let theorem_start = find_theorem_declaration(&visible_source, name)
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = visible_source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
    let body_start = theorem_start + proof_start + " := by".len();
    let body_end = visible_source[body_start..]
        .find("\ntheorem ")
        .map(|offset| body_start + offset)
        .unwrap_or_else(|| visible_source.len());
    visible_source[body_start..body_end].to_owned()
}

fn visible_source(source: &str) -> String {
    uncommented_lines(source).collect::<Vec<_>>().join("\n")
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
pub fn assert_no_uncontrolled_lean_placeholders(root: &Path) {
    let mut violations = Vec::new();
    collect_uncontrolled_lean_placeholders(root, &mut violations);
    assert!(
        violations.is_empty(),
        "Lean sources must not use uncontrolled proof placeholders: {violations:?}"
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
            for token in ["sorry", "admit", "axiom"] {
                if contains_identifier_token(line, token) {
                    violations.push(format!("{}:{}:{token}", path.display(), line_index + 1));
                }
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

fn uncommented_lines(source: &str) -> impl Iterator<Item = String> + '_ {
    let mut block_depth = 0usize;
    source.lines().map(move |line| {
        let mut visible = String::new();
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if block_depth > 0 {
                if ch == '-' && chars.peek() == Some(&'/') {
                    chars.next();
                    block_depth -= 1;
                } else if ch == '/' && chars.peek() == Some(&'-') {
                    chars.next();
                    block_depth += 1;
                }
                continue;
            }
            if ch == '-' && chars.peek() == Some(&'-') {
                break;
            }
            if ch == '/' && chars.peek() == Some(&'-') {
                chars.next();
                block_depth += 1;
                continue;
            }
            visible.push(ch);
        }
        visible
    })
}
