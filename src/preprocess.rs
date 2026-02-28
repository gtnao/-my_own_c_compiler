use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Macro definition: object-like or function-like.
#[derive(Clone)]
enum MacroDef {
    Object(String),                         // #define NAME value
    Function(Vec<String>, String, bool),    // #define NAME(params) body, is_variadic
}

/// Simple preprocessor that handles #include, #define directives.
pub fn preprocess(source: &str, file_path: &str) -> String {
    let mut included = HashSet::new();
    included.insert(PathBuf::from(file_path).canonicalize().unwrap_or_default());
    let mut macros = HashMap::new();
    // Predefined macros
    macros.insert("__STDC__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__STDC_VERSION__".to_string(), MacroDef::Object("201112L".to_string()));
    macros.insert("__STDC_HOSTED__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__LP64__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__x86_64__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__x86_64".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__amd64__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__amd64".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__linux__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__linux".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("linux".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__unix__".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__unix".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("unix".to_string(), MacroDef::Object("1".to_string()));
    macros.insert("__GNUC__".to_string(), MacroDef::Object("4".to_string()));
    macros.insert("__GNUC_MINOR__".to_string(), MacroDef::Object("0".to_string()));
    macros.insert("__GNUC_PATCHLEVEL__".to_string(), MacroDef::Object("0".to_string()));
    macros.insert("__SIZEOF_SHORT__".to_string(), MacroDef::Object("2".to_string()));
    macros.insert("__SIZEOF_INT__".to_string(), MacroDef::Object("4".to_string()));
    macros.insert("__SIZEOF_LONG__".to_string(), MacroDef::Object("8".to_string()));
    macros.insert("__SIZEOF_LONG_LONG__".to_string(), MacroDef::Object("8".to_string()));
    macros.insert("__SIZEOF_POINTER__".to_string(), MacroDef::Object("8".to_string()));
    macros.insert("__SIZEOF_FLOAT__".to_string(), MacroDef::Object("4".to_string()));
    macros.insert("__SIZEOF_DOUBLE__".to_string(), MacroDef::Object("8".to_string()));
    macros.insert("__CHAR_BIT__".to_string(), MacroDef::Object("8".to_string()));
    macros.insert("__BYTE_ORDER__".to_string(), MacroDef::Object("1234".to_string()));
    macros.insert("__ORDER_LITTLE_ENDIAN__".to_string(), MacroDef::Object("1234".to_string()));
    macros.insert("__ORDER_BIG_ENDIAN__".to_string(), MacroDef::Object("4321".to_string()));
    macros.insert("__INTMAX_TYPE__".to_string(), MacroDef::Object("long".to_string()));
    macros.insert("__INT64_TYPE__".to_string(), MacroDef::Object("long".to_string()));
    macros.insert("__UINT64_TYPE__".to_string(), MacroDef::Object("unsigned long".to_string()));
    macros.insert("__SIZE_TYPE__".to_string(), MacroDef::Object("unsigned long".to_string()));
    macros.insert("__PTRDIFF_TYPE__".to_string(), MacroDef::Object("long".to_string()));
    macros.insert("__INTPTR_TYPE__".to_string(), MacroDef::Object("long".to_string()));
    macros.insert("__UINTPTR_TYPE__".to_string(), MacroDef::Object("unsigned long".to_string()));
    macros.insert("__WCHAR_TYPE__".to_string(), MacroDef::Object("int".to_string()));
    macros.insert("__INT_MAX__".to_string(), MacroDef::Object("2147483647".to_string()));
    macros.insert("__LONG_MAX__".to_string(), MacroDef::Object("9223372036854775807L".to_string()));
    macros.insert("__SHRT_MAX__".to_string(), MacroDef::Object("32767".to_string()));
    macros.insert("__SCHAR_MAX__".to_string(), MacroDef::Object("127".to_string()));
    macros.insert("NULL".to_string(), MacroDef::Object("((void *)0)".to_string()));
    preprocess_recursive(source, file_path, &mut included, &mut macros)
}

/// Join backslash-continuation lines before processing.
fn join_continuation_lines(source: &str) -> String {
    let mut result = String::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        if line.ends_with('\\') {
            // Continuation: append without the trailing backslash
            result.push_str(&line[..line.len() - 1]);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn preprocess_recursive(
    source: &str,
    file_path: &str,
    included: &mut HashSet<PathBuf>,
    macros: &mut HashMap<String, MacroDef>,
) -> String {
    let source = join_continuation_lines(source);
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
                let mut params: Vec<String> = params_str.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                // Check for variadic: last param is "..."
                let is_variadic = params.last().map_or(false, |p| p == "...");
                if is_variadic {
                    params.pop(); // remove "..."
                }
                let body = after_name[paren_end + 1..].trim().to_string();
                macros.insert(name.to_string(), MacroDef::Function(params, body, is_variadic));
            } else {
                // Object-like macro: #define NAME value
                let value = after_name.trim().to_string();
                macros.insert(name.to_string(), MacroDef::Object(value));
            }
        } else if trimmed.starts_with("#undef") {
            let name = trimmed["#undef".len()..].trim();
            macros.remove(name);
        } else if trimmed.starts_with("#error") {
            let msg = trimmed["#error".len()..].trim();
            eprintln!("{}:{}: error: {}", file_path, line_no + 1, msg);
            std::process::exit(1);
        } else if trimmed.starts_with("#warning") {
            let msg = trimmed["#warning".len()..].trim();
            eprintln!("{}:{}: warning: {}", file_path, line_no + 1, msg);
        } else if trimmed.starts_with("#line") {
            // #line N — ignored (informational only)
        } else if trimmed.starts_with("#pragma") {
            // #pragma — ignored
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
                    MacroDef::Function(params, body, is_variadic) => {
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
                            // For variadic macros, collect extra args as __VA_ARGS__
                            let mut subst_params = params.clone();
                            let mut subst_args = args.clone();
                            if is_variadic {
                                // Collect args beyond named params as __VA_ARGS__
                                let va_args = if args.len() > params.len() {
                                    args[params.len()..].join(", ")
                                } else {
                                    String::new()
                                };
                                subst_params.push("__VA_ARGS__".to_string());
                                // Trim subst_args to just the named params
                                subst_args.truncate(params.len());
                                subst_args.push(va_args);
                            }
                            // Substitute parameters in body
                            let substituted = substitute_params(&body, &subst_params, &subst_args);
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

/// Evaluate a preprocessor conditional expression for #if / #elif.
/// Supports: integer literals, defined(NAME), &&, ||, !, ==, !=, <, >, <=, >=,
/// +, -, *, /, %, parentheses, bitwise &, |, ^, ~, <<, >>, ternary ? :
fn evaluate_simple_cond(cond: &str, macros: &HashMap<String, MacroDef>) -> bool {
    let mut eval = CondEval::new(cond, macros);
    eval.eval_expr() != 0
}

/// Tokenizer + recursive descent evaluator for preprocessor #if expressions.
struct CondEval<'a> {
    input: Vec<char>,
    pos: usize,
    macros: &'a HashMap<String, MacroDef>,
}

impl<'a> CondEval<'a> {
    fn new(cond: &str, macros: &'a HashMap<String, MacroDef>) -> Self {
        Self { input: cond.chars().collect(), pos: 0, macros }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.input.get(self.pos).copied()
    }

    fn peek2(&mut self) -> Option<char> {
        self.skip_ws();
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) { self.pos += 1; }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while self.pos < self.input.len() && (self.input[self.pos].is_ascii_alphanumeric() || self.input[self.pos] == '_') {
            s.push(self.input[self.pos]);
            self.pos += 1;
        }
        s
    }

    fn read_number(&mut self) -> i64 {
        let mut s = String::new();
        // Handle 0x hex prefix
        if self.pos < self.input.len() && self.input[self.pos] == '0' {
            s.push('0');
            self.pos += 1;
            if self.pos < self.input.len() && (self.input[self.pos] == 'x' || self.input[self.pos] == 'X') {
                self.pos += 1;
                let mut hex = String::new();
                while self.pos < self.input.len() && self.input[self.pos].is_ascii_hexdigit() {
                    hex.push(self.input[self.pos]);
                    self.pos += 1;
                }
                // Skip suffixes (U, L, LL, UL, ULL)
                while self.pos < self.input.len() && matches!(self.input[self.pos], 'u' | 'U' | 'l' | 'L') {
                    self.pos += 1;
                }
                return i64::from_str_radix(&hex, 16).unwrap_or(0);
            }
        }
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            s.push(self.input[self.pos]);
            self.pos += 1;
        }
        // Skip suffixes
        while self.pos < self.input.len() && matches!(self.input[self.pos], 'u' | 'U' | 'l' | 'L') {
            self.pos += 1;
        }
        s.parse::<i64>().unwrap_or(0)
    }

    // expr = ternary
    fn eval_expr(&mut self) -> i64 {
        self.eval_ternary()
    }

    // ternary = logical_or ("?" expr ":" ternary)?
    fn eval_ternary(&mut self) -> i64 {
        let val = self.eval_logical_or();
        self.skip_ws();
        if self.peek() == Some('?') {
            self.advance();
            let then_val = self.eval_expr();
            self.skip_ws();
            if self.peek() == Some(':') { self.advance(); }
            let else_val = self.eval_ternary();
            if val != 0 { then_val } else { else_val }
        } else {
            val
        }
    }

    // logical_or = logical_and ("||" logical_and)*
    fn eval_logical_or(&mut self) -> i64 {
        let mut val = self.eval_logical_and();
        loop {
            self.skip_ws();
            if self.peek() == Some('|') && self.peek2() == Some('|') {
                self.advance(); self.advance();
                let rhs = self.eval_logical_and();
                val = if val != 0 || rhs != 0 { 1 } else { 0 };
            } else {
                break;
            }
        }
        val
    }

    // logical_and = bitwise_or ("&&" bitwise_or)*
    fn eval_logical_and(&mut self) -> i64 {
        let mut val = self.eval_bitwise_or();
        loop {
            self.skip_ws();
            if self.peek() == Some('&') && self.peek2() == Some('&') {
                self.advance(); self.advance();
                let rhs = self.eval_bitwise_or();
                val = if val != 0 && rhs != 0 { 1 } else { 0 };
            } else {
                break;
            }
        }
        val
    }

    // bitwise_or = bitwise_xor ("|" bitwise_xor)*
    fn eval_bitwise_or(&mut self) -> i64 {
        let mut val = self.eval_bitwise_xor();
        loop {
            self.skip_ws();
            if self.peek() == Some('|') && self.peek2() != Some('|') {
                self.advance();
                val |= self.eval_bitwise_xor();
            } else {
                break;
            }
        }
        val
    }

    // bitwise_xor = bitwise_and ("^" bitwise_and)*
    fn eval_bitwise_xor(&mut self) -> i64 {
        let mut val = self.eval_bitwise_and();
        loop {
            self.skip_ws();
            if self.peek() == Some('^') {
                self.advance();
                val ^= self.eval_bitwise_and();
            } else {
                break;
            }
        }
        val
    }

    // bitwise_and = equality ("&" equality)*
    fn eval_bitwise_and(&mut self) -> i64 {
        let mut val = self.eval_equality();
        loop {
            self.skip_ws();
            if self.peek() == Some('&') && self.peek2() != Some('&') {
                self.advance();
                val &= self.eval_equality();
            } else {
                break;
            }
        }
        val
    }

    // equality = relational (("==" | "!=") relational)*
    fn eval_equality(&mut self) -> i64 {
        let mut val = self.eval_relational();
        loop {
            self.skip_ws();
            if self.peek() == Some('=') && self.peek2() == Some('=') {
                self.advance(); self.advance();
                let rhs = self.eval_relational();
                val = if val == rhs { 1 } else { 0 };
            } else if self.peek() == Some('!') && self.peek2() == Some('=') {
                self.advance(); self.advance();
                let rhs = self.eval_relational();
                val = if val != rhs { 1 } else { 0 };
            } else {
                break;
            }
        }
        val
    }

    // relational = shift (("<" | ">" | "<=" | ">=") shift)*
    fn eval_relational(&mut self) -> i64 {
        let mut val = self.eval_shift();
        loop {
            self.skip_ws();
            if self.peek() == Some('<') && self.peek2() == Some('=') {
                self.advance(); self.advance();
                let rhs = self.eval_shift();
                val = if val <= rhs { 1 } else { 0 };
            } else if self.peek() == Some('>') && self.peek2() == Some('=') {
                self.advance(); self.advance();
                let rhs = self.eval_shift();
                val = if val >= rhs { 1 } else { 0 };
            } else if self.peek() == Some('<') && self.peek2() != Some('<') {
                self.advance();
                let rhs = self.eval_shift();
                val = if val < rhs { 1 } else { 0 };
            } else if self.peek() == Some('>') && self.peek2() != Some('>') {
                self.advance();
                let rhs = self.eval_shift();
                val = if val > rhs { 1 } else { 0 };
            } else {
                break;
            }
        }
        val
    }

    // shift = add (("<<" | ">>") add)*
    fn eval_shift(&mut self) -> i64 {
        let mut val = self.eval_add();
        loop {
            self.skip_ws();
            if self.peek() == Some('<') && self.peek2() == Some('<') {
                self.advance(); self.advance();
                val <<= self.eval_add();
            } else if self.peek() == Some('>') && self.peek2() == Some('>') {
                self.advance(); self.advance();
                val >>= self.eval_add();
            } else {
                break;
            }
        }
        val
    }

    // add = mul (("+" | "-") mul)*
    fn eval_add(&mut self) -> i64 {
        let mut val = self.eval_mul();
        loop {
            self.skip_ws();
            if self.peek() == Some('+') {
                self.advance();
                val += self.eval_mul();
            } else if self.peek() == Some('-') {
                self.advance();
                val -= self.eval_mul();
            } else {
                break;
            }
        }
        val
    }

    // mul = unary (("*" | "/" | "%") unary)*
    fn eval_mul(&mut self) -> i64 {
        let mut val = self.eval_unary();
        loop {
            self.skip_ws();
            if self.peek() == Some('*') {
                self.advance();
                val *= self.eval_unary();
            } else if self.peek() == Some('/') {
                self.advance();
                let rhs = self.eval_unary();
                if rhs != 0 { val /= rhs; }
            } else if self.peek() == Some('%') {
                self.advance();
                let rhs = self.eval_unary();
                if rhs != 0 { val %= rhs; }
            } else {
                break;
            }
        }
        val
    }

    // unary = "!" unary | "~" unary | "-" unary | "+" unary | primary
    fn eval_unary(&mut self) -> i64 {
        self.skip_ws();
        if self.peek() == Some('!') {
            self.advance();
            let val = self.eval_unary();
            return if val == 0 { 1 } else { 0 };
        }
        if self.peek() == Some('~') {
            self.advance();
            return !self.eval_unary();
        }
        if self.peek() == Some('-') {
            self.advance();
            return -self.eval_unary();
        }
        if self.peek() == Some('+') {
            self.advance();
            return self.eval_unary();
        }
        self.eval_primary()
    }

    // primary = number | "(" expr ")" | "defined" ident | "defined" "(" ident ")" | ident | char_literal
    fn eval_primary(&mut self) -> i64 {
        self.skip_ws();
        if let Some(ch) = self.peek() {
            if ch == '(' {
                self.advance();
                let val = self.eval_expr();
                self.skip_ws();
                if self.peek() == Some(')') { self.advance(); }
                return val;
            }
            if ch == '\'' {
                // Character literal
                self.advance();
                let c = if self.pos < self.input.len() {
                    let ch = self.input[self.pos];
                    self.pos += 1;
                    if ch == '\\' && self.pos < self.input.len() {
                        let esc = self.input[self.pos];
                        self.pos += 1;
                        match esc {
                            'n' => '\n' as i64,
                            't' => '\t' as i64,
                            '0' => 0,
                            _ => esc as i64,
                        }
                    } else {
                        ch as i64
                    }
                } else {
                    0
                };
                if self.pos < self.input.len() && self.input[self.pos] == '\'' {
                    self.pos += 1;
                }
                return c;
            }
            if ch.is_ascii_digit() {
                return self.read_number();
            }
            if ch.is_ascii_alphabetic() || ch == '_' {
                let ident = self.read_ident();
                if ident == "defined" {
                    self.skip_ws();
                    let has_paren = self.peek() == Some('(');
                    if has_paren { self.advance(); }
                    self.skip_ws();
                    let name = self.read_ident();
                    if has_paren {
                        self.skip_ws();
                        if self.peek() == Some(')') { self.advance(); }
                    }
                    return if self.macros.contains_key(&name) { 1 } else { 0 };
                }
                // Check if it's a macro
                if let Some(MacroDef::Object(val)) = self.macros.get(&ident) {
                    let mut sub_eval = CondEval::new(val, self.macros);
                    return sub_eval.eval_expr();
                }
                // Unknown identifier in preprocessor expression = 0
                return 0;
            }
        }
        0
    }
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
