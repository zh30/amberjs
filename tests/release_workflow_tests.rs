use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn release_assets_yaml() -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release-assets.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn ci_yaml() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn docker_yaml() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/docker.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn dockerfile_text() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Dockerfile")).unwrap()
}

fn dockerfile_has_amber_entrypoint() -> bool {
    let docker = dockerfile_text();
    docker.contains("ENTRYPOINT [\"amber\"]")
}

#[test]
fn tag_v_star_publishes_non_draft_release_with_five_amber_archives() {
    let yaml = release_assets_yaml();

    assert!(
        yaml.contains("tags: [\"v*\"]") || yaml.contains("tags: ['v*']"),
        "Release Assets must trigger on v* tags"
    );
    assert!(
        yaml.contains("startsWith(github.ref, 'refs/tags/v')"),
        "publish path must be gated on refs/tags/v"
    );
    assert!(
        yaml.contains("draft: false"),
        "GitHub Release must be non-draft"
    );
    assert!(
        yaml.contains("generate_release_notes.py") && yaml.contains("checksums.txt"),
        "release notes and SHA-256 checksums must be generated"
    );
    assert!(
        yaml.contains("cosign sign-blob") && yaml.contains("cdx.json"),
        "release must attach cosign signatures and a CycloneDX SBOM"
    );
    assert!(
        yaml.contains("x86_64-unknown-linux-gnu")
            && yaml.contains("aarch64-unknown-linux-gnu")
            && yaml.contains("aarch64-apple-darwin")
            && yaml.contains("x86_64-apple-darwin")
            && yaml.contains("x86_64-pc-windows-msvc"),
        "must ship linux gnu x64/arm64, macOS arm64/x64, and Windows x64"
    );
    assert!(
        yaml.contains("archive: zip") && yaml.contains("amber.exe"),
        "Windows asset must be a zip containing amber.exe"
    );
    assert!(
        !yaml.contains("continue-on-error: ${{ matrix.os == 'windows-latest' }}")
            && !yaml.contains("continue-on-error: ${{ matrix.os == \"windows-latest\" }}"),
        "Windows MSVC release job must not continue-on-error"
    );
    assert!(
        yaml.contains("Windows zip amber-*-x86_64-pc-windows-msvc.zip")
            || yaml.contains("x86_64-pc-windows-msvc.zip"),
        "publish job must require the Windows zip"
    );
    assert!(
        yaml.contains("amber.exe"),
        "Windows zip must be checked for amber.exe"
    );
    assert!(
        yaml.contains("CARGO_REGISTRY_TOKEN is not set") || yaml.contains("skipping crates.io"),
        "missing crates.io token must be annotated, not silent success"
    );
}

#[test]
fn homebrew_formula_push_must_not_block_github_release() {
    let yaml = release_assets_yaml();
    let release_at = yaml
        .find("uses: softprops/action-gh-release")
        .expect("GitHub Release step");
    let commit_at = yaml
        .find("git push origin \"HEAD:${default_branch}\"")
        .expect("Homebrew formula push");
    assert!(
        commit_at > release_at,
        "Homebrew git push must run after action-gh-release so a ruleset block cannot drop the Release"
    );
}

#[test]
fn dockerfile_has_curl_for_rusty_v8_static_lib_download() {
    let docker =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Dockerfile")).unwrap();
    assert!(
        docker.contains("curl") && docker.contains("python3"),
        "rusty_v8 0.22 build.rs downloads librusty_v8 with python then curl"
    );
    let ignore =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".dockerignore"))
            .unwrap();
    assert!(
        ignore.contains("!build.rs"),
        "Docker context must include build.rs"
    );
}

/// Paths `include_str!` / `include_bytes!` load from outside `src/` must be in
/// the Docker build context. GHCR failed with:
/// `couldn't read src/../types/amberjs.d.ts`.
fn rust_include_assets_outside_src() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    let mut assets = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for needle in ["include_str!(\"", "include_bytes!(\""] {
            let mut rest = text.as_str();
            while let Some(idx) = rest.find(needle) {
                let after = &rest[idx + needle.len()..];
                let end = after.find('"').expect("closing quote");
                let rel = &after[..end];
                let resolved = normalize_path(&file.parent().unwrap().join(rel));
                let src_norm = normalize_path(&src);
                if !resolved.starts_with(&src_norm) {
                    let stripped = resolved
                        .strip_prefix(normalize_path(&manifest).as_path())
                        .unwrap_or(&resolved)
                        .to_path_buf();
                    if !assets.contains(&stripped) {
                        assets.push(stripped);
                    }
                }
                rest = &after[end + 1..];
            }
        }
    }
    assets
}

fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn collect_rs(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn docker_context_includes_compile_time_assets() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ignore = fs::read_to_string(manifest.join(".dockerignore")).unwrap();
    let dockerfile = fs::read_to_string(manifest.join("Dockerfile")).unwrap();
    let assets = rust_include_assets_outside_src();
    assert!(
        !assets.is_empty(),
        "expected at least types/amberjs.d.ts via include_str!"
    );
    for asset in &assets {
        let top = asset
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy();
        assert!(
            ignore.contains(&format!("!{top}/"))
                || ignore.contains(&format!("!{}", asset.display())),
            ".dockerignore must allow {asset:?}: {ignore}"
        );
        assert!(
            dockerfile.contains(&format!("COPY {top}"))
                || dockerfile.contains(&format!("COPY {}", asset.display())),
            "Dockerfile must COPY {top} so rustc can read {asset:?}: {dockerfile}"
        );
    }
    assert!(
        assets.iter().any(|p| p.ends_with("types/amberjs.d.ts")),
        "scanner missed types/amberjs.d.ts: {assets:?}"
    );
}

#[test]
fn dependabot_ignores_windows_sys_past_0_52() {
    let yaml = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/dependabot.yml"),
    )
    .unwrap();
    assert!(
        yaml.contains("dependency-name: windows-sys") && yaml.contains(">=0.53.0"),
        "windows-sys 0.61 breaks HMODULE/Win32 module layout locked to 0.52: {yaml}"
    );
}

#[test]
fn macos_x86_64_asset_job_uses_live_intel_runner() {
    let yaml = release_assets_yaml();

    assert!(
        !yaml.contains("macos-13"),
        "macos-13 was retired 2025-12-04; the Intel macOS asset job would never start"
    );

    let intel_block = yaml
        .split("include:")
        .nth(1)
        .expect("matrix include")
        .split("steps:")
        .next()
        .expect("steps");

    // x86_64 macOS is cross-compiled on macos-latest (ARM). macos-15-intel is
    // not required and was dropped to avoid the retired / scarce Intel runner.
    assert!(
        intel_block.contains("os: macos-latest")
            && intel_block.contains("target: x86_64-apple-darwin")
            && !intel_block.contains("macos-15-intel"),
        "x86_64-apple-darwin must be listed on macos-latest (cross-compile), not macos-15-intel: {intel_block}"
    );
}

#[test]
fn ci_gates_are_fail_closed_and_cover_oses() {
    let yaml = ci_yaml();
    assert!(
        !yaml.contains("continue-on-error: true"),
        "cargo-audit must fail closed"
    );
    assert!(
        !yaml.contains("::warning::feature"),
        "feature matrix must not swallow compile failures"
    );
    assert!(
        yaml.contains("for feat in benchmarks observability"),
        "feature matrix must list only features that currently compile (not empty ai)"
    );
    let scope =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/CURRENT_SCOPE.md"))
            .unwrap();
    assert!(
        scope.contains("`benchmarks` and `observability` are in the CI compile matrix")
            || scope.contains("benchmarks") && scope.contains("observability"),
        "CURRENT_SCOPE must describe the CI feature matrix"
    );
    assert!(
        yaml.contains("cargo-deny") || yaml.contains("deny-action"),
        "CI must run cargo-deny"
    );
    assert!(
        yaml.contains("wintertc_compliance_tests"),
        "CI must run WinterTC tests as a named step"
    );
    assert!(yaml.contains("windows-latest"), "CI must smoke Windows");
    assert!(
        yaml.contains("Library tests on macOS") || yaml.contains("cargo test --lib --tests"),
        "macOS must run tests, not only --version"
    );
    assert!(
        yaml.contains("run: cargo audit")
            || yaml.contains("audit-check")
            || yaml.contains("cargo deny"),
        "CI must run cargo-audit or cargo-deny"
    );
    assert!(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".cargo/audit.toml")
            .is_file(),
        "pre-existing advisories must be listed in .cargo/audit.toml so new IDs still fail CI"
    );
}

