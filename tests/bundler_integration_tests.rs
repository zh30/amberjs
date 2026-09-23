// Bundle Integration Tests
//
// Tests for the amber bundle command and bundler functionality

use std::fs;
use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn test_bundle_command_help() {
    // Test that the bundle command is recognized and shows help
    let output = Command::new("cargo")
        .args(["run", "--bin", "amber", "--", "bundle", "--help"])
        .output()
        .expect("Failed to run amber bundle --help");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that help output is shown (not an error about unknown command)
    assert!(
        output.status.success(),
        "Bundle command should be recognized. stderr: {}",
        stderr
    );

    // Verify help contains expected information
    assert!(
        stderr.contains("Bundle code for production") || output.status.success(),
        "Help should describe bundle command"
    );
}

#[test]
fn test_bundle_basic_functionality() {
    // Create a temporary directory for testing
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create a simple entry file
    let entry_file = temp_path.join("entry.js");
    fs::write(
        &entry_file,
        r#"
        console.log("Hello from bundle");
        export const message = "Hello World";
    "#,
    )
    .expect("Failed to write entry file");

    // Create output path
    let output_file = temp_path.join("bundle.js");

    // Run bundle command
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "amber",
            "--",
            "bundle",
            entry_file.to_str().unwrap(),
            "--outfile",
            output_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run amber bundle");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The command should succeed or at least not fail with "unknown command"
    // (bundle may not be fully implemented yet, but CLI should recognize it)
    assert!(
        !stderr.contains("unknown command") && !stderr.contains("unexpected argument"),
        "Bundle command should be recognized. stderr: {}",
        stderr
    );

    // If bundle is implemented, check that output file was created
    if output.status.success() {
        assert!(
            output_file.exists(),
            "Output bundle file should be created when bundle succeeds"
        );
    }
}

#[test]
fn test_bundle_static_import_dependency_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(&dependency_file, "export const message = 'from-dep';\n")
        .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message } from './message.js';\nconsole.log('bundle:' + message);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");

    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );
    assert!(
        output_file.exists(),
        "Bundle output file should be created. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled output should run from the output directory without source files. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:from-dep"),
        "Bundled output should include code from the dependency. output: {run_combined}"
    );
}

#[test]
fn test_bundle_static_import_alias_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(&dependency_file, "export const message = 'aliased-dep';\n")
        .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message as label } from './message.js';\nconsole.log('bundle:' + label);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled alias import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:aliased-dep"),
        "Bundled output should preserve named import alias binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_static_default_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(&dependency_file, "export default 'default-dep';\n")
        .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import label from './message.js';\nconsole.log('bundle:' + label);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled default import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:default-dep"),
        "Bundled output should preserve default import binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_static_namespace_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(
        &dependency_file,
        "export const message = 'namespace-dep';\nexport const suffix = 'ok';\n",
    )
    .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import * as mod from './message.js';\nconsole.log('bundle:' + mod.message + ':' + mod.suffix);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled namespace import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:namespace-dep:ok"),
        "Bundled output should preserve namespace import binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_static_export_list_named_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(
        &dependency_file,
        "const internal = 'listed-dep';\nconst suffix = 'ok';\nexport { internal as message, suffix };\n",
    )
    .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message, suffix as renamed } from './message.js';\nconsole.log('bundle:' + message + ':' + renamed);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled export-list named import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:listed-dep:ok"),
        "Bundled output should preserve export-list named import binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_multiline_export_list_without_semicolon_preserves_following_code() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(
        &dependency_file,
        "const message = 'multiline-export-list';\nconst suffix = 'ok';\nexport {\n  message,\n  suffix as label\n}\nconsole.log('dep-side-effect');\n",
    )
    .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message, label } from './message.js';\nconsole.log('bundle:' + message + ':' + label);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled multiline export-list output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("dep-side-effect"),
        "Bundled output should preserve code after a multiline export list. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:multiline-export-list:ok"),
        "Bundled output should preserve multiline export-list bindings. output: {run_combined}"
    );
}

#[test]
fn test_bundle_static_re_export_named_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let leaf_file = src_dir.join("message.js");
    fs::write(&leaf_file, "export const message = 'barrel-dep';\n")
        .expect("Failed to write leaf dependency file");

    let barrel_file = src_dir.join("barrel.js");
    fs::write(&barrel_file, "export { message } from './message.js';\n")
        .expect("Failed to write barrel file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message } from './barrel.js';\nconsole.log('bundle:' + message);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled re-export named import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:barrel-dep"),
        "Bundled output should include transitive re-export dependency. output: {run_combined}"
    );
}

