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
    // Conditional compilation stack: true = active, false = skipped
    let mut cond_stack: Vec<bool> = Vec::new();

    for (line_no, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // Handle conditional compilation directives (even in skipped regions)
        if trimmed.starts_with("#ifdef") {
            let name = trimmed["#ifdef".len()..].trim();
            let active = cond_stack.last().copied().unwrap_or(true) && macros.contains_key(name);
            cond_stack.push(active);
            continue;
        }
        if trimmed.starts_with("#ifndef") {
            let name = trimmed["#ifndef".len()..].trim();
            let active = cond_stack.last().copied().unwrap_or(true) && !macros.contains_key(name);
            cond_stack.push(active);
            continue;
        }
        if trimmed.starts_with("#if ") {
            let cond_str = trimmed["#if".len()..].trim();
            // Simple: evaluate as "0" or non-zero
            let val = evaluate_simple_cond(cond_str, macros);
            let active = cond_stack.last().copied().unwrap_or(true) && val;
            cond_stack.push(active);
            continue;
        }
        if trimmed.starts_with("#elif") {
            let cond_str = trimmed["#elif".len()..].trim();
            let len = cond_stack.len();
            if len > 0 {
                let current = cond_stack[len - 1];
                if current {
                    // Previous branch was taken, skip this one
                    cond_stack[len - 1] = false;
                } else {
                    let parent_active = if len > 1 { cond_stack[len - 2] } else { true };
                    let val = evaluate_simple_cond(cond_str, macros);
                    cond_stack[len - 1] = parent_active && val;
                }
            }
            continue;
        }
        if trimmed == "#else" {
            let len = cond_stack.len();
            if len > 0 {
                let current = cond_stack[len - 1];
                let parent_active = if len > 1 { cond_stack[len - 2] } else { true };
                cond_stack[len - 1] = parent_active && !current;
            }
            continue;
        }
        if trimmed == "#endif" {
            cond_stack.pop();
            continue;
        }

        // Skip lines in inactive conditional regions
        if cond_stack.last().copied().unwrap_or(true) == false {
            continue;
        }

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
            // Replace predefined macros before general macro expansion
            let with_predefined = replace_predefined(line, file_path, line_no + 1);
            let expanded = expand_macros(&with_predefined, macros);
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

/// Replace predefined macros __FILE__ and __LINE__ in a line.
fn replace_predefined(line: &str, file_path: &str, line_no: usize) -> String {
    let bytes = line.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'"' {
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
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &line[start..i];
            match ident {
                "__FILE__" => {
                    result.push('"');
                    result.push_str(file_path);
                    result.push('"');
                }
                "__LINE__" => {
                    result.push_str(&line_no.to_string());
                }
                _ => {
                    result.push_str(ident);
                }
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Evaluate a simple conditional expression for #if / #elif.
/// Supports: integer literals, defined(NAME), and basic comparisons.
fn evaluate_simple_cond(cond: &str, macros: &HashMap<String, MacroDef>) -> bool {
    let expanded = expand_macros(cond, macros);
    let trimmed = expanded.trim();

    // Handle defined(NAME) or defined NAME
    if trimmed.starts_with("defined") {
        let rest = trimmed["defined".len()..].trim();
        let name = if rest.starts_with('(') {
            let end = rest.find(')').unwrap_or(rest.len());
            rest[1..end].trim()
        } else {
            rest.split_whitespace().next().unwrap_or("")
        };
        return macros.contains_key(name);
    }

    // Handle simple comparisons: ==, !=, >, <, >=, <=
    for (op, f) in &[
        ("==", (|a: i64, b: i64| a == b) as fn(i64, i64) -> bool),
        ("!=", (|a, b| a != b) as fn(i64, i64) -> bool),
        (">=", (|a, b| a >= b) as fn(i64, i64) -> bool),
        ("<=", (|a, b| a <= b) as fn(i64, i64) -> bool),
        (">", (|a, b| a > b) as fn(i64, i64) -> bool),
        ("<", (|a, b| a < b) as fn(i64, i64) -> bool),
    ] {
        if let Some(pos) = trimmed.find(op) {
            let lhs = trimmed[..pos].trim();
            let rhs = trimmed[pos + op.len()..].trim();
            let lv = parse_cond_value(lhs, macros);
            let rv = parse_cond_value(rhs, macros);
            return f(lv, rv);
        }
    }

    // Simple integer value: non-zero is true
    let val = parse_cond_value(trimmed, macros);
    val != 0
}

/// Parse a value in a preprocessor condition expression.
fn parse_cond_value(s: &str, macros: &HashMap<String, MacroDef>) -> i64 {
    let trimmed = s.trim();
    if trimmed.starts_with("defined") {
        let rest = trimmed["defined".len()..].trim();
        let name = if rest.starts_with('(') {
            let end = rest.find(')').unwrap_or(rest.len());
            rest[1..end].trim()
        } else {
            rest.split_whitespace().next().unwrap_or("")
        };
        return if macros.contains_key(name) { 1 } else { 0 };
    }
    trimmed.parse::<i64>().unwrap_or(0)
}

/// Substitute parameter names in a macro body with argument values.
/// Also handles # (stringize) and ## (token paste) operators.
fn substitute_params(body: &str, params: &[String], args: &[String]) -> String {
    let bytes = body.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        // Handle # (stringize) operator: #param → "arg"
        if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1] != b'#' {
            i += 1;
            // Skip whitespace after #
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            // Read identifier
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &body[start..i];
            if let Some(pos) = params.iter().position(|p| p == ident) {
                let arg = if pos < args.len() { &args[pos] } else { "" };
                result.push('"');
                // Escape special characters in the argument
                for ch in arg.chars() {
                    if ch == '"' || ch == '\\' {
                        result.push('\\');
                    }
                    result.push(ch);
                }
                result.push('"');
            } else {
                result.push('#');
                result.push_str(ident);
            }
            continue;
        }

        // Handle ## (token paste) operator
        if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1] == b'#' {
            // Remove trailing whitespace from result
            while result.ends_with(' ') {
                result.pop();
            }
            i += 2;
            // Skip whitespace after ##
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            // Read the next token and substitute if it's a parameter
            if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
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
            } else if i < bytes.len() {
                // Non-identifier token (e.g., digit)
                result.push(bytes[i] as char);
                i += 1;
            }
            continue;
        }

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
