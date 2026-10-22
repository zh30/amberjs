//! Contract-registry consistency gate (fail-closed).
//!
//! A Stable contract is four wirings: a `docs/*_CONTRACT*.md` page, a
//! `tests/*_contract_tests.rs` file, a named `--test <stem>` step in
//! `.github/workflows/ci.yml`, and a CURRENT_SCOPE entry. If any wiring is
//! dropped the contract silently stops being exercised. This file asserts the
//! registry stays coherent so a contract PR cannot merge with a dangling
//! piece (e.g. `inspect_contract_tests` once had scope+file but no CI step).

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Stems of every `tests/*_contract_tests.rs` on disk.
fn contract_test_stems() -> Vec<String> {
    let mut stems: Vec<String> = fs::read_dir(root().join("tests"))
        .expect("tests/ must be readable")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with("_contract_tests.rs"))
        .map(|n| n.trim_end_matches(".rs").to_string())
        .collect();
    stems.sort();
    stems
}

/// Every contract test file must be wired to a named `--test <stem>` step in
/// ci.yml — otherwise the contract is never exercised on its own in CI.
#[test]
fn contract_files_have_named_ci_steps() {
    let ci = read(&root().join(".github/workflows/ci.yml"));
    for stem in contract_test_stems() {
        assert!(
            ci.contains(&format!("--test {stem} ")),
            "tests/{stem}.rs has no `--test {stem}` step in ci.yml — \
             the contract is silently unrun"
        );
    }
}

/// Reverse direction: a `--test <stem>` step whose file was deleted is a dead
/// CI step that cargo reports as "no test target" — catch it at test time.
#[test]
fn ci_test_steps_resolve_to_files() {
    let ci = read(&root().join(".github/workflows/ci.yml"));
    for m in ci.match_indices("--test ") {
        let rest = &ci[m.0 + 7..];
        let stem: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if stem.is_empty() {
            continue;
        }
        assert!(
            root().join(format!("tests/{stem}.rs")).exists(),
            "ci.yml runs `--test {stem}` but tests/{stem}.rs does not exist"
        );
    }
}

/// CURRENT_SCOPE's "Executable tests:" enumeration must cover every contract
/// file on disk, and every `tests/<stem>.rs` it names must exist — the page
/// is the capability truth, so stale names there are fiction.
#[test]
fn scope_page_and_disk_agree() {
    let scope = read(&root().join("docs/CURRENT_SCOPE.md"));
    let named: Vec<String> = scope
        .match_indices("tests/")
        .filter_map(|(i, _)| {
            let rest = &scope[i + 6..];
            let stem: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            rest[stem.len()..].strip_prefix(".rs").map(|_| stem)
        })
        .collect();
    for stem in &named {
        assert!(
            root().join(format!("tests/{stem}.rs")).exists(),
            "CURRENT_SCOPE names tests/{stem}.rs but the file is gone"
        );
    }
    for stem in contract_test_stems() {
        assert!(
            named.contains(&stem),
            "tests/{stem}.rs is not listed in CURRENT_SCOPE's \
             Executable tests enumeration"
        );
    }
}

/// Every `](NAME.md)` link on the scope page must resolve under `docs/` —
/// a dead contract-doc link is an unverifiable capability claim.
#[test]
fn scope_doc_links_resolve() {
    let scope = read(&root().join("docs/CURRENT_SCOPE.md"));
    for m in scope.match_indices("](") {
        let rest = &scope[m.0 + 2..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !rest[name.len()..].starts_with(".md)") {
            continue;
        }
        let name = format!("{name}.md");
        assert!(
            root().join("docs").join(&name).exists(),
            "CURRENT_SCOPE links docs/{name} but the file is missing"
        );
    }
}
