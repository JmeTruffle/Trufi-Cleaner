use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Expands `%VAR%` (case-insensitive) and a leading `~`. Variable values are
/// glob-escaped so that a `[` in a user name can never act as a pattern.
/// Returns `None` when a variable is unknown: the rule simply does not apply.
pub fn expand_template(template: &str, vars: &HashMap<String, PathBuf>) -> Option<String> {
    let mut out = String::with_capacity(template.len() + 32);
    let mut rest = template;

    if let Some(after) = rest.strip_prefix('~') {
        if after.is_empty() || after.starts_with('/') || after.starts_with('\\') {
            out.push_str(&glob::Pattern::escape(&vars.get("HOME")?.to_string_lossy()));
            rest = after;
        }
    }

    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        let end = tail.find('%')?;
        let name = tail[..end].to_ascii_uppercase();
        let value = vars.get(&name)?;
        out.push_str(&glob::Pattern::escape(&value.to_string_lossy()));
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Expands list variables (`%DRIVES%`, `%STEAMLIBS%`): a template using one
/// becomes one template per value. Other variables are left for
/// [`expand_template`].
pub fn expand_lists(template: &str, lists: &HashMap<String, Vec<PathBuf>>) -> Vec<String> {
    let upper = template.to_ascii_uppercase();
    for (name, values) in lists {
        let token = format!("%{name}%");
        if let Some(pos) = upper.find(&token) {
            let (head, tail) = (&template[..pos], &template[pos + token.len()..]);
            return values
                .iter()
                .flat_map(|v| {
                    let value = glob::Pattern::escape(&v.to_string_lossy());
                    let value = value.trim_end_matches(['/', '\\']);
                    expand_lists(&format!("{head}{value}{tail}"), lists)
                })
                .collect();
        }
    }
    vec![template.to_string()]
}

fn has_wildcards(s: &str) -> bool {
    // After escaping, `[` only appears from escapes like `[[]` when it came
    // from a variable, so a literal `*` or `?` is the reliable signal.
    s.contains('*') || s.contains('?')
}

/// Normalises separators (`/` vs `\`) by rebuilding the path from components.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    path.components().collect()
}

/// Expands a template into the existing paths it refers to.
pub fn resolve_roots(template: &str, vars: &HashMap<String, PathBuf>) -> Vec<PathBuf> {
    let Some(expanded) = expand_template(template, vars) else {
        return Vec::new();
    };
    if has_wildcards(&expanded) {
        let options = glob::MatchOptions {
            case_sensitive: !cfg!(windows),
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };
        match glob::glob_with(&expanded, options) {
            Ok(paths) => paths.flatten().map(|p| normalize(&p)).collect(),
            Err(_) => Vec::new(),
        }
    } else {
        let p = PathBuf::from(unescape(&expanded));
        if p.exists() {
            vec![normalize(&p)]
        } else {
            Vec::new()
        }
    }
}

/// Reverses `glob::Pattern::escape`, which wraps each special char as `[c]`.
fn unescape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' && i + 2 < chars.len() && chars[i + 2] == ']' {
            out.push(chars[i + 1]);
            i += 3;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(home: &Path) -> HashMap<String, PathBuf> {
        HashMap::from([
            ("HOME".into(), home.to_path_buf()),
            ("LOCALAPPDATA".into(), home.join("Local")),
        ])
    }

    #[test]
    fn expands_variables_case_insensitively() {
        let v = vars(Path::new("/h"));
        assert_eq!(expand_template("%localappdata%/x", &v).unwrap(), "/h/Local/x");
        assert_eq!(expand_template("~/a", &v).unwrap(), "/h/a");
        assert!(expand_template("%NOPE%/x", &v).is_none());
        assert!(expand_template("%BROKEN", &v).is_none());
    }

    #[test]
    fn resolves_globs_and_literals() {
        let dir = tempfile::tempdir().unwrap();
        let v = vars(dir.path());
        for p in ["Local/B/User Data/Default/Cache", "Local/B/User Data/Profile 1/Cache"] {
            std::fs::create_dir_all(dir.path().join(p)).unwrap();
        }
        let mut roots = resolve_roots("%LOCALAPPDATA%/B/User Data/*/Cache", &v);
        roots.sort();
        assert_eq!(roots.len(), 2);
        assert!(roots[1].ends_with("Profile 1/Cache"));
        assert_eq!(resolve_roots("~/Local/B", &v).len(), 1);
        assert!(resolve_roots("~/Local/missing", &v).is_empty());
    }

    #[test]
    fn list_variables_fan_out() {
        let lists = HashMap::from([("DRIVES".to_string(), vec![PathBuf::from("C:\\"), PathBuf::from("D:\\")])]);
        assert_eq!(expand_lists("%drives%/NVIDIA", &lists), vec!["C:/NVIDIA", "D:/NVIDIA"]);
        assert_eq!(expand_lists("%LOCALAPPDATA%/x", &lists), vec!["%LOCALAPPDATA%/x"]);
    }

    #[test]
    fn variable_values_are_not_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("we[ird]");
        std::fs::create_dir_all(home.join("c")).unwrap();
        let v = vars(&home);
        assert_eq!(resolve_roots("~/c", &v), vec![normalize(&home.join("c"))]);
        assert_eq!(resolve_roots("~/*", &v).len(), 1);
    }
}
