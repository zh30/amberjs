// Snapshot Renderer
/// Pretty-prints snapshots and generates diff views
use std::fmt;
/// Snapshot pretty printer
pub struct SnapshotPrettyPrinter {
    config: SnapshotConfig,
}
impl SnapshotPrettyPrinter {
    pub fn new(config: SnapshotConfig) -> Self {
        SnapshotPrettyPrinter { config }
    }
    /// Render snapshot with formatting
    pub fn render(&self, value: &dyn std::fmt::Display) -> String {
        if self.config.pretty_print {
            self.format_pretty(value)
        } else {
            value.to_string()
        }
    }
    /// Format value nicely
    fn format_pretty(&self, value: &dyn std::fmt::Display) -> String {
        let str_value: _ = value.to_string();
        // Try to parse as JSON and pretty print
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&str_value) {
            return serde_json::to_string_pretty(&json_value).unwrap_or(str_value);
        }
        // For other types, just return as-is
        str_value
    }
    /// Render inline snapshot
    pub fn render_inline(&self, value: &dyn std::fmt::Display) -> String {
        let rendered: _ = self.render(value);
        format!("Snapshot({})", rendered)
    }
    /// Render diff view
    pub fn render_diff(&self, old: &str, new: &str) -> String {
        let mut diff = String::new();
        diff.push_str("Expected:\n");
        diff.push_str(&self.render(&old));
        diff.push_str("\n\nReceived:\n");
        diff.push_str(&self.render(&new));
        diff
    }
    /// Render comparison result
    pub fn render_comparison(&self, comparison: &SnapshotComparison) -> String {
        if comparison.matches {
            format!(
                "✓ Snapshot matched for '{}'",
                comparison.name
            )
        } else {
            let mut result = format!("✗ Snapshot mismatch for '{}'\n", comparison.name));
            result.push_str("\nExpected:\n");
            if let Some(expected) = &comparison.expected {
                result.push_str(&self.render(&expected));
            } else {
                result.push_str("(no snapshot found)");
            }
            result.push_str("\n\nReceived:\n");
            result.push_str(&self.render(&comparison.received));
            result.push('\n');
            if let Some(diff) = &comparison.diff {
                result.push_str("\nDiff:\n");
                result.push_str(diff);
            }
            result
        }
    }
    /// Render snapshot metadata
    pub fn render_metadata(&self, metadata: &SnapshotMetadata) -> String {
        let mut result = String::new();
        result.push_str(&format!("Snapshot: {}\n", metadata.name));
        result.push_str(&format!("Version: {}\n", metadata.version));
        result.push_str(&format!("Created: {}\n", metadata.created_at));
        result.push_str(&format!("Updated: {}\n", metadata.updated_at));
        result.push_str(&format!("Lines: {}\n", metadata.line_count));
        result.push_str(&format!("Size: {} bytes\n", metadata.size_bytes));
        result
    }
}
/// Snapshot format options
#[derive(Debug, Clone)]
pub struct SnapshotFormatOptions {
    pub pretty: bool,
    pub inline: bool,
    pub show_diff: bool,
    pub show_metadata: bool,
    pub theme: SnapshotTheme,
}
#[derive(Debug, Clone)]
pub enum SnapshotTheme {
    Plain,
    Colored,
    Json,
}
impl Default for SnapshotFormatOptions {
    fn default() -> Self {
        SnapshotFormatOptions {
            pretty: true,
            inline: false,
            show_diff: false,
            show_metadata: false,
            theme: SnapshotTheme::Plain,
        }
    }
}
/// High-level snapshot renderer
pub struct SnapshotRenderer {
    config: SnapshotConfig,
    printer: SnapshotPrettyPrinter,
}
impl SnapshotRenderer {
    pub fn new(config: SnapshotConfig) -> Self {
        SnapshotRenderer {
            printer: SnapshotPrettyPrinter::new(config.clone()),
            config,
        }
    }
    /// Render test result
    pub fn render_test_result(
        &self,
        name: &str,
        received: &dyn std::fmt::Display,
        comparison: &SnapshotComparison,
    ) -> String {
        let mut result = String::new();
        // Add status
        if comparison.matches {
            result.push_str("✓ ");
        } else {
            result.push_str("✗ ");
        }
        result.push_str(name);
        result.push('\n');
        // Add comparison details
        result.push_str(&self.printer.render_comparison(comparison));
        result.push('\n');
        result
    }
    /// Render summary
    pub fn render_summary(&self, comparisons: &[SnapshotComparison]) -> String {
        let mut result = String::new();
        result.push_str("Snapshot Test Summary\n");
        result.push_str("===================\n\n");
        let matched: _ = comparisons.iter().filter(|c| c.matches).count();
        let mismatched: _ = comparisons.len() - matched;
        result.push_str(&format!("Total: {} snapshots\n", comparisons.len());
        result.push_str(&format!("Matched: {}\n", matched));
        result.push_str(&format!("Mismatched: {}\n", mismatched));
        if mismatched > 0 {
            result.push_str("\nMismatched snapshots:\n");
            for comparison in comparisons.iter().filter(|c| !c.matches) {
                result.push_str(&format!("  - {}\n", comparison.name));
            }
        }
        result
    }
    /// Render update notice
    pub fn render_update_notice(&self) -> String {
        let mut result = String::new();
        result.push_str("⚠ Snapshot update required\n");
        result.push_str("Run tests with --update-snapshots to update snapshots\n");
        result
    }
    /// Render help text
    pub fn render_help(&self) -> String {
        let mut result = String::new();
        result.push_str("Snapshot Testing\n");
        result.push_str("================\n\n");
        result.push_str("Snapshot tests compare serialized values to stored snapshots.\n\n");
        result.push_str("Usage:\n");
        result.push_str("  - Run tests: amber test\n");
        result.push_str("  - Update snapshots: amber test --update-snapshots\n");
        result.push_str("  - Inline snapshots: amber test --inline-snapshots\n\n");
        result.push_str("Configuration:\n");
        result.push_str("  - Set SNAPSHOT_UPDATE=true to auto-update\n");
        result.push_str("  - Set SNAPSHOT_INLINE=true for inline snapshots\n");
        result.push_str("  - Customize serializer in code\n");
        result
    }
}
#[cfg(test)]
mod tests {
use std::collections::{HashMap, BTreeMap};
    #[test]
    fn test_pretty_print_json() {
        let config: _ = SnapshotConfig::default();
        let printer: _ = SnapshotPrettyPrinter::new(config);
        let value: _ = r#"{"name":"test","value":42}"#;
        let rendered: _ = printer.render(&value);
        assert!(rendered.contains("\"name\": \"test\""));
    }
    #[test]
    fn test_render_comparison_match() {
        let config: _ = SnapshotConfig::default();
        let renderer: _ = SnapshotRenderer::new(config);
        let comparison: _ = SnapshotComparison::new_match("test".to_string(), "value".to_string());
        let rendered: _ = renderer.render_test_result("test", &"value", &comparison);
        assert!(rendered.contains("✓"));
        assert!(rendered.contains("test"));
    }
    #[test]
    fn test_render_comparison_mismatch() {
        let config: _ = SnapshotConfig::default();
        let renderer: _ = SnapshotRenderer::new(config);
        let comparison: _ = SnapshotComparison::new_mismatch(
            "test".to_string(),
            "new_value".to_string(),
            "old_value".to_string(),
        );
        let rendered: _ = renderer.render_test_result("test", &"new_value", &comparison);
        assert!(rendered.contains("✗"));
        assert!(rendered.contains("Expected:"));
        assert!(rendered.contains("Received:"));
    }
    #[test]
    fn test_render_summary() {
        let config: _ = SnapshotConfig::default();
        let renderer: _ = SnapshotRenderer::new(config);
        let comparisons: _ = vec![
            SnapshotComparison::new_match("test1".to_string(), "value1".to_string()),
            SnapshotComparison::new_match("test2".to_string(), "value2".to_string()),
            SnapshotComparison::new_mismatch(
                "test3".to_string(),
                "new_value".to_string(),
                "old_value".to_string(),
            ),
        ];
        let rendered: _ = renderer.render_summary(&comparisons);
        assert!(rendered.contains("Total: 3"));
        assert!(rendered.contains("Matched: 2"));
        assert!(rendered.contains("Mismatched: 1"));
        assert!(rendered.contains("test3"));
    }
    #[test]
    fn test_render_metadata() {
        let config: _ = SnapshotConfig::default();
        let printer: _ = SnapshotPrettyPrinter::new(config);
        let metadata: _ = SnapshotMetadata {
            name: "test".to_string(),
            version: "1".to_string(),
            created_at: "1234567890".to_string(),
            updated_at: "1234567890".to_string(),
            line_count: 10,
            size_bytes: 100,
        };
        let rendered: _ = printer.render_metadata(&metadata);
        assert!(rendered.contains("Snapshot: test"));
        assert!(rendered.contains("Lines: 10"));
        assert!(rendered.contains("Size: 100 bytes"));
    }
}