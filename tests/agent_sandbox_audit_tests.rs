// Agent Sandbox Audit Trail & Traceability Integration Tests
use amberjs::agent::*;
use amberjs::permissions::*;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn test_agent_session_audit_log_generation() {
    reset_runtime_permission_state();
    let dir = tempdir().expect("Failed to create tempdir");
    let audit_file = dir.path().join("audit.jsonl");
    let tool_file = dir.path().join("tools.js");

    fs::write(
        &tool_file,
        r#"
export function calculate(args) {
    return args.x * args.y;
}
"#,
    )
    .expect("Failed to write tools.js");

    set_audit_log_path(Some(audit_file.clone())).expect("Failed to set audit log path");

    let _tools = export_tools_from_entry(&tool_file).expect("Failed to export tools");
    let mut session = AgentSession::new(tool_file, false).expect("Failed to create session");

    let _input_line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"calculate","arguments":{"x":6,"y":7}}}"#;
    let response = session.call_tool("calculate", &serde_json::json!({ "x": 6, "y": 7 }));
    assert!(response.is_ok(), "Tool call failed: {:?}", response.err());

    reset_runtime_permission_state();

    let audit_content = fs::read_to_string(&audit_file).expect("Failed to read audit.jsonl");
    assert!(
        audit_content.contains("tool:calculate"),
        "Audit log does not contain tool:calculate: {}",
        audit_content
    );
    assert!(
        audit_content.contains("\"decision\":\"Allow\""),
        "Audit log does not record Allow decision: {}",
        audit_content
    );
}

#[test]
#[serial]
fn test_sandbox_audit_trail_intercept_denial() {
    reset_runtime_permission_state();
    let dir = tempdir().expect("Failed to create tempdir");
    let audit_file = dir.path().join("audit_deny.jsonl");
    let forbidden_file = dir.path().join("secret.key");

    set_audit_log_path(Some(audit_file.clone())).expect("Failed to set audit log path");

    let mut broker = ResourceBroker::default();
    broker.deny_all();

    let decision = broker.check(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Path(forbidden_file.clone()),
    );
    assert_eq!(decision, PermissionDecision::Deny);

    reset_runtime_permission_state();

    let audit_content = fs::read_to_string(&audit_file).expect("Failed to read audit log");
    assert!(
        audit_content.contains("\"decision\":\"Deny\""),
        "Audit log does not contain Deny record: {}",
        audit_content
    );
    assert!(
        audit_content.contains("secret.key"),
        "Audit log does not contain resource path: {}",
        audit_content
    );
}
