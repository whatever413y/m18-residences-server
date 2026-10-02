//! Keeps each domain liftable into its own service: a domain uses only itself,
//! `crate::app` and the shared crates, writes only the tables it owns, and stays
//! free of the Workers runtime. Checked on the source text (no database).
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Each domain and the tables it owns (the only ones it may write).
const DOMAINS: &[(&str, &[&str])] = &[
    ("accounts", &[]),
    ("property", &["room", "tenant", "electricity_reading"]),
    ("billing", &["bill", "additional_charge"]),
];
const TABLES: &[&str] = &[
    "room",
    "tenant",
    "electricity_reading",
    "bill",
    "additional_charge",
];
/// Folders under `src/` that are not domains.
const NOT_DOMAINS: &[&str] = &[];
/// Files directly under `src/` that are not domains (left over legacy layer folders are listed by the test failure).
const ROOT_FILES: &[&str] = &["lib.rs", "app.rs", "worker_entry.rs"];

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    files
}

/// The source without `//` comments (doc comments included), so prose can name anything.
fn code(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rel(path: &Path) -> String {
    path.strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn src_holds_only_known_domains() {
    let known: Vec<&str> = DOMAINS
        .iter()
        .map(|(d, _)| *d)
        .chain(NOT_DOMAINS.iter().copied())
        .collect();
    for entry in fs::read_dir(src()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if entry.path().is_dir() {
            assert!(
                known.contains(&name.as_str()),
                "src/{name}/ is not a registered domain (add it to DOMAINS in tests/boundaries.rs)"
            );
        } else {
            assert!(
                ROOT_FILES.contains(&name.as_str()),
                "unexpected file src/{name}"
            );
        }
    }
}

#[test]
fn domains_only_use_themselves_app_and_shared_crates() {
    let mut problems = Vec::new();
    for (domain, _) in DOMAINS {
        for file in rust_files(&src().join(domain)) {
            let code = code(&file);
            let mut rest = code.as_str();
            while let Some(i) = rest.find("crate::") {
                let after = &rest[i + "crate::".len()..];
                let allowed = |path: &str| {
                    [domain, &"app"]
                        .iter()
                        .any(|root| path == **root || path.starts_with(&format!("{root}::")))
                };
                let ok = match after.strip_prefix('{') {
                    // `crate::{a::b, c}`: every top-level item must be allowed.
                    Some(group) => top_level_items(group).iter().all(|item| allowed(item)),
                    None => allowed(after),
                };
                if !ok {
                    problems.push(format!(
                        "{}: `crate::{}`",
                        rel(&file),
                        after.chars().take(40).collect::<String>()
                    ));
                }
                rest = after;
            }
            if code.contains("super::super") {
                problems.push(format!("{}: `super::super`", rel(&file)));
            }
            if code.contains("#[path") {
                problems.push(format!("{}: #[path]", rel(&file)));
            }
            for (other, _) in DOMAINS.iter().filter(|(d, _)| d != domain) {
                if code.contains(&format!("m18_residences_server::{other}")) {
                    problems.push(format!("{}: uses the {other} domain", rel(&file)));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "domain boundary violations:\n{}",
        problems.join("\n")
    );
}

#[test]
fn domains_write_only_their_own_tables() {
    let mut problems = Vec::new();
    for (domain, owned) in DOMAINS {
        for file in rust_files(&src().join(domain)) {
            let code = code(&file);
            let read_only = file
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with("_read_repo.rs");
            for table in TABLES {
                let writes = [
                    format!("{table}::ActiveModel"),
                    format!("{table}::Entity::insert"),
                    format!("{table}::Entity::update"),
                    format!("{table}::Entity::delete"),
                    format!("INSERT INTO {table}"),
                    format!("UPDATE {table}"),
                    format!("DELETE FROM {table}"),
                ];
                if let Some(write) = writes.iter().find(|w| code.contains(w.as_str())) {
                    if read_only {
                        problems.push(format!(
                            "{}: read-only repository writes ({write})",
                            rel(&file)
                        ));
                    } else if !owned.contains(table) {
                        problems.push(format!(
                            "{}: writes {table}, owned by another domain ({write})",
                            rel(&file)
                        ));
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "table ownership violations:\n{}",
        problems.join("\n")
    );
}

#[test]
fn domains_stay_portable_to_the_workers_runtime() {
    let banned = [
        (
            "worker::",
            "the Workers runtime belongs in src/worker_entry.rs and the shared crates",
        ),
        (
            "println!",
            "use log_ok!/log_warn!/log_error! (stdout is lost on Workers)",
        ),
        (
            "eprintln!",
            "use log_ok!/log_warn!/log_error! (stderr is lost on Workers)",
        ),
        ("SystemTime", "panics on wasm32; use chrono::Utc::now()"),
        ("Instant::now", "panics on wasm32; use chrono::Utc::now()"),
        ("tokio::spawn", "no Tokio runtime on Workers"),
        (
            ".begin()",
            "D1 has no interactive transactions; use Db::atomic",
        ),
        (
            ".transaction",
            "D1 has no interactive transactions; use Db::atomic",
        ),
    ];
    let mut problems = Vec::new();
    for (domain, _) in DOMAINS {
        for file in rust_files(&src().join(domain)) {
            let code = code(&file);
            for (needle, why) in banned {
                if code.contains(needle) {
                    problems.push(format!("{}: `{needle}` ({why})", rel(&file)));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "runtime portability violations:\n{}",
        problems.join("\n")
    );
}

/// The comma-separated items of a `{...}` group (given the text after `{`), up to its closing brace.
fn top_level_items(group: &str) -> Vec<String> {
    let mut items = vec![String::new()];
    let mut depth = 0;
    for c in group.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => break,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(String::new());
                continue;
            }
            _ => {}
        }
        items.last_mut().unwrap().push(c);
    }
    items
        .into_iter()
        .map(|item| item.split_whitespace().collect::<String>())
        .filter(|item| !item.is_empty())
        .collect()
}
