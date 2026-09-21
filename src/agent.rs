//! Agent host surface: tool export, stdin JSON-RPC session, MCP stdio.
//!
//! Models stay outside Amber. This module only loads a JS/TS tool file and
//! calls named exports under the existing MinimalRuntime + ResourceBroker.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};

use crate::runtime_minimal::MinimalRuntime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_object_schema", rename = "inputSchema")]
    pub input_schema: Value,
}

fn default_object_schema() -> Value {
    json!({ "type": "object" })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolsFile {
    #[serde(default)]
    tools: Vec<ToolSchema>,
}

pub fn export_tools_from_entry(entry: &Path) -> Result<Vec<ToolSchema>> {
    let sidecar = tools_json_path(entry);
    if sidecar.is_file() {
        let contents = std::fs::read_to_string(&sidecar)
            .map_err(|e| anyhow!("Failed to read tools manifest {}: {e}", sidecar.display()))?;
        let parsed: ToolsFile = serde_json::from_str(&contents).map_err(|e| {
            anyhow!(
                "Failed to parse tools manifest {} as JSON: {e}",
                sidecar.display()
            )
        })?;
        if parsed.tools.is_empty() {
            return Err(anyhow!(
                "tools manifest {} has an empty tools array",
                sidecar.display()
            ));
        }
        return Ok(parsed.tools);
    }

    let source = std::fs::read_to_string(entry)
        .map_err(|e| anyhow!("Failed to read {}: {e}", entry.display()))?;
    let scanned = scan_exported_functions(&source);
    if scanned.is_empty() {
        return Err(anyhow!(
            "No tools.json next to {} and no exported functions found",
            entry.display()
        ));
    }
    Ok(scanned)
}

fn wrap_source_for_tool_exports(source: &str) -> String {
    let names: Vec<String> = scan_exported_functions(source)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    let rewritten = source
        .replace("export async function ", "async function ")
        .replace("export function ", "function ");
    if names.is_empty() {
        return rewritten;
    }
    format!(
        "{rewritten}\n;globalThis.__beeToolExports = {{ {} }};\n",
        names.join(", ")
    )
}

fn tools_json_path(entry: &Path) -> PathBuf {
    entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("tools.json")
}

fn scan_exported_functions(source: &str) -> Vec<ToolSchema> {
    let mut tools = Vec::new();
    let mut pending_doc = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/**") || trimmed.starts_with('*') || trimmed.starts_with("//") {
            let clean = trimmed
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .trim_start_matches('*')
                .trim_start_matches("//")
                .trim();
            if !clean.is_empty() && !clean.starts_with('@') {
                pending_doc.push(clean.to_string());
            }
            continue;
        }

        let name = trimmed
            .strip_prefix("export async function ")
            .or_else(|| trimmed.strip_prefix("export function "))
            .and_then(|rest| rest.split(|c: char| !is_ident_char(c)).next())
            .filter(|name| !name.is_empty());

        if let Some(name) = name {
            if !tools.iter().any(|tool: &ToolSchema| tool.name == name) {
                let description = if !pending_doc.is_empty() {
                    pending_doc.join(" ")
                } else {
                    format!("Exported function `{name}`")
                };
                tools.push(ToolSchema {
                    name: name.to_string(),
                    description,
                    input_schema: default_object_schema(),
                });
            }
            pending_doc.clear();
        } else if !trimmed.is_empty() {
            pending_doc.clear();
        }
    }
    tools
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

pub fn tools_list_json(tools: &[ToolSchema]) -> Value {
    json!({ "tools": tools })
}

pub struct AgentSession {
    entry: PathBuf,
    source: String,
    isolate_per_call: bool,
    runtime: Option<MinimalRuntime>,
}

impl AgentSession {
    pub fn new(entry: PathBuf, isolate_per_call: bool) -> Result<Self> {
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| anyhow!("Failed to read {}: {e}", entry.display()))?;
        let mut session = Self {
            entry,
            source,
            isolate_per_call,
            runtime: None,
        };
        if !isolate_per_call {
            session.runtime = Some(session.spawn_runtime()?);
        }
        Ok(session)
    }

    fn spawn_runtime(&self) -> Result<MinimalRuntime> {
        let mut runtime =
            MinimalRuntime::new().map_err(|e| anyhow!("Failed to create runtime: {e}"))?;
        runtime.set_main_module_path(&self.entry);
        runtime
            .execute_code(&wrap_source_for_tool_exports(&self.source))
            .map_err(|e| anyhow!("Failed to load tool module {}: {e}", self.entry.display()))?;
        Ok(runtime)
    }

    pub fn call_tool(&mut self, name: &str, arguments: &Value) -> Result<Value> {
        let args_json = serde_json::to_string(arguments)?;
        let result = if self.isolate_per_call {
            let mut runtime = self.spawn_runtime()?;
            parse_tool_result(&runtime.call_named_export(name, &args_json)?)
        } else {
            let runtime = self
                .runtime
                .as_mut()
                .ok_or_else(|| anyhow!("session runtime missing"))?;
            parse_tool_result(&runtime.call_named_export(name, &args_json)?)
        };

        crate::permissions::record_tool_call_audit(name, result.is_ok());
        result
    }
}

fn parse_tool_result(raw: &str) -> Result<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "undefined" || trimmed == "null" {
        return Ok(Value::Null);
    }
    match serde_json::from_str(trimmed) {
        Ok(value) => Ok(value),
        Err(_) => Ok(Value::String(trimmed.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default = "default_jsonrpc")]
    jsonrpc: String,
    id: Option<Value>,
    #[serde(default)]
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    args: Option<Value>,
}

fn default_jsonrpc() -> String {
    "2.0".to_string()
}

pub fn run_jsonrpc_session(
    entry: PathBuf,
    isolate_per_call: bool,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<()> {
    let tools = export_tools_from_entry(&entry)?;
    let mut session = AgentSession::new(entry, isolate_per_call)?;
    for line in input.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let response = handle_jsonrpc_line(&tools, &mut session, line);
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}

fn handle_jsonrpc_line(tools: &[ToolSchema], session: &mut AgentSession, line: &str) -> String {
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => {
            return json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": error.to_string() }
            })
            .to_string();
        }
    };
    dispatch_jsonrpc(tools, session, request)
}

fn dispatch_jsonrpc(
    tools: &[ToolSchema],
    session: &mut AgentSession,
    request: JsonRpcRequest,
) -> String {
    let id = request.id.clone().unwrap_or(Value::Null);
    if let Some(tool) = request.tool.clone() {
        return match session.call_tool(&tool, request.args.as_ref().unwrap_or(&json!({}))) {
            Ok(result) => {
                json!({"jsonrpc": request.jsonrpc, "id": id, "result": result}).to_string()
            }
            Err(error) => jsonrpc_error(&request.jsonrpc, id, -32000, error.to_string()),
        };
    }

    match request.method.as_str() {
        "tools/list" | "list" => json!({
            "jsonrpc": request.jsonrpc,
            "id": id,
            "result": tools_list_json(tools)
        })
        .to_string(),
        "tools/call" | "call" => {
            let name = request
                .params
                .get("name")
                .or_else(|| request.params.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .or_else(|| request.params.get("args"))
                .cloned()
                .unwrap_or(json!({}));
            match session.call_tool(name, &arguments) {
                Ok(result) => {
                    json!({"jsonrpc": request.jsonrpc, "id": id, "result": result}).to_string()
                }
                Err(error) => jsonrpc_error(&request.jsonrpc, id, -32000, error.to_string()),
            }
        }
        "prompts/list" => json!({
            "jsonrpc": request.jsonrpc,
            "id": id,
            "result": { "prompts": [] }
        })
        .to_string(),
        "resources/list" => json!({
            "jsonrpc": request.jsonrpc,
            "id": id,
            "result": { "resources": [] }
        })
        .to_string(),
        "resources/templates/list" => json!({
            "jsonrpc": request.jsonrpc,
            "id": id,
            "result": { "resourceTemplates": [] }
        })
        .to_string(),
        "initialize" => json!({
            "jsonrpc": request.jsonrpc,
            "id": id,
            "result": mcp_initialize_result()
        })
        .to_string(),
        "ping" => json!({"jsonrpc": request.jsonrpc, "id": id, "result": {}}).to_string(),
        "" => jsonrpc_error(&request.jsonrpc, id, -32600, "missing method"),
        other => jsonrpc_error(
            &request.jsonrpc,
            id,
            -32601,
            format!("unknown method: {other}"),
        ),
    }
}

fn jsonrpc_error(version: &str, id: Value, code: i32, message: impl Into<String>) -> String {
    json!({
        "jsonrpc": version,
        "id": id,
        "error": { "code": code, "message": message.into() }
    })
    .to_string()
}

fn mcp_initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "prompts": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "amberjs",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

pub fn run_mcp_server(
    entry: PathBuf,
    isolate_per_call: bool,
    mut input: impl Read,
    mut output: impl Write,
) -> Result<()> {
    let tools = export_tools_from_entry(&entry)?;
    let mut session = AgentSession::new(entry, isolate_per_call)?;
    loop {
        let Some(body) = read_mcp_message(&mut input)? else {
            break;
        };
        if body.trim().is_empty() {
            continue;
        }
        let request: JsonRpcRequest = match serde_json::from_str(&body) {
            Ok(request) => request,
            Err(error) => {
                write_mcp_message(
                    &mut output,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": { "code": -32700, "message": error.to_string() }
                    })
                    .to_string(),
                )?;
                continue;
            }
        };
        if request.method == "notifications/initialized"
            || request.method.starts_with("notifications/")
        {
            continue;
        }
        let response = if request.method == "tools/list" {
            json!({
                "jsonrpc": request.jsonrpc,
                "id": request.id.unwrap_or(Value::Null),
                "result": {
                    "tools": tools.iter().map(|tool| json!({
                        "name": tool.name,
                        "description": tool.description,
                        "inputSchema": tool.input_schema
                    })).collect::<Vec<_>>()
                }
            })
            .to_string()
        } else if request.method == "tools/call" {
            let name = request
                .params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));
            match session.call_tool(name, &arguments) {
                Ok(result) => json!({
                    "jsonrpc": request.jsonrpc,
                    "id": request.id.unwrap_or(Value::Null),
                    "result": {
                        "content": [{ "type": "text", "text": stringify_tool_content(&result) }],
                        "isError": false
                    }
                })
                .to_string(),
                Err(error) => json!({
                    "jsonrpc": request.jsonrpc,
                    "id": request.id.unwrap_or(Value::Null),
                    "result": {
                        "content": [{ "type": "text", "text": error.to_string() }],
                        "isError": true
                    }
                })
                .to_string(),
            }
        } else {
            dispatch_jsonrpc(&tools, &mut session, request)
        };
        write_mcp_message(&mut output, &response)?;
    }
    Ok(())
}

fn stringify_tool_content(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn read_mcp_message(input: &mut impl Read) -> Result<Option<String>> {
    let mut header = Vec::new();
    let mut buf = [0u8; 1];
    while header.windows(4).all(|w| w != b"\r\n\r\n") {
        if header.len() >= 4 && header.ends_with(b"\n\n") {
            break;
        }
        let n = input.read(&mut buf)?;
        if n == 0 {
            return if header.is_empty() {
                Ok(None)
            } else {
                Err(anyhow!("truncated MCP header"))
            };
        }
        header.push(buf[0]);
        if header.len() > 64 * 1024 {
            return Err(anyhow!("MCP header too large"));
        }
    }

    let header_text = String::from_utf8_lossy(&header);
    if !header_text.to_ascii_lowercase().contains("content-length") {
        let rest = header_text.trim();
        if rest.is_empty() {
            return Ok(None);
        }
        return Ok(Some(rest.to_string()));
    }

    let length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("MCP message missing Content-Length"))?;

    let mut body = vec![0u8; length];
    input.read_exact(&mut body)?;
    Ok(Some(
        String::from_utf8(body).map_err(|e| anyhow!("MCP body is not UTF-8: {e}"))?,
    ))
}

fn write_mcp_message(output: &mut impl Write, body: &str) -> Result<()> {
    write!(
        output,
        "Content-Length: {}\r\n\r\n{}",
        body.as_bytes().len(),
        body
    )?;
    output.flush()?;
    Ok(())
}
