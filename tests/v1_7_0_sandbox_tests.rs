use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn test_sandbox_enclave_isolation_and_execution() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const sandbox = require('bee:sandbox');

    // 1. Basic expression evaluation
    const res1 = sandbox.createEnclave('50 * 2 + 5');
    if (res1 !== 105) throw new Error('Enclave expression mismatch: ' + res1);

    // 2. Context parameter injection
    const res2 = sandbox.createEnclave('x * y + factor', {
        context: { x: 6, y: 7, factor: 10 }
    });
    if (res2 !== 52) throw new Error('Enclave context injection mismatch: ' + res2);

    // 3. Function execution with closure context
    const res3 = sandbox.createEnclave(() => {
        const list = [1, 2, 3, 4, 5];
        return list.reduce((acc, v) => acc + v, 0);
    });
    if (res3 !== 15) throw new Error('Enclave function execution mismatch: ' + res3);

    JSON.stringify({ success: true, res1, res2, res3 });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_sandbox_audit_logger_jsonl_output() {
    let dir = tempdir().expect("tempdir");
    let audit_file = dir.path().join("audit.jsonl");
    let audit_file_str = audit_file.to_string_lossy().replace('\\', "\\\\");

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = format!(
        r#"
        const sandbox = require('bee:sandbox');
        const permissions = require('bee:permissions');

        // 1. Start audit logging
        sandbox.startAuditLog("{path}");
        const currentPath = sandbox.getAuditLogPath();
        if (!currentPath || !currentPath.includes("audit.jsonl")) {{
            throw new Error('Audit log path mismatch: ' + currentPath);
        }}

        // 2. Perform permission queries and operations
        permissions.query({{ name: 'read', path: '/etc/hosts' }});
        permissions.has({{ name: 'net', host: 'api.github.com' }});
        permissions.revoke({{ name: 'env', varName: 'DATABASE_URL' }});

        // 3. Stop audit logging
        sandbox.stopAuditLog();
        const stoppedPath = sandbox.getAuditLogPath();
        if (stoppedPath !== null) throw new Error('Audit log path should be null after stop');

        JSON.stringify({{ success: true }});
        "#,
        path = audit_file_str
    );

    let res = runtime.execute_code(&code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));

    // Verify audit log file was created and contains valid JSON lines
    assert!(audit_file.exists(), "Audit log file should exist");
    let content = fs::read_to_string(&audit_file).expect("Read audit log");
    assert!(!content.trim().is_empty(), "Audit log should not be empty");

    // Each non-empty line must be valid JSON with expected fields
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: serde_json::Value = serde_json::from_str(line).expect("JSON line parse");
        assert!(parsed.get("ts").is_some());
        assert!(parsed.get("kind").is_some());
        assert!(parsed.get("action").is_some());
        assert!(parsed.get("decision").is_some());
    }
}
