//! Project-specific Rust lints for the Inertia workspace.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");

    let mut violations = Vec::new();
    scan_dir(&workspace_root.join("crates"), &mut violations);

    if violations.is_empty() {
        println!("inertia-lint: ok");
        return ExitCode::SUCCESS;
    }

    eprintln!("inertia-lint: found {} violation(s)\n", violations.len());
    for violation in &violations {
        eprintln!("{violation}");
    }
    eprintln!(
        "\nUse column names with rusqlite row access, e.g. row.get(\"display_name\")? \
         instead of row.get(0)?."
    );
    ExitCode::FAILURE
}

fn scan_dir(dir: &Path, violations: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }

        if path.extension().is_some_and(|ext| ext == "rs") {
            lint_file(&path, violations);
        }
    }
}

fn lint_file(path: &Path, violations: &mut Vec<String>) {
    let contents = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });

    for (line_number, line) in contents.lines().enumerate() {
        if let Some(index) = find_numeric_row_get(line) {
            let relative = path
                .strip_prefix(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../..")
                        .canonicalize()
                        .expect("workspace root"),
                )
                .unwrap_or(path);

            violations.push(format!(
                "{}:{}: row.get({index}) — use row.get(\"column_name\") instead\n    {}",
                relative.display(),
                line_number + 1,
                line.trim()
            ));
        }
    }
}

fn find_numeric_row_get(line: &str) -> Option<u32> {
    let trimmed = line.split("//").next()?;
    let mut search_from = 0;

    while let Some(start) = trimmed[search_from..].find("row.get") {
        let after = search_from + start + "row.get".len();
        let rest = trimmed.get(after..)?;
        let args = skip_generics_and_whitespace(rest)?;
        if let Some(index) = parse_numeric_arg(args)? {
            return Some(index);
        }
        search_from = after + 1;
    }

    None
}

fn skip_generics_and_whitespace(input: &str) -> Option<&str> {
    let mut rest = input;
    while rest.starts_with(' ') {
        rest = &rest[1..];
    }

    if rest.starts_with("::<") {
        let end = rest.find('>')?;
        rest = &rest[end + 1..];
        while rest.starts_with(' ') {
            rest = &rest[1..];
        }
    }

    if !rest.starts_with('(') {
        return None;
    }

    let args = &rest[1..];
    let close = args.find(')')?;
    Some(&args[..close])
}

fn parse_numeric_arg(args: &str) -> Option<Option<u32>> {
    let mut chars = args.chars().peekable();
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }

    if let Some(digit) = chars.next() {
        if digit.is_ascii_digit() {
            let mut index = digit.to_digit(10)?;
            while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                index = index * 10 + chars.next()?.to_digit(10)?;
            }
            return Some(Some(index));
        }
    }

    Some(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_numeric_row_get() {
        assert_eq!(find_numeric_row_get("let x = row.get(0)?;"), Some(0));
        assert_eq!(
            find_numeric_row_get("row.get::<_, String>(3)?"),
            Some(3)
        );
        assert_eq!(
            find_numeric_row_get("row.get(\"display_name\")?"),
            None
        );
        assert_eq!(
            find_numeric_row_get("row.get(0)? // row.get(\"allowed\")"),
            Some(0)
        );
    }
}
