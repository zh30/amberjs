// Pins the Stable `amber compile` SEA contract in docs/COMPILE_CONTRACT.md.
// CI runs this target explicitly so the written contract cannot silently regress.

use amberjs::tooling::compiler::{
    compile_binary, find_standalone_trailer, read_standalone_payload, COMPILE_ERROR_PREFIX,
    MAGIC_TRAILER, STANDALONE_ERROR_PREFIX, TRAILER_TOTAL_SIZE,
};
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn install_mutated_sea(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("write mutated sea");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    // The macOS trailer sits inside the ad-hoc signature. A mutated copy is
    // killed by the kernel unless it is signed again; signing does not change
    // the `__AMBER` bytes the runtime reads.
    #[cfg(target_os = "macos")]
    {
        let signed = Command::new("codesign")
            .args(["--sign", "-", "--force"])
            .arg(path)
            .output()
            .expect("codesign");
        assert!(
            signed.status.success(),
            "re-sign mutated SEA: {}",
            String::from_utf8_lossy(&signed.stderr)
        );
    }
}

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture");
}

fn combined(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_stable_compile_error(output: &std::process::Output, needles: &[&str]) {
    let text = combined(output);
    assert!(
        !output.status.success(),
        "expected amber compile to fail. output: {text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with(COMPILE_ERROR_PREFIX)),
        "failure must start a line with {COMPILE_ERROR_PREFIX}. output: {text}"
    );
    for needle in needles {
        assert!(
            text.contains(needle),
            "failure must contain {needle:?}. output: {text}"
        );
    }
}

fn output_name(dir: &Path, stem: &str) -> std::path::PathBuf {
    if cfg!(windows) {
        dir.join(format!("{stem}.exe"))
    } else {
        dir.join(stem)
    }
}

fn run_compile(args: &[&str]) -> std::process::Output {
    Command::new(amber_path())
        .arg("compile")
        .args(args)
        .output()
        .expect("failed to execute amber compile")
}

#[test]
fn contract_library_missing_entry_uses_stable_prefix() {
    let err = compile_binary(
        Path::new("/definitely/missing/amber-compile-contract.js"),
        Path::new("/tmp/amber-compile-contract-should-not-matter"),
    )
    .expect_err("missing entry")
    .to_string();
    assert!(err.starts_with(COMPILE_ERROR_PREFIX), "{err}");
    assert!(err.contains("entry file not found"), "{err}");
}

#[test]
fn contract_entry_missing_uses_stable_diagnostic() {
    let dir = tempdir().expect("tempdir");
    let missing = dir.path().join("nope.js");
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[missing.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["entry file not found"]);
    assert!(!outfile.exists(), "failed compile must not write output");
}

#[test]
fn contract_directory_entry_fails() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("src");
    fs::create_dir(&entry).expect("mkdir");
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["entry must be a file"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_relative_unresolved_fails_before_write() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    write(
        &entry,
        "import { x } from './missing.js';\nconsole.log(x);\n",
    );
    let outfile = dir
        .path()
        .join("dist")
        .join(if cfg!(windows) { "app.exe" } else { "app" });
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["cannot resolve", "./missing.js"]);
    assert!(!outfile.exists(), "failed relative resolve must not write");
}

