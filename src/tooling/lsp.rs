//! Built-in Language Server Protocol (LSP) Server for Amber (`amber lsp`).
//!
//! Provides zero-configuration language support for editors (VS Code, Neovim, Helix):
//! - Real-time diagnostics via OXC linter (`textDocument/publishDiagnostics`)
//! - Instant code formatting via OXC codegen (`textDocument/formatting`)
//! - Built-in API hover documentation for `amber:ai`, `Tensor`, `LLM`, etc. (`textDocument/hover`)

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};

use crate::tooling::formatter::format_source;
use crate::tooling::linter::{lint_source, DiagnosticSeverity};

/// Reads an LSP packet with `Content-Length: <n>\r\n\r\n<payload>` framing.
pub fn read_lsp_message<R: Read>(reader: &mut BufReader<R>) -> Result<Option<Value>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut header_line = String::new();
        let bytes_read = reader.read_line(&mut header_line)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let trimmed = header_line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // End of headers
            break;
        }

        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            if let Ok(len) = rest.trim().parse::<usize>() {
                content_length = Some(len);
            }
        }
    }

    let length = match content_length {
        Some(len) => len,
        None => return Err(anyhow!("Missing Content-Length header in LSP message")),
    };

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;

    let val: Value = serde_json::from_slice(&body)
        .map_err(|e| anyhow!("Failed to parse LSP JSON payload: {}", e))?;

    Ok(Some(val))
}

/// Writes an LSP packet with standard `Content-Length` framing.
pub fn write_lsp_message<W: Write>(writer: &mut W, val: &Value) -> Result<()> {
    let json_bytes = serde_json::to_vec(val)?;
    write!(writer, "Content-Length: {}\r\n\r\n", json_bytes.len())?;
    writer.write_all(&json_bytes)?;
    writer.flush()?;
    Ok(())
}

/// Returns hover documentation for known Amber symbols.
pub fn get_hover_doc(symbol: &str) -> Option<String> {
    match symbol {
        "amber:ai" => Some(
            "### `amber:ai`\n\nNative agentic AI module in Amber, featuring zero-copy Tensor operations, local streaming LLM inference, and deterministic AgentPipeline."
                .to_string(),
        ),
        "Tensor" => Some(
            "### `class Tensor` (amber:ai)\n\nHigh-performance n-dimensional Tensor backed by Float32Array.\n\nMethods: `matmul`, `dot`, `norm`, `softmax`, `cosineSimilarity`, `add`, `sub`, `mul`, `div`."
                .to_string(),
        ),
        "LLM" => Some(
            "### `class LLM` (amber:ai)\n\nLocal streaming LLM inference engine.\n\nMethods:\n- `generate({ prompt, maxTokens, temperature })`\n- `generateStream({ prompt })`\n- `embed(text)`"
                .to_string(),
        ),
        "AgentPipeline" => Some(
            "### `class AgentPipeline` (amber:ai)\n\nDeterministic orchestrator for registering and executing Agent tools."
                .to_string(),
        ),
        "bench" => Some(
            "### `bench(name, fn)`\n\nRegisters a microbenchmark function for execution with `amber bench`."
                .to_string(),
        ),
        _ => None,
    }
}

