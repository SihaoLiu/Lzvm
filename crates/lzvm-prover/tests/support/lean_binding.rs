pub fn contains_theorem_declaration(source: &str, name: &str) -> bool {
    uncommented_lines(source).any(|line| {
        let Some(rest) = line.trim_start().strip_prefix("theorem ") else {
            return false;
        };
        let Some(after_name) = rest.strip_prefix(name) else {
            return false;
        };
        after_name
            .chars()
            .next()
            .map(|ch| ch.is_whitespace() || ch == ':')
            .unwrap_or(true)
    })
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
    let visible_source = uncommented_lines(source).collect::<Vec<_>>().join("\n");
    let theorem_start = visible_source
        .find(&format!("theorem {name}"))
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