#[test]
fn docker_workflow_publishes_ghcr_on_v_tags() {
    let yaml = docker_yaml();
    assert!(
        yaml.contains("ghcr.io/zh30/amberjs"),
        "image must target ghcr.io/zh30/amberjs"
    );
    assert!(
        yaml.contains("tags: [\"v*\"]") || yaml.contains("tags: ['v*']"),
        "GHCR workflow must trigger on v* tags"
    );
    assert!(
        yaml.contains("push: true"),
        "GHCR workflow must push, not only load: true"
    );
    assert!(
        yaml.contains("docker run --rm") && yaml.contains("--version"),
        "image job must smoke amber --version"
    );
    assert!(
        !yaml.contains("docker run --rm ${{ env.IMAGE }}:ci --version")
            || dockerfile_has_amber_entrypoint(),
        "docker run IMAGE --version requires ENTRYPOINT [\"amber\"]; otherwise --version replaces CMD"
    );
    assert!(
        yaml.contains("amd64-only") || yaml.contains("linux/amd64 only"),
        "GHCR must document amd64-only rather than a fake dual-arch tag"
    );
    assert!(
        !yaml.contains("linux/arm64"),
        "do not advertise linux/arm64 unless a real arm64 build exists"
    );
}

#[test]
fn dockerfile_entrypoint_is_amber_so_docker_run_version_works() {
    let docker = dockerfile_text();
    assert!(
        docker.contains("ENTRYPOINT [\"amber\"]"),
        "GHCR smoke is `docker run IMAGE --version`; that only works with ENTRYPOINT amber: {docker}"
    );
    assert!(
        docker.contains("CMD [\"serve\""),
        "default container args must be serve host/port, not a second amber executable: {docker}"
    );
    assert!(
        !docker.contains("CMD [\"amber\", \"serve\""),
        "CMD must not start with amber once ENTRYPOINT is amber (would become `amber amber serve`)"
    );
}

#[test]
fn dependabot_covers_cargo_and_github_actions() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/dependabot.yml");
    let yaml = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(yaml.contains("package-ecosystem: cargo"));
    assert!(yaml.contains("package-ecosystem: github-actions"));
}

#[test]
fn homebrew_formula_points_at_github_release_assets() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Formula/amber.rb");
    let formula =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    for asset in [
        "amber-v#{version}-aarch64-apple-darwin.tar.gz",
        "amber-v#{version}-x86_64-apple-darwin.tar.gz",
        "amber-v#{version}-aarch64-unknown-linux-gnu.tar.gz",
        "amber-v#{version}-x86_64-unknown-linux-gnu.tar.gz",
    ] {
        assert!(
            formula.contains(asset),
            "Homebrew formula missing asset {asset}"
        );
    }
    assert!(formula.contains("https://github.com/zh30/amberjs/releases/download/"));
}

#[test]
fn generate_release_notes_detects_aarch64_linux_before_generic_linux() {
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/generate_release_notes.py");
    let output = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import importlib.util
spec = importlib.util.spec_from_file_location("grn", r"{script}")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
assert mod.detect_target_platform("amber-v1.8.0-aarch64-unknown-linux-gnu.tar.gz") == "Linux (aarch64)", mod.detect_target_platform("amber-v1.8.0-aarch64-unknown-linux-gnu.tar.gz")
assert mod.detect_target_platform("amber-v1.8.0-x86_64-unknown-linux-gnu.tar.gz") == "Linux (x86_64)"
print("ok")
"#,
            script = script.display()
        ))
        .output()
        .expect("python detect_target_platform");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "detect_target_platform: {stderr}{stdout}"
    );
}

