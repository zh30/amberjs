// Grafana Dashboard Integration Module
//
// This module provides comprehensive Grafana dashboard integration for Amber runtime:
// - Dashboard management and configuration
// - Real-time metric visualization
// - Custom chart and graph rendering
// - Grafana API integration
//
// # Examples
//
// ```rust
// use amberjs::observability::dashboard::{
//     DashboardManager, DashboardConfig, PanelConfig,
//     GrafanaClient, ChartRenderer, GraphRenderer, TemplateEngine
// };
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// let config: _ = DashboardConfig::default();
// let manager: _ = DashboardManager::new(config).await?;
//
// // Create a new dashboard
// let dashboard: _ = manager.create_dashboard("amberjs-overview")?;
//
// # Ok(())
// # }
// ```
pub mod manager;
pub mod renderer;
pub use manager::*;
pub use renderer::*;
pub use renderer::{
    ChartConfig, ChartData, ChartInstance, ChartRenderer, EdgeStyle, ForceParams, GraphEdge,
    GraphInstance, GraphNode, GraphRenderer, HierarchicalParams, InteractionConfig, LayoutConfig,
    LayoutEngine, LayoutType, Position, RenderStats, SeriesData, Size, Template, TemplateEngine,
    TemplateFunction, WebSocketClient,
};
use serde::{Deserialize, Serialize};
/// Dashboard panel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    /// Panel ID
    pub id: String,
    /// Panel title
    pub title: String,
    /// Panel type (graph, stat, table, etc.)
    pub panel_type: String,
    /// Panel position and size
    pub grid_pos: GridPos,
    /// Data source
    pub datasource: String,
    /// Query configuration
    pub targets: Vec<QueryTarget>,
    /// Visualization options
    pub field_config: FieldConfig,
    /// Panel options
    pub options: PanelOptions,
}
/// Grid position for panels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
/// Query target for panels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTarget {
    pub ref_id: String,
    pub query: String,
    /// Query interval
    pub interval: String,
    /// Query legend format
    pub legend_format: String,
}
/// Field configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    /// Min/Max values
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Unit of measurement
    pub unit: Option<String>,
    /// Decimal precision
    pub decimals: Option<u32>,
    /// Thresholds
    pub thresholds: Option<ThresholdsConfig>,
}
/// Thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdsConfig {
    pub steps: Vec<ThresholdStep>,
}
/// Threshold step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdStep {
    pub color: String,
    pub value: Option<f64>,
}
/// Panel options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelOptions {
    /// Legend configuration
    pub legend: LegendConfig,
    /// Tooltip configuration
    pub tooltip: TooltipConfig,
    /// Time range
    pub time: TimeRangeConfig,
}
/// Legend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegendConfig {
    pub show: bool,
    pub position: String,
}
/// Tooltip configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipConfig {
    pub mode: String,
    pub sort: String,
}
/// Time range configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRangeConfig {
    pub from: String,
    pub to: String,
}
/// Dashboard tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTags {
    pub tags: Vec<String>,
}
/// Dashboard version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardVersion {
    pub version: i32,
}
/// Dashboard refresh interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshConfig {
    pub interval: String,
    pub pause: bool,
}
/// Custom chart types for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    LineChart,
    BarChart,
    PieChart,
    HeatMap,
    StatChart,
    Table,
}
/// Custom graph types for topology and dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphType {
    TopologyGraph,
    DependencyGraph,
    TraceGraph,
    NetworkGraph,
}
/// Template variable for dynamic dashboards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub label: String,
    pub query: String,
    pub options: Vec<VariableOption>,
}
/// Variable option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableOption {
    pub text: String,
    pub value: String,
    pub selected: bool,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_panel_config_creation() {
        let config: _ = PanelConfig {
            id: "panel-1".to_string(),
            title: "CPU Usage".to_string(),
            panel_type: "graph".to_string(),
            grid_pos: GridPos {
                x: 0,
                y: 0,
                w: 12,
                h: 8,
            },
            datasource: "Prometheus".to_string(),
            targets: vec![],
            field_config: FieldConfig {
                min: Some(0.0),
                max: Some(100.0),
                unit: Some("percent".to_string()),
                decimals: Some(2),
                thresholds: None,
            },
            options: PanelOptions {
                legend: LegendConfig {
                    show: true,
                    position: "bottom".to_string(),
                },
                tooltip: TooltipConfig {
                    mode: "multi".to_string(),
                    sort: "none".to_string(),
                },
                time: TimeRangeConfig {
                    from: "now-1h".to_string(),
                    to: "now".to_string(),
                },
            },
        };
        assert_eq!(config.id, "panel-1");
        assert_eq!(config.title, "CPU Usage");
        assert_eq!(config.panel_type, "graph");
    }
    #[test]
    fn test_grid_pos_validation() {
        let pos: _ = GridPos {
            x: 0,
            y: 0,
            w: 24,
            h: 12,
        };
        assert!(pos.w > 0);
        assert!(pos.h > 0);
    }
    #[test]
    fn test_threshold_config() {
        let thresholds: _ = ThresholdsConfig {
            steps: vec![
                ThresholdStep {
                    color: "green".to_string(),
                    value: None,
                },
                ThresholdStep {
                    color: "yellow".to_string(),
                    value: Some(50.0),
                },
                ThresholdStep {
                    color: "red".to_string(),
                    value: Some(80.0),
                },
            ],
        };
        assert_eq!(thresholds.steps.len(), 3);
        assert_eq!(thresholds.steps[1].color, "yellow");
    }
}
use std::collections::{BTreeMap, HashMap};