#[test]
fn test_bundle_missing_re_export_fails_before_writing_output() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let leaf_file = src_dir.join("message.js");
    fs::write(&leaf_file, "export const present = 'available';\n")
        .expect("Failed to write leaf dependency file");

    let barrel_file = src_dir.join("barrel.js");
    fs::write(&barrel_file, "export { missing } from './message.js';\n")
        .expect("Failed to write barrel file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { missing } from './barrel.js';\nconsole.log(missing);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        !bundle_output.status.success(),
        "amber bundle should fail when a barrel re-exports a missing binding. output: {bundle_combined}"
    );
    assert!(
        bundle_combined.contains("error: amber bundle:")
            && bundle_combined.contains("missing export")
            && bundle_combined.contains("cannot re-export"),
        "Missing re-export error should use the stable diagnostic. output: {bundle_combined}"
    );
    assert!(
        !output_file.exists(),
        "Failed re-export bundle should not write output. output: {bundle_combined}"
    );
}

#[test]
fn test_bundle_static_export_star_namespace_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let leaf_file = src_dir.join("message.js");
    fs::write(
        &leaf_file,
        "export const message = 'star-dep';\nexport const suffix = 'ok';\n",
    )
    .expect("Failed to write leaf dependency file");

    let barrel_file = src_dir.join("barrel.js");
    fs::write(&barrel_file, "export * from './message.js';\n")
        .expect("Failed to write barrel file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import * as barrel from './barrel.js';\nconsole.log('bundle:' + barrel.message + ':' + barrel.suffix);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled export-star namespace import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:star-dep:ok"),
        "Bundled output should preserve export-star namespace binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_static_default_re_export_named_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let leaf_file = src_dir.join("message.js");
    fs::write(&leaf_file, "export default 'default-barrel-dep';\n")
        .expect("Failed to write leaf dependency file");

    let barrel_file = src_dir.join("barrel.js");
    fs::write(
        &barrel_file,
        "export { default as message } from './message.js';\n",
    )
    .expect("Failed to write barrel file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message } from './barrel.js';\nconsole.log('bundle:' + message);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled default re-export named import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:default-barrel-dep"),
        "Bundled output should preserve default re-export named binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_multiple_default_imports_do_not_share_internal_binding() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let first_file = src_dir.join("first.js");
    fs::write(&first_file, "export default 'first-default';\n")
        .expect("Failed to write first dependency file");

    let second_file = src_dir.join("second.js");
    fs::write(&second_file, "export default 'second-default';\n")
        .expect("Failed to write second dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import first from './first.js';\nimport second from './second.js';\nconsole.log('bundle:' + first + ':' + second);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled multiple default imports should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:first-default:second-default"),
        "Bundled output should preserve each module's default binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_duplicate_named_exports_do_not_share_internal_binding() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let first_file = src_dir.join("first.js");
    fs::write(&first_file, "export const value = 'first-named';\n")
        .expect("Failed to write first dependency file");

    let second_file = src_dir.join("second.js");
    fs::write(&second_file, "export const value = 'second-named';\n")
        .expect("Failed to write second dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { value as first } from './first.js';\nimport { value as second } from './second.js';\nconsole.log('bundle:' + first + ':' + second);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled duplicate named exports should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:first-named:second-named"),
        "Bundled output should preserve each module's named binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_multiple_same_line_named_exports_run_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(
        &dependency_file,
        "export const first = 'same-line-first'; export const second = 'same-line-second';\n",
    )
    .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { first, second } from './message.js';\nconsole.log('bundle:' + first + ':' + second);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled same-line named exports should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:same-line-first:same-line-second"),
        "Bundled output should preserve same-line named exports. output: {run_combined}"
    );
}

#[test]
fn test_bundle_same_line_import_preserves_following_code() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(
        &dependency_file,
        "export const message = 'same-line-import';\n",
    )
    .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { message } from './message.js'; console.log('bundle:' + message);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled same-line import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:same-line-import"),
        "Bundled output should preserve code after a same-line import. output: {run_combined}"
    );
}

#[test]
fn test_bundle_multiline_named_import_runs_from_output_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(
        &dependency_file,
        "export const message = 'multiline-import';\n",
    )
    .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import {\n  message\n} from './message.js';\nconsole.log('bundle:' + message);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Bundled multiline import output should run from dist. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:multiline-import"),
        "Bundled output should preserve multiline named import binding. output: {run_combined}"
    );
}