#[test]
fn install_sh_maps_unix_platforms_to_release_targets() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("install.sh");
    let cases = [
        ("Darwin", "arm64", "aarch64-apple-darwin"),
        ("Darwin", "x86_64", "x86_64-apple-darwin"),
        ("Linux", "x86_64", "x86_64-unknown-linux-gnu"),
        ("Linux", "aarch64", "aarch64-unknown-linux-gnu"),
    ];
    for (os, arch, expected) in cases {
        let output = Command::new("sh")
            .arg(&script)
            .arg("--print-platform")
            .env("AMBER_UNAME_S", os)
            .env("AMBER_UNAME_M", arch)
            .output()
            .expect("run install.sh --print-platform");
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "install.sh --print-platform {os}/{arch} failed: {stderr}"
        );
        assert_eq!(stdout, expected, "platform mapping for {os}/{arch}");
        assert!(
            !stderr.contains("prebuilt Linux arm64 archive is not available yet"),
            "Linux aarch64 must be supported"
        );
    }

    let ps1 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("install.ps1");
    let ps1_text = fs::read_to_string(&ps1).expect("install.ps1");
    assert!(ps1_text.contains("x86_64-pc-windows-msvc.zip"));
    assert!(ps1_text.contains("amber.exe"));
}

#[test]
fn windows_sys_imports_match_v0_52_modules() {
    let cpu = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/nodejs_core/process.rs"),
    )
    .unwrap();
    assert!(
        cpu.contains("use windows_sys::Win32::System::Threading::GetCurrentProcess"),
        "GetCurrentProcess must come from Threading on windows-sys 0.52"
    );
    assert!(
        cpu.contains("use windows_sys::Win32::System::ProcessStatus::{")
            && cpu.contains("GetProcessMemoryInfo")
            && cpu.contains("PROCESS_MEMORY_COUNTERS"),
        "GetProcessMemoryInfo/PROCESS_MEMORY_COUNTERS must come from ProcessStatus"
    );
    assert!(
        !cpu.contains("Win32::Foundation::GetCurrentProcess"),
        "GetCurrentProcess is not in Foundation in windows-sys 0.52"
    );
    assert!(
        !cpu.contains("Diagnostics::Debug::{\n            GetProcessMemoryInfo"),
        "GetProcessMemoryInfo is not in Diagnostics::Debug in windows-sys 0.52"
    );
    assert!(
        !cpu.contains("Diagnostics::Process::GetProcessTimes"),
        "Win32::System::Diagnostics::Process does not exist"
    );
}

fn snippet_after(src: &str, needle: &str, len: usize) -> String {
    let idx = src
        .find(needle)
        .unwrap_or_else(|| panic!("missing `{needle}`"));
    src[idx..src.len().min(idx + len)].to_string()
}

#[test]
fn windows_hmodule_is_isize_not_pointer() {
    let napi =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/napi/mod.rs"))
            .unwrap();
    let napi_load = snippet_after(&napi, "LoadLibraryA", 280);
    assert!(
        napi_load.contains("if handle == 0"),
        "LoadLibraryA HMODULE must be compared to 0: {napi_load}"
    );
    assert!(
        !napi_load.contains("is_null()"),
        "HMODULE (isize) has no is_null(): {napi_load}"
    );

    let ffi = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ffi/mod.rs"))
        .unwrap();
    assert!(
        ffi.contains("handle: windows_sys::Win32::Foundation::HMODULE"),
        "ffi DynamicLibrary stores HMODULE"
    );
    let ffi_load = snippet_after(&ffi, "LoadLibraryA", 420);
    assert!(
        ffi_load.contains("if handle == 0"),
        "ffi LoadLibraryA HMODULE must be compared to 0: {ffi_load}"
    );
    assert!(
        !ffi_load.contains("is_null()") && !ffi_load.contains("null_mut()"),
        "ffi LoadLibraryA must not treat HMODULE as a pointer: {ffi_load}"
    );
    let ffi_free = snippet_after(&ffi, "FreeLibrary", 220);
    assert!(
        ffi.contains("use windows_sys::Win32::Foundation::FreeLibrary"),
        "windows-sys 0.52 FreeLibrary is in Foundation, not LibraryLoader: {ffi_free}"
    );
    assert!(
        !ffi.contains("LibraryLoader::FreeLibrary"),
        "LibraryLoader has FreeLibraryAndExitThread, not FreeLibrary: {ffi_free}"
    );
    assert!(
        ffi.contains("if self.handle != 0") && ffi.contains("self.handle = 0"),
        "ffi FreeLibrary path must use integer 0 for HMODULE"
    );
    assert!(
        !ffi_free.contains("is_null()") && !ffi_free.contains("null_mut()"),
        "ffi FreeLibrary must not treat HMODULE as a pointer: {ffi_free}"
    );
}

