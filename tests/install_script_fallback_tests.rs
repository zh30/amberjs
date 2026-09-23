use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn install_sh() -> PathBuf {
    repo_root().join("install.sh")
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

fn make_tarball(dir: &Path, bin_name: &str, contents: &str, dest: &Path) {
    write_executable(&dir.join(bin_name), contents);
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(dest)
        .arg("-C")
        .arg(dir)
        .arg(bin_name)
        .status()
        .expect("tar");
    assert!(status.success(), "tar -czf failed");
}

struct FixtureServer {
    child: Child,
    base: String,
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_fixture_server(root: &Path) -> FixtureServer {
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(
            r#"
import http.server, os, sys
os.chdir(sys.argv[1])
httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), http.server.SimpleHTTPRequestHandler)
print(httpd.server_address[1], flush=True)
httpd.serve_forever()
"#,
        )
        .arg(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("python3 http server");

    let stdout = child.stdout.take().expect("server stdout");
    let mut ready = BufReader::new(stdout);
    let mut line = String::new();
    if ready.read_line(&mut line).unwrap_or(0) == 0 {
        let _ = child.kill();
        panic!("fixture server exited before publishing its port");
    }
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("invalid fixture port: {line:?}"));

    FixtureServer {
        child,
        base: format!("http://127.0.0.1:{port}"),
    }
}

fn run_install(
    version: &str,
    os: &str,
    arch: &str,
    release_base: &str,
    install_dir: &Path,
    home: &Path,
) -> std::process::Output {
    Command::new("sh")
        .arg(install_sh())
        .env("AMBER_VERSION", version)
        .env("AMBER_UNAME_S", os)
        .env("AMBER_UNAME_M", arch)
        .env("AMBER_RELEASE_BASE", release_base)
        .env("AMBER_INSTALL_DIR", install_dir)
        .env("HOME", home)
        .env(
            "PATH",
            format!(
                "{}:{}",
                install_dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("run install.sh")
}

#[test]
fn print_asset_urls_prefers_amber_then_bee_for_apple_silicon() {
    let output = Command::new("sh")
        .arg(install_sh())
        .arg("--print-asset-urls")
        .env("AMBER_VERSION", "v1.16.0")
        .env("AMBER_UNAME_S", "Darwin")
        .env("AMBER_UNAME_M", "arm64")
        .env_remove("AMBER_RELEASE_BASE")
        .output()
        .expect("print-asset-urls");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "print-asset-urls failed: {stderr}{stdout}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "https://github.com/zh30/amberjs/releases/download/v1.16.0/amber-v1.16.0-aarch64-apple-darwin.tar.gz",
            "https://github.com/zh30/amberjs/releases/download/v1.16.0/bee-v1.16.0-aarch64-apple-darwin.tar.gz",
        ]
    );
}

#[test]
fn website_public_installers_match_repo_root() {
    for name in ["install.sh", "install.ps1"] {
        let root = fs::read_to_string(repo_root().join(name)).unwrap();
        let public =
            fs::read_to_string(repo_root().join("apps/website/public").join(name)).unwrap();
        assert_eq!(
            root, public,
            "{name} website public copy drifted from repo root"
        );
    }
}

