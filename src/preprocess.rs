use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Simple preprocessor that handles #include directives.
pub fn preprocess(source: &str, file_path: &str) -> String {
    let mut included = HashSet::new();
    included.insert(PathBuf::from(file_path).canonicalize().unwrap_or_default());
    preprocess_recursive(source, file_path, &mut included)
}

fn preprocess_recursive(source: &str, file_path: &str, included: &mut HashSet<PathBuf>) -> String {
    let dir = Path::new(file_path).parent().unwrap_or(Path::new("."));
    let mut result = String::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#include") {
            let rest = trimmed["#include".len()..].trim();
            let (include_path, _is_system) = if rest.starts_with('"') {
                // #include "file"
                let end = rest[1..].find('"').map(|i| i + 1);
                if let Some(end) = end {
                    (rest[1..end].to_string(), false)
                } else {
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
            } else if rest.starts_with('<') {
                // #include <file>
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

            // Resolve path relative to current file's directory
            let resolved = dir.join(&include_path);
            if let Ok(canonical) = resolved.canonicalize() {
                if included.contains(&canonical) {
                    // Already included — skip (include guard behavior)
                    continue;
                }
                included.insert(canonical);
            }

            if let Ok(contents) = std::fs::read_to_string(&resolved) {
                let processed = preprocess_recursive(
                    &contents,
                    resolved.to_str().unwrap_or(&include_path),
                    included,
                );
                result.push_str(&processed);
                result.push('\n');
            }
            // If file not found, silently skip (for system headers)
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