#[test]
fn windows_os_uptime_uses_gettickcount64_not_sys_info_boottime() {
    let os =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/nodejs_core/os.rs"))
            .unwrap();
    let uptime = snippet_after(&os, "fn os_uptime_callback", 900);
    assert!(
        uptime.contains("GetTickCount64"),
        "Windows os.uptime must use GetTickCount64: {uptime}"
    );
    assert!(
        uptime.contains("cfg(windows)") && uptime.contains("sys_info::boottime"),
        "sys_info::boottime is cfg(not(windows)) and must stay off the Windows path: {uptime}"
    );
    let cargo =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("Win32_System_SystemInformation"),
        "GetTickCount64 needs windows-sys feature Win32_System_SystemInformation"
    );
}

#[test]
fn linux_amber_exports_napi_symbols_via_per_binary_flag() {
    let build = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
        .expect("build.rs");
    assert!(
        build.contains("cargo:rustc-link-arg-bin=amber=-Wl,--export-dynamic"),
        "Linux N-API addons resolve napi_* from amber dynsym; flag must be per-binary: {build}"
    );
    assert!(
        build.contains("linux"),
        "export-dynamic is an ELF requirement, not a global rustflag: {build}"
    );
    let cargo_config =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".cargo/config.toml"))
            .unwrap_or_default();
    assert!(
        !cargo_config.contains("rdynamic") && !cargo_config.contains("export-dynamic"),
        "do not put -rdynamic in .cargo/config.toml (breaks rust-crypto): {cargo_config}"
    );
}

#[test]
fn debugger_docs_describe_inspect_not_stage59_debug() {
    let debugger = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/DEBUGGER_USAGE.md"),
    )
    .unwrap();
    assert!(debugger.contains("amber run --inspect"));
    assert!(debugger.contains("amber run --inspect-brk"));
    assert!(debugger.contains("9229"));
    assert!(
        !debugger.contains("当前 public CLI 仅暴露 `amber debug"),
        "DEBUGGER_USAGE.md must not claim amber debug is the public inspector"
    );
    assert!(
        !debugger.contains("v0.1.0 Stage 59"),
        "DEBUGGER_USAGE.md must not describe Stage 59 as current"
    );

    let cli = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/CLI_USAGE_GUIDE.md"),
    )
    .unwrap();
    assert!(cli.contains("--inspect-brk"));
    assert!(cli.contains("--https"));
    assert!(
        cli.contains("退出码 **2**") || cli.contains("exit code 2") || cli.contains("退出码 **2**"),
        "CLI guide must document --parallel exit 2"
    );
    assert!(cli.contains("--parallel"));
}

#[test]
fn homebrew_updater_writes_nonzero_sha256_and_refuses_zeros() {
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/update_homebrew_formula.py");
    let formula_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Formula/amber.rb");
    let dir = tempfile::tempdir().unwrap();
    let release = dir.path().join("release");
    fs::create_dir_all(&release).unwrap();
    let formula = dir.path().join("amber.rb");
    fs::copy(&formula_src, &formula).unwrap();

    let version = "1.11.0";
    for target in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
    ] {
        fs::write(
            release.join(format!("amber-v{version}-{target}.tar.gz")),
            format!("dummy-{target}-payload"),
        )
        .unwrap();
    }

    let output = Command::new("python3")
        .args([
            script.to_str().unwrap(),
            "--formula",
            formula.to_str().unwrap(),
            "--release-dir",
            release.to_str().unwrap(),
            "--version",
            version,
            "--write",
        ])
        .output()
        .expect("update_homebrew_formula.py");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "homebrew updater failed: {stderr}");
    let text = fs::read_to_string(&formula).unwrap();
    assert!(text.contains("version \"1.11.0\""));
    assert!(!text.contains("0000000000000000000000000000000000000000000000000000000000000000"));
    assert!(text.contains("sha256 \""));
    for line in text.lines() {
        if line.trim().starts_with("sha256") {
            assert!(
                !line.contains("\"0000"),
                "sha256 must not be all zeros: {line}"
            );
        }
    }
}

#[test]
fn winget_manifest_installer_url_uses_windows_zip() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("manifests/winget/zh30.amber.yaml");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        text.contains("x86_64-pc-windows-msvc.zip"),
        "winget InstallerUrl must use the Windows zip name"
    );
    assert!(text.contains("amber-v"));
}