#[test]
fn contract_unsupported_asset_fails() {
    let dir = tempdir().expect("tempdir");
    write(&dir.path().join("app.css"), "body{}\n");
    let entry = dir.path().join("entry.js");
    write(&entry, "import './app.css';\n");
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["unsupported module", "app.css"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_native_addon_is_rejected() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("addon.node"), [0u8, 1, 2, 3]).expect("addon");
    let entry = dir.path().join("entry.js");
    write(&entry, "require('./addon.node');\n");
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["native addon", "./addon.node"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_dynamic_import_is_rejected_even_if_the_file_exists() {
    let dir = tempdir().expect("tempdir");
    write(&dir.path().join("extra.js"), "export const n = 1;\n");
    let entry = dir.path().join("entry.js");
    write(
        &entry,
        "const mod = import('./extra.js');\nconsole.log(mod);\n",
    );
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["dynamic import() is not embedded"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_computed_require_is_rejected() {
    let dir = tempdir().expect("tempdir");
    write(&dir.path().join("extra.js"), "module.exports = 1;\n");
    let entry = dir.path().join("entry.js");
    write(
        &entry,
        "const name = './extra.js';\nconsole.log(require(name));\n",
    );
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["computed require() is not embedded"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_absolute_specifier_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    let specifier = if cfg!(windows) {
        "C:/absolutely-not-embedded.js"
    } else {
        "/absolutely-not-embedded.js"
    };
    write(&entry, &format!("require('{specifier}');\n"));
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["absolute module specifiers are not embedded"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_missing_export_is_bundle_failure_without_output() {
    let dir = tempdir().expect("tempdir");
    write(&dir.path().join("dep.js"), "export const present = 1;\n");
    let entry = dir.path().join("entry.js");
    write(
        &entry,
        "import { absent } from './dep.js';\nconsole.log(absent);\n",
    );
    let outfile = output_name(dir.path(), "out");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["bundle failed", "absent"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_output_directory_fails() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    write(&entry, "console.log(1);\n");
    let outfile = dir.path().join("outdir");
    fs::create_dir(&outfile).expect("mkdir");
    let output = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert_stable_compile_error(&output, &["output path is a directory"]);
}

#[test]
fn contract_refuses_to_overwrite_runtime_binary() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    write(&entry, "console.log(1);\n");
    let amber = amber_path();
    let output = run_compile(&[entry.to_str().unwrap(), "-o", amber]);
    assert_stable_compile_error(&output, &["refusing to overwrite the Amber runtime binary"]);
    let version = Command::new(amber)
        .arg("--version")
        .output()
        .expect("amber still runs");
    assert!(
        version.status.success(),
        "runtime binary must survive a refused overwrite: {}",
        combined(&version)
    );
}

#[test]
fn contract_env_var_does_not_select_standalone_mode() {
    let output = Command::new(amber_path())
        .env("AMBER_STANDALONE", "1")
        .args(["eval", "1 + 1"])
        .output()
        .expect("eval");
    let text = combined(&output);
    assert!(output.status.success(), "{text}");
    assert!(text.contains('2'), "{text}");
    assert!(
        !text.contains(STANDALONE_ERROR_PREFIX),
        "AMBER_STANDALONE is not an environment switch: {text}"
    );
}

#[test]
#[serial]
fn contract_sea_runs_graph_and_skips_cli() {
    let dir = tempdir().expect("tempdir");
    write(
        &dir.path().join("calc.ts"),
        "export function compute(x: number, y: number): number { return x + y; }\n",
    );
    write(
        &dir.path().join("label.tsx"),
        "export function label(name: string): string { return 'L:' + name; }\n",
    );
    write(&dir.path().join("config.json"), "{ \"name\": \"sea\" }\n");
    write(
        &dir.path().join("legacy.cjs"),
        "module.exports = { tag: 'cjs' };\n",
    );
    let entry = dir.path().join("entry.ts");
    write(
        &entry,
        r#"
import { compute } from "./calc.ts";
import { label } from "./label.tsx";
import config from "./config.json";
const legacy = require("./legacy.cjs");
const path = require("path");
const args = process.argv.slice(2).join(",");
console.log(`SEA_CONTRACT:${compute(2, 3)}:${label("A")}:${config.name}:${legacy.tag}:${path.basename("file.txt")}:${args}`);
"#,
    );
    let outfile = dir
        .path()
        .join("dist")
        .join(if cfg!(windows) { "sea.exe" } else { "sea" });
    let compiled = run_compile(&[
        entry.to_str().unwrap(),
        "--output",
        outfile.to_str().unwrap(),
    ]);
    let text = combined(&compiled);
    assert!(compiled.status.success(), "compile should succeed: {text}");
    assert!(outfile.exists(), "compiled binary must exist");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&outfile).expect("meta").permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "SEA binary should be executable, mode {mode:o}"
        );
    }

    let bytes = fs::read(&outfile).expect("read sea");
    assert!(bytes.len() > TRAILER_TOTAL_SIZE as usize);
    let trailer = find_standalone_trailer(&bytes)
        .expect("locate")
        .expect("trailer");
    assert_eq!(trailer.flags, 0, "reserved flags must be written as 0");
    if cfg!(target_os = "macos") {
        assert_ne!(
            &bytes[bytes.len() - 16..],
            MAGIC_TRAILER,
            "macOS ad-hoc signature occupies the end of the file"
        );
    } else {
        assert_eq!(&bytes[bytes.len() - 16..], MAGIC_TRAILER);
        assert_eq!(trailer.trailer_end, bytes.len());
    }
    let payload = std::str::from_utf8(
        &bytes[trailer.payload_start..trailer.payload_start + trailer.payload_len as usize],
    )
    .expect("utf8 payload");
    assert!(
        payload.contains("SEA_CONTRACT"),
        "payload must contain the bundled script"
    );
    let via_reader = read_standalone_payload(&outfile)
        .expect("reader")
        .expect("payload");
    assert!(via_reader.contains("SEA_CONTRACT"));

    let run = Command::new(&outfile)
        .args(["--version", "ok"])
        .output()
        .expect("run sea");
    let run_text = combined(&run);
    assert!(run.status.success(), "SEA run failed: {run_text}");
    assert!(
        run_text.contains("SEA_CONTRACT:5:L:A:sea:cjs:file.txt:--version,ok"),
        "stdout mismatch: {run_text}"
    );
    assert!(
        !run_text.contains("Usage:"),
        "SEA boot must skip the amber CLI parser: {run_text}"
    );
}

#[test]
#[serial]
fn contract_script_failure_uses_standalone_diagnostic() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("boom.js");
    write(&entry, "throw new Error('sea-boom');\n");
    let outfile = output_name(dir.path(), "boom");
    let compiled = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert!(
        compiled.status.success(),
        "compile of a throwing script should still succeed: {}",
        combined(&compiled)
    );
    let run = Command::new(&outfile).output().expect("run");
    let text = combined(&run);
    assert!(!run.status.success(), "throw should be non-zero: {text}");
    assert!(
        text.lines()
            .any(|line| line.starts_with(STANDALONE_ERROR_PREFIX)),
        "script failure must use {STANDALONE_ERROR_PREFIX}: {text}"
    );
    assert!(text.contains("sea-boom"), "{text}");
}

#[test]
#[serial]
fn contract_corrupt_trailer_does_not_fall_through_to_cli() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("ok.js");
    write(&entry, "console.log('SEA_OK');\n");
    let outfile = output_name(dir.path(), "ok");
    let compiled = run_compile(&[entry.to_str().unwrap(), "-o", outfile.to_str().unwrap()]);
    assert!(compiled.status.success(), "{}", combined(&compiled));
    let original = fs::read(&outfile).expect("read");
    let trailer = find_standalone_trailer(&original)
        .expect("locate")
        .expect("trailer");
    let len_at = trailer.trailer_end - 32;
    let flags_at = trailer.trailer_end - 24;

    let bad_len = dir.path().join(if cfg!(windows) {
        "bad-len.exe"
    } else {
        "bad-len"
    });
    let mut bytes = original.clone();
    bytes[len_at..len_at + 8].copy_from_slice(&0u64.to_le_bytes());
    install_mutated_sea(&bad_len, &bytes);
    let run = Command::new(&bad_len).output().expect("run bad len");
    let text = combined(&run);
    assert!(!run.status.success(), "{text}");
    assert!(
        text.contains("invalid payload length"),
        "corrupt length must not fall through to the CLI: {text}"
    );
    assert!(!text.contains("Usage:"), "{text}");

    let bad_flags = dir.path().join(if cfg!(windows) {
        "bad-flags.exe"
    } else {
        "bad-flags"
    });
    let mut bytes = original.clone();
    bytes[flags_at..flags_at + 8].copy_from_slice(&7u64.to_le_bytes());
    install_mutated_sea(&bad_flags, &bytes);
    let run = Command::new(&bad_flags).output().expect("run bad flags");
    let text = combined(&run);
    assert!(!run.status.success(), "{text}");
    assert!(
        text.contains("unsupported trailer flags: 7"),
        "nonzero flags must fail closed: {text}"
    );

    let bad_utf8 = dir.path().join(if cfg!(windows) {
        "bad-utf8.exe"
    } else {
        "bad-utf8"
    });
    let mut bytes = original;
    bytes[trailer.payload_start] = 0xFF;
    install_mutated_sea(&bad_utf8, &bytes);
    let run = Command::new(&bad_utf8).output().expect("run bad utf8");
    let text = combined(&run);
    assert!(!run.status.success(), "{text}");
    assert!(
        text.contains("payload is not UTF-8"),
        "non-utf8 payload must fail closed: {text}"
    );
}

#[test]
#[serial]
fn contract_default_output_name() {
    let dir = tempdir().expect("tempdir");
    write(&dir.path().join("app.js"), "console.log('default-sea');\n");
    let compiled = Command::new(amber_path())
        .current_dir(dir.path())
        .args(["compile", "app.js"])
        .output()
        .expect("compile");
    let text = combined(&compiled);
    assert!(compiled.status.success(), "{text}");
    let expected = if cfg!(windows) {
        dir.path().join("app.exe")
    } else {
        dir.path().join("app")
    };
    assert!(
        expected.exists(),
        "default outfile should be the entry stem"
    );
    let run = Command::new(&expected).output().expect("run");
    let run_text = combined(&run);
    assert!(run.status.success(), "{run_text}");
    assert!(run_text.contains("default-sea"), "{run_text}");
}
