use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Macro definition: object-like or function-like.
#[derive(Clone)]
enum MacroDef {
    Object(String),                         // #define NAME value
    Function(Vec<String>, String),          // #define NAME(params) body
}

/// Simple preprocessor that handles #include, #define directives.
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
    macros: &mut HashMap<String, MacroDef>,
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
            let rest = trimmed["#define".len()..].trim();
            // Check for function-like macro: NAME(params) body
            // NAME must be immediately followed by '(' (no space)
            let name_end = rest.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(rest.len());
            let name = &rest[..name_end];
            let after_name = &rest[name_end..];

            if after_name.starts_with('(') {
                // Function-like macro: #define NAME(a, b) body
                let paren_end = after_name.find(')').unwrap_or(after_name.len());
                let params_str = &after_name[1..paren_end];
                let params: Vec<String> = params_str.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let body = after_name[paren_end + 1..].trim().to_string();
                macros.insert(name.to_string(), MacroDef::Function(params, body));
            } else {
                // Object-like macro: #define NAME value
                let value = after_name.trim().to_string();
                macros.insert(name.to_string(), MacroDef::Object(value));
            }
        } else if trimmed.starts_with("#undef") {
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

/// Expand macros in a line by replacing identifiers.
fn expand_macros(line: &str, macros: &HashMap<String, MacroDef>) -> String {
    if macros.is_empty() {
        return line.to_string();
    }

    let bytes = line.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &line[start..i];
            if let Some(def) = macros.get(ident) {
                match def.clone() {
                    MacroDef::Object(value) => {
                        let expanded = expand_macros(&value, macros);
                        result.push_str(&expanded);
                    }
                    MacroDef::Function(params, body) => {
                        // Check for '(' immediately after identifier
                        let mut j = i;
                        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                            j += 1;
                        }
                        if j < bytes.len() && bytes[j] == b'(' {
                            // Parse arguments
                            j += 1; // skip '('
                            let args = parse_macro_args(&line[j..]);
                            // Skip past the closing ')'
                            let mut depth = 1;
                            while j < bytes.len() && depth > 0 {
                                if bytes[j] == b'(' { depth += 1; }
                                if bytes[j] == b')' { depth -= 1; }
                                j += 1;
                            }
                            i = j;
                            // Substitute parameters in body
                            let substituted = substitute_params(&body, &params, &args);
                            let expanded = expand_macros(&substituted, macros);
                            result.push_str(&expanded);
                        } else {
                            // No '(' follows — not a function-like invocation
                            result.push_str(ident);
                        }
                    }
                }
            } else {
                result.push_str(ident);
            }
        } else if bytes[i] == b'"' {
            // Skip string literals
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

/// Parse comma-separated macro arguments from input (after the opening '(').
fn parse_macro_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'(' {
            depth += 1;
            current.push('(');
        } else if bytes[i] == b')' {
            if depth == 0 {
                break;
            }
            depth -= 1;
            current.push(')');
        } else if bytes[i] == b',' && depth == 0 {
            args.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(bytes[i] as char);
        }
        i += 1;
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() || !args.is_empty() {
        args.push(trimmed);
    }
    args
}

/// Substitute parameter names in a macro body with argument values.
fn substitute_params(body: &str, params: &[String], args: &[String]) -> String {
    let bytes = body.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &body[start..i];
            if let Some(pos) = params.iter().position(|p| p == ident) {
                if pos < args.len() {
                    result.push_str(&args[pos]);
                }
            } else {
                result.push_str(ident);
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}
