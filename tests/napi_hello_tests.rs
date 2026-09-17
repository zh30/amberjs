use serial_test::serial;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn bee() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn compile_hello_addon(out_dir: &std::path::Path) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("tests/fixtures/napi_hello/hello.c");
    let include = manifest.join("tests/fixtures/napi_hello");
    let dest = out_dir.join("hello.node");
    let mut cmd = Command::new("cc");
    cmd.arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(&dest)
        .arg(&src)
        .arg("-I")
        .arg(&include);
    if cfg!(target_os = "macos") {
        cmd.arg("-undefined").arg("dynamic_lookup");
    }
    let output = cmd.output().expect("cc");
    assert!(
        output.status.success(),
        "cc failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    dest
}

#[test]
#[serial]
fn process_dlopen_registers_hello_addon() {
    let dir = tempdir().unwrap();
    let addon = compile_hello_addon(dir.path());
    let script = dir.path().join("load.js");
    let script_body = format!(
        r#"
const path = {};
const module = {{ exports: {{}} }};
process.dlopen(module, path);
if (typeof module.exports.hello !== 'function') {{
  throw new Error('hello export missing: ' + JSON.stringify(Object.keys(module.exports)));
}}
const result = module.exports.hello();
if (result !== 'world') {{
  throw new Error('expected world, got ' + JSON.stringify(result));
}}
console.log(result);
"#,
        serde_json::to_string(&addon.to_string_lossy().as_ref()).unwrap()
    );
    std::fs::write(&script, script_body).unwrap();

    let output = Command::new(bee())
        .arg("run")
        .arg(&script)
        .output()
        .expect("bee run napi hello");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "napi hello failed: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.trim().contains("world"),
        "expected world on stdout: {stdout} {stderr}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn bee_dynsym_exports_napi_create_function() {
    let output = Command::new("nm")
        .args(["-D", "--defined-only", bee()])
        .output()
        .expect("nm -D");
    assert!(
        output.status.success(),
        "nm -D failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("napi_create_function"),
        "Linux dlopen of hello.node needs napi_create_function in bee dynsym: {stdout}"
    );
}