#[test]
fn install_scripts_document_bee_fallback_and_final_amber_name() {
    let sh = fs::read_to_string(install_sh()).unwrap();
    assert!(sh.contains("bee-${version_tag}-${target}.tar.gz"));
    assert!(sh.contains("amber-${version_tag}-${target}.tar.gz"));
    assert!(sh.contains("-name bee"));
    assert!(
        sh.contains(r#"AMBER_INSTALL_DIR/amber"#) || sh.contains(r#"$AMBER_INSTALL_DIR/amber"#)
    );
    assert!(sh.contains("download failed (tried:"));
    assert!(sh.contains("set -e"));

    let ps1 = fs::read_to_string(repo_root().join("install.ps1")).unwrap();
    assert!(ps1.contains("amber-$Version-$Target.zip"));
    assert!(ps1.contains("bee-$Version-$Target.zip"));
    assert!(ps1.contains("bee.exe"));
    assert!(ps1.contains("amber.exe"));
    assert!(ps1.contains("download failed (tried:"));
}

#[test]
fn install_sh_falls_back_to_bee_archive_and_installs_as_amber() {
    let tmp = tempfile::tempdir().unwrap();
    let www = tmp.path().join("www");
    let version_dir = www.join("v1.16.0");
    fs::create_dir_all(&version_dir).unwrap();

    let stage = tmp.path().join("stage-bee");
    fs::create_dir_all(&stage).unwrap();
    make_tarball(
        &stage,
        "bee",
        "#!/bin/sh\necho bee-payload\n",
        &version_dir.join("bee-v1.16.0-aarch64-apple-darwin.tar.gz"),
    );

    let server = spawn_fixture_server(&www);
    let home = tmp.path().join("home");
    let install_dir = tmp.path().join("bin");
    fs::create_dir_all(&home).unwrap();

    let output = run_install(
        "v1.16.0",
        "Darwin",
        "arm64",
        &server.base,
        &install_dir,
        &home,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "install.sh fallback failed: {stderr}{stdout}"
    );
    assert!(
        stdout.contains("bee-v1.16.0-aarch64-apple-darwin.tar.gz"),
        "should download bee- URL after amber- 404: {stdout}"
    );
    assert!(
        stdout.contains("amber-v1.16.0-aarch64-apple-darwin.tar.gz"),
        "should try amber- first: {stdout}"
    );

    let installed = install_dir.join("amber");
    assert!(installed.exists(), "expected {installed:?}");
    assert!(!install_dir.join("bee").exists());
    let probe = Command::new(&installed)
        .output()
        .expect("run installed amber");
    assert!(probe.status.success());
    assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "bee-payload");
}

#[test]
fn install_sh_prefers_amber_archive_when_both_exist() {
    let tmp = tempfile::tempdir().unwrap();
    let www = tmp.path().join("www");
    let version_dir = www.join("v1.17.0");
    fs::create_dir_all(&version_dir).unwrap();

    let amber_stage = tmp.path().join("stage-amber");
    let bee_stage = tmp.path().join("stage-bee");
    fs::create_dir_all(&amber_stage).unwrap();
    fs::create_dir_all(&bee_stage).unwrap();
    make_tarball(
        &amber_stage,
        "amber",
        "#!/bin/sh\necho amber-payload\n",
        &version_dir.join("amber-v1.17.0-x86_64-unknown-linux-gnu.tar.gz"),
    );
    make_tarball(
        &bee_stage,
        "bee",
        "#!/bin/sh\necho bee-payload\n",
        &version_dir.join("bee-v1.17.0-x86_64-unknown-linux-gnu.tar.gz"),
    );

    let server = spawn_fixture_server(&www);
    let home = tmp.path().join("home");
    let install_dir = tmp.path().join("bin");
    fs::create_dir_all(&home).unwrap();

    let output = run_install(
        "v1.17.0",
        "Linux",
        "x86_64",
        &server.base,
        &install_dir,
        &home,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "install.sh amber-prefer failed: {stderr}{stdout}"
    );
    assert!(
        !stdout.contains("bee-v1.17.0"),
        "must not fall back when amber- exists: {stdout}"
    );
    let probe = Command::new(install_dir.join("amber"))
        .output()
        .expect("run installed amber");
    assert_eq!(
        String::from_utf8_lossy(&probe.stdout).trim(),
        "amber-payload"
    );
}

#[test]
fn install_sh_lists_tried_urls_when_both_assets_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let www = tmp.path().join("www");
    fs::create_dir_all(www.join("v1.16.0")).unwrap();
    let server = spawn_fixture_server(&www);
    let home = tmp.path().join("home");
    let install_dir = tmp.path().join("bin");
    fs::create_dir_all(&home).unwrap();

    let output = run_install(
        "v1.16.0",
        "Darwin",
        "arm64",
        &server.base,
        &install_dir,
        &home,
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("download failed (tried:"),
        "missing tried URLs: {stderr}"
    );
    assert!(stderr.contains("amber-v1.16.0-aarch64-apple-darwin.tar.gz"));
    assert!(stderr.contains("bee-v1.16.0-aarch64-apple-darwin.tar.gz"));
}

#[test]
fn release_workflow_still_emits_amber_prefix_archives() {
    let yaml =
        fs::read_to_string(repo_root().join(".github/workflows/release-assets.yml")).unwrap();
    assert!(yaml.contains("amber-${RELEASE_TAG}-${TARGET}.tar.gz"));
    assert!(yaml.contains("amber-${RELEASE_TAG}-${TARGET}"));
    assert!(
        !yaml.contains("bee-${RELEASE_TAG}") && !yaml.contains("bee-${{"),
        "future releases must not go back to bee- prefixes: found bee- in workflow"
    );
}