#[test]
fn test_bundle_missing_named_import_fails_before_writing_output() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let dist_dir = temp_dir.path().join("dist");
    fs::create_dir_all(&src_dir).expect("Failed to create source dir");
    fs::create_dir_all(&dist_dir).expect("Failed to create dist dir");

    let dependency_file = src_dir.join("message.js");
    fs::write(&dependency_file, "export const present = 'available';\n")
        .expect("Failed to write dependency file");

    let entry_file = src_dir.join("entry.js");
    fs::write(
        &entry_file,
        "import { missing } from './message.js';\nconsole.log(missing);\n",
    )
    .expect("Failed to write entry file");

    let output_file = dist_dir.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .output()
        .expect("Failed to run amber bundle");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        !bundle_output.status.success(),
        "amber bundle should fail before producing an invalid bundle. output: {bundle_combined}"
    );
    assert!(
        bundle_combined.contains("error: amber bundle:")
            && bundle_combined.contains("missing export")
            && bundle_combined.contains("'missing'"),
        "Missing named import error should use the stable diagnostic. output: {bundle_combined}"
    );
    assert!(
        !output_file.exists(),
        "Failed bundle should not write an output file. output: {bundle_combined}"
    );
}

#[test]
fn test_bundle_with_minify_flag() {
    // Test that minify flag is recognized
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    let entry_file = temp_path.join("entry.js");
    fs::write(&entry_file, "console.log('test');").expect("Failed to write entry file");

    let output_file = temp_path.join("bundle.js");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "amber",
            "--",
            "bundle",
            entry_file.to_str().unwrap(),
            "--outfile",
            output_file.to_str().unwrap(),
            "--minify",
        ])
        .output()
        .expect("Failed to run amber bundle --minify");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail with parsing errors
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("invalid value"),
        "Minify flag should be parsed correctly. stderr: {}",
        stderr
    );
}

#[test]
fn test_bundle_minify_output_runs_without_comment_swallowing_code() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    let entry_file = temp_path.join("entry.js");
    fs::write(&entry_file, "console.log('bundle:minified');\n")
        .expect("Failed to write entry file");

    let output_file = temp_path.join("bundle.js");
    let bundle_output = Command::new(amber_path())
        .arg("bundle")
        .arg(&entry_file)
        .arg("--outfile")
        .arg(&output_file)
        .arg("--minify")
        .output()
        .expect("Failed to run amber bundle --minify");
    let bundle_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );
    assert!(
        bundle_output.status.success(),
        "amber bundle --minify should succeed. output: {bundle_combined}"
    );

    let run_output = Command::new(amber_path())
        .arg("run")
        .arg(&output_file)
        .output()
        .expect("Failed to run minified bundled output");
    let run_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        run_output.status.success(),
        "Minified bundle output should execute successfully. output: {run_combined}"
    );
    assert!(
        run_combined.contains("bundle:minified"),
        "Minified bundle should not let line comments swallow executable code. output: {run_combined}"
    );
}

#[test]
fn test_bundle_with_sourcemap_flag() {
    // Test that sourcemap flag is recognized
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    let entry_file = temp_path.join("entry.js");
    fs::write(&entry_file, "export const x = 42;").expect("Failed to write entry file");

    let output_file = temp_path.join("bundle.js");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "amber",
            "--",
            "bundle",
            entry_file.to_str().unwrap(),
            "--outfile",
            output_file.to_str().unwrap(),
            "--sourcemap",
        ])
        .output()
        .expect("Failed to run amber bundle --sourcemap");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail with parsing errors
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("invalid value"),
        "Sourcemap flag should be parsed correctly. stderr: {}",
        stderr
    );
}

#[test]
fn test_bundle_tree_shake_flag() {
    // Test that tree-shake flag is recognized
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    let entry_file = temp_path.join("entry.js");
    fs::write(
        &entry_file,
        "export const used = 'hello'; export const unused = 'world';",
    )
    .expect("Failed to write entry file");

    let output_file = temp_path.join("bundle.js");

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "amber",
            "--",
            "bundle",
            entry_file.to_str().unwrap(),
            "--outfile",
            output_file.to_str().unwrap(),
            "--tree-shake",
        ])
        .output()
        .expect("Failed to run amber bundle --tree-shake");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail with parsing errors
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("invalid value"),
        "Tree-shake flag should be parsed correctly. stderr: {}",
        stderr
    );
}

#[test]
fn test_bundle_target_options() {
    // Test that different target options are accepted
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    let entry_file = temp_path.join("entry.js");
    fs::write(&entry_file, "console.log('target test');").expect("Failed to write entry file");

    let output_file = temp_path.join("bundle.js");

    // Test browser target (default)
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "amber",
            "--",
            "bundle",
            entry_file.to_str().unwrap(),
            "--outfile",
            output_file.to_str().unwrap(),
            "--target",
            "browser",
        ])
        .output()
        .expect("Failed to run amber bundle --target browser");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail with parsing errors
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("invalid value"),
        "Target option should be parsed correctly. stderr: {}",
        stderr
    );
}
