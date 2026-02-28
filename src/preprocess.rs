use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Simple preprocessor that handles #include and #define directives.
pub fn preprocess(source: &str, file_path: &str) -> String {
    let mut included = HashSet::new();
    included.insert(PathBuf::from(file_path).canonicalize().unwrap_or_default());
    let mut macros = HashMap::new();
    preprocess_recursive(source, file_path, &mut included, &mut macros)
}

fn preprocess_recursive(
    source: &str,
    file_path: &str,
    included: &mut HashSet<PathBuf>,
    macros: &mut HashMap<String, String>,
) -> String {
    let dir = Path::new(file_path).parent().unwrap_or(Path::new("."));
    let mut result = String::new();

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#include") {
            let rest = trimmed["#include".len()..].trim();
            let (include_path, _is_system) = if rest.starts_with('"') {
                let end = rest[1..].find('"').map(|i| i + 1);
                if let Some(end) = end {
                    (rest[1..end].to_string(), false)
                } else {
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
            } else if rest.starts_with('<') {
                let end = rest[1..].find('>').map(|i| i + 1);
                if let Some(end) = end {
                    (rest[1..end].to_string(), true)
                } else {
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
            } else {
                result.push_str(line);
                result.push('\n');
                continue;
            };

            let resolved = dir.join(&include_path);
            if let Ok(canonical) = resolved.canonicalize() {
                if included.contains(&canonical) {
                    continue;
                }
                included.insert(canonical);
            }

            if let Ok(contents) = std::fs::read_to_string(&resolved) {
                let processed = preprocess_recursive(
                    &contents,
                    resolved.to_str().unwrap_or(&include_path),
                    included,
                    macros,
                );
                result.push_str(&processed);
                result.push('\n');
            }
        } else if trimmed.starts_with("#define") {
            // #define NAME value
            let rest = trimmed["#define".len()..].trim();
            let mut parts = rest.splitn(2, |c: char| c.is_ascii_whitespace());
            if let Some(name) = parts.next() {
                let value = parts.next().unwrap_or("").trim().to_string();
                macros.insert(name.to_string(), value);
            }
        } else if trimmed.starts_with("#undef") {
            // #undef NAME
            let name = trimmed["#undef".len()..].trim();
            macros.remove(name);
        } else {
            // Expand macros in regular lines
            let expanded = expand_macros(line, macros);
            result.push_str(&expanded);
            result.push('\n');
        }
    }

    result
}

/// Expand object-like macros in a line by replacing identifiers.
fn expand_macros(line: &str, macros: &HashMap<String, String>) -> String {
    if macros.is_empty() {
        return line.to_string();
    }

    let bytes = line.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        // Check for identifier start
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &line[start..i];
            if let Some(value) = macros.get(ident) {
                // Recursively expand (for chained macros)
                let expanded = expand_macros(value, macros);
                result.push_str(&expanded);
            } else {
                result.push_str(ident);
            }
        } else if bytes[i] == b'"' {
            // Skip string literals (don't expand macros inside strings)
            result.push('"');
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    result.push(bytes[i] as char);
                    i += 1;
                }
                result.push(bytes[i] as char);
                i += 1;
            }
            if i < bytes.len() {
                result.push('"');
                i += 1;
            }
        } else if bytes[i] == b'\'' {
            // Skip char literals
            result.push('\'');
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    result.push(bytes[i] as char);
                    i += 1;
                }
                result.push(bytes[i] as char);
                i += 1;
            }
            if i < bytes.len() {
                result.push('\'');
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}
