// os module tests for Amber runtime
// v0.3.37: Comprehensive os module testing

use serial_test::serial;

#[test]
#[serial]
fn test_os_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime.execute_code("typeof os").expect("Execution failed");
    assert_eq!(result.trim(), "object", "os should be an object");
}

#[test]
#[serial]
fn test_os_platform() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.platform()")
        .expect("Execution failed");
    let platform = result.trim();
    assert!(
        platform == "darwin" || platform == "linux" || platform == "win32",
        "platform() should return darwin, linux, or win32, got: {}",
        platform
    );
}

#[test]
#[serial]
fn test_os_arch() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime.execute_code("os.arch()").expect("Execution failed");
    let arch = result.trim();
    assert!(
        arch == "x64" || arch == "arm64",
        "arch() should return x64 or arm64, got: {}",
        arch
    );
}

#[test]
#[serial]
fn test_os_cpus_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("Array.isArray(os.cpus())")
        .expect("Execution failed");
    assert_eq!(result.trim(), "true", "os.cpus() should return an array");
}

#[test]
#[serial]
fn test_os_cpus_length() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.cpus().length")
        .expect("Execution failed");
    let length: usize = result.trim().parse().expect("Should be a number");
    assert!(
        length == num_cpus::get() || length == 4,
        "cpus length should match num_cpus or 4, got: {}",
        length
    );
}

#[test]
#[serial]
fn test_os_cpus_cpu_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const cpus = os.cpus();
        if (cpus.length > 0) {
            const cpu = cpus[0];
            typeof cpu.model + '|' + typeof cpu.speed + '|' + typeof cpu.times;
        } else {
            'empty';
        }
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(
        result.trim().starts_with("string|number|object"),
        "cpu object should have model (string), speed (number), and times (object)"
    );
}

#[test]
#[serial]
fn test_os_cpus_times() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const cpus = os.cpus();
        if (cpus.length > 0) {
            const times = cpus[0].times;
            typeof times.user + '|' + typeof times.nice + '|' + typeof times.sys + '|' + typeof times.idle + '|' + typeof times.irq;
        } else {
            'empty';
        }
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "number|number|number|number|number",
        "times object should have numeric properties"
    );
}

#[test]
#[serial]
fn test_os_freemem() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.freemem()")
        .expect("Execution failed");
    let freemem: u64 = result.trim().parse().expect("Should be a number");
    assert!(freemem > 0, "freemem should be > 0, got: {}", freemem);
}

#[test]
#[serial]
fn test_os_totalmem() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.totalmem()")
        .expect("Execution failed");
    let totalmem: u64 = result.trim().parse().expect("Should be a number");
    assert!(totalmem > 0, "totalmem should be > 0, got: {}", totalmem);
}

#[test]
#[serial]
fn test_os_uptime() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.uptime()")
        .expect("Execution failed");
    let uptime: f64 = result.trim().parse().expect("Should be a number");
    assert!(uptime > 0.0, "uptime should be > 0, got: {}", uptime);
}

#[test]
#[serial]
fn test_os_type() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime.execute_code("os.type()").expect("Execution failed");
    let os_type = result.trim();
    assert!(
        os_type == "Darwin" || os_type == "Linux" || os_type == "Windows_NT",
        "type() should return Darwin, Linux, or Windows_NT, got: {}",
        os_type
    );
}

#[test]
#[serial]
fn test_os_release() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.release()")
        .expect("Execution failed");
    let release = result.trim();
    assert!(!release.is_empty(), "release should not be empty");
    // Should be a version string like X.Y.Z
    let parts: Vec<&str> = release.split('.').collect();
    assert!(
        parts.len() >= 2,
        "release should be a version string, got: {}",
        release
    );
}

#[test]
#[serial]
fn test_os_homedir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.homedir()")
        .expect("Execution failed");
    let homedir = result.trim();
    assert!(!homedir.is_empty(), "homedir should not be empty");
    assert!(
        homedir.starts_with('/') || homedir.starts_with("C:") || homedir.starts_with("\\\\"),
        "homedir should be an absolute path, got: {}",
        homedir
    );
}

#[test]
#[serial]
fn test_os_tmpdir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("os.tmpdir()")
        .expect("Execution failed");
    let tmpdir = result.trim();
    assert!(!tmpdir.is_empty(), "tmpdir should not be empty");
}

#[test]
#[serial]
fn test_os_freemem_less_than_totalmem() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const freemem = os.freemem();
        const totalmem = os.totalmem();
        freemem < totalmem;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "freemem should be less than totalmem"
    );
}

#[test]
#[serial]
fn test_os_all_properties() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof os.platform + '|' +
        typeof os.arch + '|' +
        typeof os.cpus + '|' +
        typeof os.freemem + '|' +
        typeof os.totalmem + '|' +
        typeof os.uptime + '|' +
        typeof os.type + '|' +
        typeof os.release + '|' +
        typeof os.homedir + '|' +
        typeof os.tmpdir;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function|function|function|function|function|function|function|function|function|function",
        "os module should have all expected properties"
    );
}

#[test]
#[serial]
fn test_os_functions_are_callable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const platform = os.platform();
        const arch = os.arch();
        const cpus = os.cpus();
        const freemem = os.freemem();
        const totalmem = os.totalmem();
        const uptime = os.uptime();
        const type = os.type();
        const release = os.release();
        const homedir = os.homedir();
        const tmpdir = os.tmpdir();
        platform.length + '|' + arch.length + '|' + cpus.length + '|' +
        freemem + '|' + totalmem + '|' +
        uptime.toString().length + '|' +
        type.length + '|' + release.length + '|' +
        homedir.length + '|' + tmpdir.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(
        !result.trim().is_empty(),
        "All os functions should return values"
    );
}