/// Main LSP server loop over stdin/stdout.
pub fn run_lsp_server<R: Read, W: Write>(input: R, mut output: W) -> Result<()> {
    let mut reader = BufReader::new(input);
    let mut documents: HashMap<String, String> = HashMap::new();

    while let Some(msg) = read_lsp_message(&mut reader)? {
        let method = msg.get("method").and_then(|m| m.as_str());
        let id = msg.get("id").cloned();

        match method {
            Some("initialize") => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1, // Full sync
                            "documentFormattingProvider": true,
                            "hoverProvider": true
                        },
                        "serverInfo": {
                            "name": "amber-lsp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                });
                write_lsp_message(&mut output, &resp)?;
            }
            Some("initialized") => {
                // Client confirmed initialization; no response needed
            }
            Some("textDocument/didOpen") => {
                if let Some(params) = msg.get("params") {
                    if let Some(doc) = params.get("textDocument") {
                        let uri = doc.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                        let text = doc.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        documents.insert(uri.to_string(), text.to_string());

                        // Publish diagnostics
                        publish_diagnostics(&mut output, uri, text)?;
                    }
                }
            }
            Some("textDocument/didChange") => {
                if let Some(params) = msg.get("params") {
                    let uri = params
                        .get("textDocument")
                        .and_then(|d| d.get("uri"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    if let Some(changes) = params.get("contentChanges").and_then(|c| c.as_array()) {
                        if let Some(last_change) = changes.last() {
                            if let Some(text) = last_change.get("text").and_then(|t| t.as_str()) {
                                documents.insert(uri.to_string(), text.to_string());
                                publish_diagnostics(&mut output, uri, text)?;
                            }
                        }
                    }
                }
            }
            Some("textDocument/formatting") => {
                let uri = msg
                    .get("params")
                    .and_then(|p| p.get("textDocument"))
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");

                let edits = if let Some(content) = documents.get(uri) {
                    let line_count = content.lines().count();
                    match format_source(content, uri) {
                        Ok(formatted) => {
                            json!([
                                {
                                    "range": {
                                        "start": { "line": 0, "character": 0 },
                                        "end": { "line": line_count + 1, "character": 0 }
                                    },
                                    "newText": formatted
                                }
                            ])
                        }
                        Err(_) => json!([]),
                    }
                } else {
                    json!([])
                };

                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": edits
                });
                write_lsp_message(&mut output, &resp)?;
            }
            Some("textDocument/hover") => {
                let uri = msg
                    .get("params")
                    .and_then(|p| p.get("textDocument"))
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("");

                let line_idx = msg
                    .get("params")
                    .and_then(|p| p.get("position"))
                    .and_then(|pos| pos.get("line"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0) as usize;

                let char_idx = msg
                    .get("params")
                    .and_then(|p| p.get("position"))
                    .and_then(|pos| pos.get("character"))
                    .and_then(|c| c.as_u64())
                    .unwrap_or(0) as usize;

                let hover_result = documents.get(uri).and_then(|doc| {
                    let line = doc.lines().nth(line_idx)?;
                    let word = extract_word_at(line, char_idx);
                    let doc_str = get_hover_doc(&word)?;
                    Some(json!({
                        "contents": {
                            "kind": "markdown",
                            "value": doc_str
                        }
                    }))
                });

                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": hover_result.unwrap_or(Value::Null)
                });
                write_lsp_message(&mut output, &resp)?;
            }
            Some("shutdown") => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": Value::Null
                });
                write_lsp_message(&mut output, &resp)?;
            }
            Some("exit") => {
                break;
            }
            _ => {
                // If it's a request with an id, reply with method not found
                if let Some(id_val) = id {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "error": {
                            "code": -32601,
                            "message": "Method not found"
                        }
                    });
                    write_lsp_message(&mut output, &err_resp)?;
                }
            }
        }
    }

    Ok(())
}

fn extract_word_at(line: &str, char_idx: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    if char_idx >= chars.len() {
        return String::new();
    }

    let mut start = char_idx;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '_'
            || chars[start - 1] == ':')
    {
        start -= 1;
    }

    let mut end = char_idx;
    while end < chars.len()
        && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == ':')
    {
        end += 1;
    }

    chars[start..end].iter().collect()
}

fn publish_diagnostics<W: Write>(writer: &mut W, uri: &str, text: &str) -> Result<()> {
    let diags = lint_source(text, uri);
    let lsp_diags: Vec<Value> = diags
        .into_iter()
        .map(|d| {
            let severity = match d.severity {
                DiagnosticSeverity::Error => 1,
                DiagnosticSeverity::Warning => 2,
            };
            json!({
                "range": {
                    "start": { "line": d.line.saturating_sub(1), "character": d.col.saturating_sub(1) },
                    "end": { "line": d.line.saturating_sub(1), "character": d.col }
                },
                "severity": severity,
                "code": d.rule_name,
                "source": "amberjs",
                "message": d.message
            })
        })
        .collect();

    let notif = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": lsp_diags
        }
    });

    write_lsp_message(writer, &notif)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_message_roundtrip() {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        });

        let mut buf = Vec::new();
        write_lsp_message(&mut buf, &msg).expect("write");

        let mut reader = BufReader::new(&buf[..]);
        let read = read_lsp_message(&mut reader).expect("read").expect("some");
        assert_eq!(read, msg);
    }

    #[test]
    fn test_lsp_extract_word_and_hover() {
        let line = "import { Tensor } from 'amber:ai';";
        let word = extract_word_at(line, 10);
        assert_eq!(word, "Tensor");
        let doc = get_hover_doc(&word);
        assert!(doc.is_some());
        assert!(doc.unwrap().contains("class Tensor"));
    }
}
