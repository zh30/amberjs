// Agent 工具沙箱与 MCP 协议测试套件
use amberjs::agent::*;
use serde_json::Value;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn test_export_tools_with_jsdoc_comments() {
    let dir = tempdir().expect("Failed to create tempdir");
    let tool_file = dir.path().join("calculator.js");
    fs::write(
        &tool_file,
        r#"
/**
 * Calculate the sum of two numbers
 */
export function add(args) {
    return args.a + args.b;
}

/**
 * Multiply two numbers together
 */
export async function multiply(args) {
    return args.a * args.b;
}
"#,
    )
    .expect("Failed to write tool file");

    let tools = export_tools_from_entry(&tool_file).expect("Failed to export tools");
    assert_eq!(tools.len(), 2);

    let add_tool = tools.iter().find(|t| t.name == "add").unwrap();
    assert_eq!(add_tool.description, "Calculate the sum of two numbers");

    let mult_tool = tools.iter().find(|t| t.name == "multiply").unwrap();
    assert_eq!(mult_tool.description, "Multiply two numbers together");
}

#[test]
#[serial]
fn test_jsonrpc_session_tools_list_and_call() {
    let dir = tempdir().expect("Failed to create tempdir");
    let tool_file = dir.path().join("tools.js");
    fs::write(
        &tool_file,
        r#"
export function greet(args) {
    return `Hello, ${args.name}!`;
}
"#,
    )
    .expect("Failed to write tool file");

    let input_data = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"greet","arguments":{"name":"Beejs"}}}
"#;
    let mut output = Vec::new();
    run_jsonrpc_session(tool_file, false, input_data.as_bytes(), &mut output)
        .expect("JSON-RPC session execution failed");

    let output_str = String::from_utf8(output).expect("Output must be utf-8");
    let lines: Vec<&str> = output_str.trim().lines().collect();
    assert_eq!(lines.len(), 2);

    let res1: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(res1["id"], 1);
    assert_eq!(res1["result"]["tools"][0]["name"], "greet");

    let res2: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(res2["id"], 2);
    assert_eq!(res2["result"], "Hello, Beejs!");
}

#[test]
#[serial]
fn test_mcp_server_initialize_and_tools_call_with_error_handling() {
    let dir = tempdir().expect("Failed to create tempdir");
    let tool_file = dir.path().join("mcp_tools.js");
    fs::write(
        &tool_file,
        r#"
export function safeAction(args) {
    return { ok: true, value: args.val };
}

export function failingAction(_args) {
    throw new Error("Intentional tool failure");
}
"#,
    )
    .expect("Failed to write tool file");

    // Construct MCP messages with Content-Length framing
    let msg1 = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let msg2 = r#"{"jsonrpc":"2.0","id":2,"method":"prompts/list"}"#;
    let msg3 = r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#;
    let msg4 = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"safeAction","arguments":{"val":42}}}"#;
    let msg5 = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"failingAction","arguments":{}}}"#;

    let input_bytes = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        msg1.len(), msg1,
        msg2.len(), msg2,
        msg3.len(), msg3,
        msg4.len(), msg4,
        msg5.len(), msg5,
    );

    let mut output = Vec::new();
    run_mcp_server(tool_file, false, input_bytes.as_bytes(), &mut output)
        .expect("run_mcp_server failed");

    let output_str = String::from_utf8(output).expect("Output must be utf-8");

    // Verify initialize response has tools, prompts, and resources capabilities
    assert!(output_str.contains(r#""capabilities":{"#));
    assert!(output_str.contains(r#""prompts":{}"#));
    assert!(output_str.contains(r#""resources":{}"#));

    // Verify safeAction succeeded with isError: false
    assert!(output_str.contains(r#""isError":false"#));
    assert!(output_str.contains("42"));

    // Verify failingAction returned structured error with isError: true
    assert!(output_str.contains(r#""isError":true"#));
    assert!(output_str.contains("Intentional tool failure"));
}
