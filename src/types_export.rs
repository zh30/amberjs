//! TypeScript Types Exporter for Amberjs
//!
//! Provides zero-overhead access and export of built-in TypeScript declarations.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Embedded type definitions from `types/amberjs.d.ts`
pub const BUILTIN_TYPE_DEFINITIONS: &str = include_str!("../types/amberjs.d.ts");

/// Returns the embedded type definitions.
pub fn get_type_definitions() -> &'static str {
    BUILTIN_TYPE_DEFINITIONS
}

/// Exports type definitions to an optional file or prints to stdout.
pub fn export_types(output_path: Option<&Path>) -> Result<()> {
    match output_path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| {
                        anyhow!("Failed to create directory {}: {}", parent.display(), e)
                    })?;
                }
            }
            fs::write(path, BUILTIN_TYPE_DEFINITIONS)
                .map_err(|e| anyhow!("Failed to write types to {}: {}", path.display(), e))?;
            println!("✅ Type definitions written to {}", path.display());
        }
        None => {
            print!("{}", BUILTIN_TYPE_DEFINITIONS);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_types_definitions_available() {
        let types = get_type_definitions();
        assert!(types.contains("declare module \"amber:ai\""));
        assert!(types.contains("declare module \"amber:db\""));
        assert!(types.contains("declare module \"amber:vector\""));
        assert!(types.contains("declare module \"amber:std\""));
        assert!(types.contains("declare module \"amber:mcp\""));
        assert!(types.contains("declare module \"amber:vfs\""));
        assert!(types.contains("declare module \"amber:ffi\""));
        assert!(types.contains("declare module \"amber:pool\""));
        assert!(types.contains("declare module \"amber:wasm\""));
        assert!(types.contains("declare module \"amber:replay\""));
        assert!(types.contains("declare module \"amber:weights\""));
        assert!(types.contains("declare module \"amber:security\""));
        assert!(types.contains("declare module \"amber:permissions\""));
        assert!(types.contains("declare module \"amber:kv\""));
        assert!(types.contains("declare module \"amber:tools\""));
        assert!(types.contains("declare module \"amber:sandbox\""));
        assert!(types.contains("declare module \"amber:bus\""));
        assert!(types.contains("declare module \"amber:grammar\""));
        assert!(types.contains("declare module \"amber:checkpoint\""));
        assert!(types.contains("declare module \"amber:sockets\""));
        assert!(types.contains("class DOMException"));
        assert!(types.contains("class URLPattern"));
        assert!(types.contains("class MessageBus"));
        assert!(types.contains("parsePartialJSON"));
        assert!(types.contains("class CheckpointManager"));
        assert!(types.contains("class Tensor"));
        assert!(types.contains("class Database"));
        assert!(types.contains("class VectorDB"));
        assert!(types.contains("class LLM"));
        assert!(types.contains("class IsolatePool"));
        assert!(types.contains("class MemoryView"));
        assert!(types.contains("function dlopen"));
        assert!(types.contains("function copyMemory"));
        assert!(types.contains("class McpServer"));
        assert!(types.contains("class McpClient"));
        assert!(types.contains("embedBatch"));
        assert!(types.contains("declare namespace amber"));
    }

    #[test]
    fn test_export_types_to_file() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("amber.d.ts");
        export_types(Some(&file_path)).expect("export should succeed");
        let content = fs::read_to_string(&file_path).expect("read");
        assert!(content.contains("declare module \"amber:ai\""));
    }
}
