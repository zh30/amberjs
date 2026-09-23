// Visualization Module - Advanced Chart and Graph Components
//
// This module provides high-performance visualization components for real-time data:
// - Line charts for time series data
// - Bar charts for categorical data
// - Pie charts for proportional data
// - Heatmaps for density visualization
// - Topology graphs for system architecture
// - Dependency graphs for module relationships
// - Trace graphs for request flows
//
// # Examples
//
// ```rust
// use amberjs::observability::visualization::{
//     LineChartBuilder, BarChartBuilder, PieChartBuilder,
//     TopologyGraphBuilder, DependencyGraphBuilder
// };
//
// let chart: _ = LineChartBuilder::new()
//     .title("CPU Usage")
//     .data(vec![10.0, 20.0, 30.0])
//     .build()?;
// ```
pub mod charts;
pub mod graphs;
pub use charts::*;
pub use graphs::*;
use serde::{Deserialize, Serialize};
/// Base visualization trait
pub trait Visualizable {
    fn render(&self) -> String;
    fn update_data(&mut self, data: Vec<f64>) -> Result<()>;
    fn get_config(&self) -> &VisualizationConfig;
}
/// Base configuration for all visualizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationConfig {
    /// Visualization ID
    pub id: String,
    /// Title
    pub title: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Background color
    pub background_color: String,
    /// Text color
    pub text_color: String,
    /// Font family
    pub font_family: String,
    /// Font size
    pub font_size: u32,
    /// Padding
    pub padding: Padding,
    /// Animation settings
    pub animation: AnimationConfig,
}
/// Padding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Padding {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}
/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    pub enabled: bool,
    pub duration: u32,
    pub easing: String,
}
/// Color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
}
/// Data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub color: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
/// Data series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: Option<String>,
    pub visible: bool,
    pub line_style: LineStyle,
}
/// Line style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineStyle {
    pub width: f64,
    pub dash_array: Option<Vec<f64>>,
    pub cap: String,
    pub join: String,
}
/// Marker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerConfig {
    pub enabled: bool,
    pub size: u32,
    pub symbol: String,
    pub color: String,
}
/// Axis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    pub show: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub tick_count: u32,
    pub label: String,
    pub unit: Option<String>,
    pub format: String,
}
/// Legend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegendConfig {
    pub show: bool,
    pub position: String,
    pub align: String,
}
/// Tooltip configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipConfig {
    pub show: bool,
    pub trigger: String,
    pub format: String,
}
/// Grid configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    pub show: bool,
    pub color: String,
    pub opacity: f64,
    pub line_width: f64,
}
/// Zoom configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoomConfig {
    pub enabled: bool,
    pub zoom_on_wheel: bool,
    pub zoom_on_drag: bool,
    pub max_zoom: f64,
}
/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: String, // "svg", "png", "pdf"
    pub quality: u8,
    pub transparent: bool,
}
/// Event handler
#[derive(Debug, Clone)]
pub struct EventHandler {
    pub event_type: String,
    pub callback: Box<dyn Fn(&serde_json::Value) + Send + Sync>,
}
/// Interactive feature
#[derive(Debug, Clone)]
pub struct InteractiveFeature {
    pub feature_type: String,
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
}
/// Responsive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsiveConfig {
    pub enabled: bool,
    pub breakpoints: HashMap<String, u32>,
}
impl Default for VisualizationConfig {
    fn default() -> Self {
        Self {
            id: "viz-1".to_string(),
            title: "Visualization".to_string(),
            width: 800,
            height: 600,
            background_color: "#ffffff".to_string(),
            text_color: "#333333".to_string(),
            font_family: "Arial, sans-serif".to_string(),
            font_size: 12,
            padding: Padding {
                top: 20,
                right: 20,
                bottom: 40,
                left: 60,
            },
            animation: AnimationConfig {
                enabled: true,
                duration: 300,
                easing: "easeInOutQuart".to_string(),
            },
        }
    }
}
impl Default for Padding {
    fn default() -> Self {
        Self {
            top: 20,
            right: 20,
            bottom: 40,
            left: 60,
        }
    }
}
impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration: 300,
            easing: "easeInOutQuart".to_string(),
        }
    }
}
impl Default for LineStyle {
    fn default() -> Self {
        Self {
            width: 2.0,
            dash_array: None,
            cap: "round".to_string(),
            join: "round".to_string(),
        }
    }
}
impl Default for MarkerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 6,
            symbol: "circle".to_string(),
            color: "#3b82f6".to_string(),
        }
    }
}
impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            show: true,
            min: None,
            max: None,
            tick_count: 5,
            label: "".to_string(),
            unit: None,
            format: "{}".to_string(),
        }
    }
}
impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            show: true,
            position: "bottom".to_string(),
            align: "center".to_string(),
        }
    }
}
impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            show: true,
            trigger: "item".to_string(),
            format: "{}".to_string(),
        }
    }
}
impl Default for GridConfig {
    fn default() -> Self {
        Self {
            show: true,
            color: "#e5e7eb".to_string(),
            opacity: 0.5,
            line_width: 1.0,
        }
    }
}
impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            zoom_on_wheel: true,
            zoom_on_drag: false,
            max_zoom: 5.0,
        }
    }
}
impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: "svg".to_string(),
            quality: 90,
            transparent: false,
        }
    }
}
impl Default for ResponsiveConfig {
    fn default() -> Self {
        let mut breakpoints = HashMap::new();
        breakpoints.insert("xs".to_string(), 480);
        breakpoints.insert("sm".to_string(), 768);
        breakpoints.insert("md".to_string(), 1024);
        breakpoints.insert("lg".to_string(), 1280);
        breakpoints.insert("xl".to_string(), 1920);
        Self {
            enabled: true,
            breakpoints,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_visualization_config_default() {
        let config: _ = VisualizationConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.font_size, 12);
    }
    #[test]
    fn test_data_point_creation() {
        let point: _ = DataPoint {
            x: 10.0,
            y: 20.0,
            label: Some("Test".to_string()),
            color: Some("#3b82f6".to_string()),
            metadata: HashMap::new(),
        };
        assert_eq!(point.x, 10.0);
        assert_eq!(point.y, 20.0);
        assert_eq!(point.label, Some("Test".to_string()));
    }
    #[test]
    fn test_data_series_creation() {
        let series: _ = DataSeries {
            name: "Series 1".to_string(),
            data: vec![
                DataPoint {
                    x: 0.0,
                    y: 10.0,
                    label: None,
                    color: None,
                    metadata: HashMap::new(),
                },
                DataPoint {
                    x: 1.0,
                    y: 20.0,
                    label: None,
                    color: None,
                    metadata: HashMap::new(),
                },
            ],
            color: Some("#3b82f6".to_string()),
            visible: true,
            line_style: LineStyle::default(),
        };
        assert_eq!(series.name, "Series 1");
        assert_eq!(series.data.len(), 2);
        assert!(series.visible);
    }
    #[test]
    fn test_axis_config() {
        let axis: _ = AxisConfig {
            show: true,
            min: Some(0.0),
            max: Some(100.0),
            tick_count: 10,
            label: "Value".to_string(),
            unit: Some("%".to_string()),
            format: "{.1f}".to_string(),
        };
        assert_eq!(axis.min, Some(0.0));
        assert_eq!(axis.max, Some(100.0));
        assert_eq!(axis.unit, Some("%".to_string()));
    }
    #[test]
    fn test_line_style() {
        let style: _ = LineStyle {
            width: 3.0,
            dash_array: Some(vec![5.0, 5.0]),
            cap: "square".to_string(),
            join: "bevel".to_string(),
        };
        assert_eq!(style.width, 3.0);
        assert_eq!(style.dash_array, Some(vec![5.0, 5.0]));
    }
    #[test]
    fn test_color_palette() {
        let palette: _ = ColorPalette {
            primary: "#3b82f6".to_string(),
            secondary: "#10b981".to_string(),
            tertiary: "#f59e0b".to_string(),
            accent: "#ef4444".to_string(),
            success: "#22c55e".to_string(),
            warning: "#eab308".to_string(),
            error: "#ef4444".to_string(),
        };
        assert_eq!(palette.primary, "#3b82f6");
        assert_eq!(palette.success, "#22c55e");
    }
    #[test]
    fn test_padding_default() {
        let padding: _ = Padding::default();
        assert_eq!(padding.top, 20);
        assert_eq!(padding.bottom, 40);
        assert_eq!(padding.left, 60);
        assert_eq!(padding.right, 20);
    }
    #[test]
    fn test_marker_config() {
        let marker: _ = MarkerConfig {
            enabled: true,
            size: 8,
            symbol: "diamond".to_string(),
            color: "#ef4444".to_string(),
        };
        assert!(marker.enabled);
        assert_eq!(marker.size, 8);
        assert_eq!(marker.symbol, "diamond");
    }
    #[test]
    fn test_responsive_config() {
        let responsive: _ = ResponsiveConfig::default();
        assert!(responsive.enabled);
        assert!(responsive.breakpoints.contains_key("md"));
        assert_eq!(responsive.breakpoints["lg"], 1280);
    }
    #[test]
    fn test_export_config() {
        let export: _ = ExportConfig {
            format: "png".to_string(),
            quality: 95,
            transparent: true,
        };
        assert_eq!(export.format, "png");
        assert_eq!(export.quality, 95);
        assert!(export.transparent);
    }
    #[test]
    fn test_grid_config() {
        let grid: _ = GridConfig {
            show: false,
            color: "#cccccc".to_string(),
            opacity: 0.3,
            line_width: 2.0,
        };
        assert!(!grid.show);
        assert_eq!(grid.opacity, 0.3);
    }
    #[test]
    fn test_legend_config() {
        let legend: _ = LegendConfig {
            show: false,
            position: "right".to_string(),
            align: "start".to_string(),
        };
        assert!(!legend.show);
        assert_eq!(legend.position, "right");
    }
    #[test]
    fn test_tooltip_config() {
        let tooltip: _ = TooltipConfig {
            show: true,
            trigger: "axis".to_string(),
            format: "{.2f}".to_string(),
        };
        assert!(tooltip.show);
        assert_eq!(tooltip.trigger, "axis");
    }
}
use std::collections::{BTreeMap, HashMap};
