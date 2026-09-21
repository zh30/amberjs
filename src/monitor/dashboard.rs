// Web 仪表板模块
// 提供实时性能监控 Web 界面

use crate::monitor::alerts::{AlertInstance, AlertSystem};
use crate::monitor::data_store::{DataStore, ExportFormat, QueryCondition};
use crate::monitor::performance_monitor::{MetricType, MetricValue};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::time::{Duration, Instant};

/// Web 仪表板配置
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// HTTP 端口
    pub port: u16,
    /// 主机地址
    pub host: String,
    /// 静态文件路径
    pub static_files_path: String,
    /// 实时更新间隔
    pub update_interval: Duration,
    /// 最大并发连接数
    pub max_connections: usize,
    /// 启用 CORS
    pub enable_cors: bool,
}
/// 图表配置
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// 图表 ID
    pub id: String,
    /// 图表标题
    pub title: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 图表类型
    pub chart_type: ChartType,
    /// 时间范围
    pub time_range: Duration,
    /// 颜色
    pub color: String,
    /// 是否启用
    pub enabled: bool,
}
/// 图表类型
#[derive(Debug, Clone)]
pub enum ChartType {
    Line,
    Bar,
    Area,
    Pie,
    Gauge,
}
/// 仪表板布局
#[derive(Debug, Clone)]
pub struct DashboardLayout {
    /// 布局名称
    pub name: String,
    /// 布局描述
    pub description: String,
    /// 图表配置列表
    pub charts: Vec<ChartConfig>,
    /// 布局配置
    pub config: LayoutConfig,
}
/// 布局配置
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// 列数
    pub columns: usize,
    /// 行高
    pub row_height: u32,
    /// 响应式断点
    pub breakpoints: HashMap<String, BreakpointConfig>,
}
/// 断点配置
#[derive(Debug, Clone)]
pub struct BreakpointConfig {
    /// 断点名称
    pub name: String,
    /// 最小宽度
    pub min_width: u32,
    /// 最大宽度
    pub max_width: u32,
    /// 列数
    pub columns: usize,
}
/// Web 仪表板
#[derive(Debug)]
pub struct WebDashboard {
    /// 配置
    config: DashboardConfig,
    /// 数据存储引用
    data_store: Arc<DataStore>,
    /// 告警系统引用
    alert_system: Arc<AlertSystem>,
    /// 仪表板布局
    layout: Arc<Mutex<DashboardLayout>>,
    /// 连接统计
    connection_stats: Arc<Mutex<ConnectionStats>>,
    /// 最后更新时间
    last_update: Arc<Mutex<Instant>>,
}
/// 连接统计
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// 当前连接数
    pub current_connections: usize,
    /// 总连接数
    pub total_connections: u64,
    /// 连接峰值
    pub peak_connections: usize,
    /// 断开连接数
    pub disconnected_count: u64,
}
/// 仪表板数据
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// 实时指标
    pub real_time_metrics: Vec<MetricValue>,
    /// 聚合指标
    pub aggregated_metrics: HashMap<MetricType, f64>,
    /// 活跃告警
    pub active_alerts: Vec<AlertInstance>,
    /// 连接统计
    pub connection_stats: ConnectionStats,
    /// 更新时间戳
    pub updated_at: u64,
}
/// API 响应
#[derive(Debug, Clone)]
pub struct ApiResponse<T> {
    /// 成功状态
    pub success: bool,
    /// 数据
    pub data: Option<T>,
    /// 错误信息
    pub error: Option<String>,
    /// 时间戳
    pub timestamp: u64,
}
/// 导出配置
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// 格式
    pub format: ExportFormat,
    /// 时间范围
    pub time_range: Option<Duration>,
    /// 包含的指标
    pub included_metrics: Vec<MetricType>,
}
impl WebDashboard {
    /// 创建新的 Web 仪表板
    pub fn new(
        config: DashboardConfig,
        data_store: Arc<DataStore>,
        alert_system: Arc<AlertSystem>,
    ) -> Self {
        let layout: _ = DashboardLayout {
            name: "Default Dashboard".to_string(),
            description: "Default Amber monitoring dashboard".to_string(),
            charts: Self::create_default_charts(),
            config: Self::create_default_layout_config(),
        };
        Self {
            config,
            data_store,
            alert_system,
            layout: Arc::new(Mutex::new(layout)),
            connection_stats: Arc::new(Mutex::new(ConnectionStats {
                current_connections: 0,
                total_connections: 0,
                peak_connections: 0,
                disconnected_count: 0,
            })),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }
    /// 创建默认配置
    pub fn with_default_config(data_store: Arc<DataStore>, alert_system: Arc<AlertSystem>) -> Self {
        let config: _ = DashboardConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            static_files_path: "./static".to_string(),
            update_interval: Duration::from_secs(5),
            max_connections: 1000,
            enable_cors: true,
        };
        Self::new(config, data_store, alert_system)
    }
    /// 创建默认图表配置
    fn create_default_charts() -> Vec<ChartConfig> {
        vec![
            ChartConfig {
                id: "cpu_usage".to_string(),
                title: "CPU Usage".to_string(),
                metric_type: MetricType::CpuUsage,
                chart_type: ChartType::Line,
                time_range: Duration::from_secs(3600), // 1小时
                color: "#FF6384".to_string(),
                enabled: true,
            },
            ChartConfig {
                id: "memory_usage".to_string(),
                title: "Memory Usage".to_string(),
                metric_type: MetricType::MemoryUsage,
                chart_type: ChartType::Area,
                time_range: Duration::from_secs(3600),
                color: "#36A2EB".to_string(),
                enabled: true,
            },
            ChartConfig {
                id: "execution_time".to_string(),
                title: "Execution Time".to_string(),
                metric_type: MetricType::ExecutionTime,
                chart_type: ChartType::Bar,
                time_range: Duration::from_secs(1800), // 30分钟
                color: "#FFCE56".to_string(),
                enabled: true,
            },
            ChartConfig {
                id: "cache_hit_rate".to_string(),
                title: "Cache Hit Rate".to_string(),
                metric_type: MetricType::CacheHitRate,
                chart_type: ChartType::Gauge,
                time_range: Duration::from_secs(1800),
                color: "#4BC0C0".to_string(),
                enabled: true,
            },
        ]
    }
    /// 创建默认布局配置
    fn create_default_layout_config() -> LayoutConfig {
        let mut breakpoints = HashMap::new();
        breakpoints.insert(
            "lg".to_string(),
            BreakpointConfig {
                name: "Large".to_string(),
                min_width: 1200,
                max_width: u32::MAX,
                columns: 3,
            },
        );
        breakpoints.insert(
            "md".to_string(),
            BreakpointConfig {
                name: "Medium".to_string(),
                min_width: 768,
                max_width: 1199,
                columns: 2,
            },
        );
        breakpoints.insert(
            "sm".to_string(),
            BreakpointConfig {
                name: "Small".to_string(),
                min_width: 0,
                max_width: 767,
                columns: 1,
            },
        );
        LayoutConfig {
            columns: 3,
            row_height: 300,
            breakpoints,
        }
    }
    /// 启动 Web 服务器
    pub fn start_server(&self) -> Result<(), String> {
        // 简化实现，实际应该启动 HTTP 服务器
        // 这里仅模拟服务器启动
        println!(
            "Starting Amber Web Dashboard on {}:{}",
            self.config.host, self.config.port
        );
        Ok(())
    }
    /// 停止 Web 服务器
    pub fn stop_server(&self) -> Result<(), String> {
        // 简化实现
        println!("Stopping Amber Web Dashboard");
        Ok(())
    }
    /// 获取仪表板数据
    pub fn get_dashboard_data(&self) -> Result<DashboardData, String> {
        let current_time: _ = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // 获取实时指标
        let real_time_metrics: _ = self.data_store.get_real_time_metrics()?;
        // 获取聚合指标（简化实现）
        let mut aggregated_metrics = HashMap::new();
        for metric in &real_time_metrics {
            aggregated_metrics.insert(metric.metric_type.clone(), metric.value);
        }
        // 获取活跃告警
        let active_alerts: _ = self.alert_system.get_active_alerts()?;
        // 获取连接统计
        let connection_stats: _ = {
            let stats: _ = self.connection_stats.lock().map_err(|e| e.to_string())?;
            stats.clone()
        };
        // 更新最后更新时间
        {
            let mut last_update = self.last_update.lock().map_err(|e| e.to_string())?;
            *last_update = Instant::now();
        }
        Ok(DashboardData {
            real_time_metrics,
            aggregated_metrics,
            active_alerts,
            connection_stats,
            updated_at: current_time,
        })
    }
    /// 获取图表数据
    pub fn get_chart_data(&self, chart_id: &str) -> Result<ChartData, String> {
        let layout: _ = self.layout.lock().map_err(|e| e.to_string())?;
        // 查找图表配置
        let chart_config: _ = layout
            .charts
            .iter()
            .find(|chart| chart.id == chart_id && chart.enabled)
            .ok_or_else(|| format!("Chart '{}' not found or disabled", chart_id))?;
        // 查询数据
        let condition: _ = QueryCondition {
            metric_type: Some(chart_config.metric_type.clone()),
            start_time: None,
            end_time: None,
            tag_filters: HashMap::new(),
            limit: Some(100), // 限制返回数据点数量
        };
        let data_points: _ = self.data_store.query(condition)?;
        // 转换数据格式
        let chart_data: _ = self.convert_to_chart_data(&data_points, chart_config)?;
        Ok(chart_data)
    }
    /// 转换数据为图表格式
    fn convert_to_chart_data(
        &self,
        data_points: &[MetricValue],
        chart_config: &ChartConfig,
    ) -> Result<ChartData, String> {
        let mut labels = Vec::new();
        let mut values = Vec::new();
        for metric in data_points {
            labels.push(metric.timestamp.to_string());
            values.push(metric.value);
        }
        Ok(ChartData {
            chart_id: chart_config.id.clone(),
            chart_type: chart_config.chart_type.clone(),
            title: chart_config.title.clone(),
            labels,
            datasets: vec![Dataset {
                label: chart_config.title.clone(),
                data: values,
                color: chart_config.color.clone(),
            }],
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    /// 更新布局
    pub fn update_layout(&self, layout: DashboardLayout) -> Result<(), String> {
        let mut current_layout = self.layout.lock().map_err(|e| e.to_string())?;
        *current_layout = layout;
        Ok(())
    }
    /// 获取当前布局
    pub fn get_layout(&self) -> Result<DashboardLayout, String> {
        let layout: _ = self.layout.lock().map_err(|e| e.to_string())?;
        Ok(layout.clone())
    }
    /// 导出仪表板数据
    pub fn export_dashboard(&self, export_config: ExportConfig) -> Result<String, String> {
        let condition: _ = QueryCondition {
            metric_type: None,
            start_time: None,
            end_time: None,
            tag_filters: HashMap::new(),
            limit: None,
        };
        self.data_store.export(condition, export_config.format)
    }
    /// 生成 HTML 仪表板
    pub fn generate_html(&self) -> Result<String, String> {
        let dashboard_data: _ = self.get_dashboard_data()?;
        let layout: _ = self.get_layout()?;
        let mut html = String::new();
        // HTML 头部
        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html lang=\"en\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str(
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n",
        );
        html.push_str("<title>Amber Monitoring Dashboard</title>\n");
        html.push_str("<script src=\"https://cdn.jsdelivr.net/npm/chart.js\"></script>\n");
        html.push_str("<style>\n");
        html.push_str(&Self::generate_css());
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        // 仪表板标题
        html.push_str(&format!(
            "<div class=\"dashboard-header\">\n<h1>{}</h1>\n<p>{}</p>\n</div>\n",
            layout.name, layout.description
        ));
        // 统计卡片
        html.push_str("<div class=\"stats-grid\">\n");
        html.push_str(&format!(
            "<div class=\"stat-card\">\n<h3>Active Alerts</h3>\n<p class=\"stat-value\">{}</p>\n</div>\n",
            dashboard_data.active_alerts.len()));
        html.push_str(&format!(
            "<div class=\"stat-card\">\n<h3>Connections</h3>\n<p class=\"stat-value\">{}</p>\n</div>\n",
            dashboard_data.connection_stats.current_connections
        ));
        html.push_str(&format!(
            "<div class=\"stat-card\">\n<h3>Metrics</h3>\n<p class=\"stat-value\">{}</p>\n</div>\n",
            dashboard_data.real_time_metrics.len()
        ));
        html.push_str("</div>\n");
        // 图表区域
        html.push_str("<div class=\"charts-grid\">\n");
        for chart_config in &layout.charts {
            if !chart_config.enabled {
                continue;
            }
            html.push_str(&format!(
                "<div class=\"chart-container\">\n<h3>{}</h3>\n<canvas id=\"{}\"></canvas>\n</div>\n",
                chart_config.title, chart_config.id
            ));
        }
        html.push_str("</div>\n");
        // 告警区域
        if !dashboard_data.active_alerts.is_empty() {
            html.push_str("<div class=\"alerts-section\">\n<h2>Active Alerts</h2>\n<ul class=\"alerts-list\">\n");
            for alert in &dashboard_data.active_alerts {
                html.push_str(&format!(
                    "<li class=\"alert alert-{}\">{}</li>\n",
                    alert.severity.as_str().to_lowercase(),
                    self.escape_html(&alert.message)
                ));
            }
            html.push_str("</ul>\n</div>\n");
        }
        // JavaScript
        html.push_str("<script>\n");
        html.push_str(&Self::generate_javascript(&layout));
        html.push_str("</script>\n");
        html.push_str("</body>\n</html>\n");
        Ok(html)
    }
    /// HTML 转义函数
    fn escape_html(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
    /// 生成 CSS 样式
    fn generate_css() -> String {
        r#"
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 0;
            background-color: #f5f5f5;
        }
        .dashboard-header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 2rem;
            text-align: center;
        }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            padding: 2rem;
        }
        .stat-card {
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            text-align: center;
        }
        .stat-value {
            font-size: 2rem;
            font-weight: bold;
            color: #667eea;
            margin: 0;
        }
        .charts-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 2rem;
            padding: 0 2rem 2rem;
        }
        .chart-container {
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .alerts-section {
            background: white;
            margin: 2rem;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .alerts-list {
            list-style: none;
            padding: 0;
            margin: 0;
        }
        .alert {
            padding: 0.75rem;
            margin: 0.5rem 0;
            border-radius: 4px;
            border-left: 4px solid;
        }
        .alert-critical {
            background-color: #fee;
            border-color: #e53e3e;
            color: #742a2a;
        }
        .alert-warning {
            background-color: #fef5e7;
            border-color: #dd6b20;
            color: #7b341e;
        }
        .alert-info {
            background-color: #ebf8ff;
            border-color: #3182ce;
            color: #2c5282;
        }
        "#
        .to_string()
    }
    /// 生成 JavaScript 代码
    fn generate_javascript(layout: &DashboardLayout) -> String {
        let mut js = String::new();
        // 图表初始化
        js.push_str("const charts = {};\n");
        for chart_config in &layout.charts {
            if !chart_config.enabled {
                continue;
            }
            js.push_str(&format!(
                r#"
const ctx_{} = document.getElementById('{}').getContext('2d');
charts['{}'] = new Chart(ctx_{}, {{
    type: '{}',
    data: {{
        labels: [],
        datasets: [{{
            label: '{}',
            data: [],
            borderColor: '{}',
            backgroundColor: '{}',
            fill: {}
        }}]
    }},
    options: {{
        responsive: true,
        maintainAspectRatio: false,
        scales: {{
            y: {{
                beginAtZero: true
            }}
        }}
    }}
}});
"#,
                chart_config.id,
                chart_config.id,
                chart_config.id,
                chart_config.id,
                match &chart_config.chart_type {
                    ChartType::Line => "line",
                    ChartType::Bar => "bar",
                    ChartType::Area => "line",
                    ChartType::Pie => "pie",
                    ChartType::Gauge => "doughnut",
                },
                chart_config.title,
                chart_config.color,
                chart_config.color,
                match &chart_config.chart_type {
                    ChartType::Area => "true",
                    ChartType::Pie => "false",
                    ChartType::Gauge => "false",
                    _ => "false",
                }
            ));
        }
        // 数据更新函数 - 使用安全的 DOM 操作
        js.push_str(
            r#"
async function updateDashboard() {
    try {
        const response = await fetch('/api/dashboard');
        const data = await response.json();
        // 更新统计卡片
        const statCards = document.querySelectorAll('.stat-value');
        if (statCards[0]) statCards[0].textContent = data.active_alerts.length;
        if (statCards[1]) statCards[1].textContent = data.connection_stats.current_connections;
        if (statCards[2]) statCards[2].textContent = data.real_time_metrics.length;
        // 更新告警列表 - 使用安全的 DOM 操作，避免 innerHTML
        const alertsList = document.querySelector('.alerts-list');
        if (alertsList) {
            // 清空现有内容
            while (alertsList.firstChild) {
                alertsList.removeChild(alertsList.firstChild);
            }
            // 创建新的告警项
            data.active_alerts.forEach(alert => {
                const li = document.createElement('li');
                li.className = 'alert alert-' + alert.severity.toLowerCase();
                // 使用 textContent 安全设置文本内容
                li.textContent = alert.message;
                alertsList.appendChild(li);
            });
        }
    } catch (error) {
        console.error('Error updating dashboard:', error);
    }
}
// 每5秒更新一次数据
setInterval(updateDashboard, 5000);
updateDashboard(); // 立即更新一次
"#,
        );
        js
    }
    /// 获取连接统计
    pub fn get_connection_stats(&self) -> Result<ConnectionStats, String> {
        let stats: _ = self.connection_stats.lock().map_err(|e| e.to_string())?;
        Ok(stats.clone())
    }
    /// 更新连接计数
    pub fn update_connection_count(&self, delta: isize) -> Result<(), String> {
        let mut stats = self.connection_stats.lock().map_err(|e| e.to_string())?;
        stats.current_connections = stats.current_connections.saturating_add(delta as usize);
        if delta > 0 {
            stats.total_connections += delta as u64;
            if stats.current_connections > stats.peak_connections {
                stats.peak_connections = stats.current_connections;
            }
        } else if delta < 0 {
            stats.disconnected_count += (-delta) as u64;
        }
        Ok(())
    }
}
/// 图表数据
#[derive(Debug, Clone)]
pub struct ChartData {
    pub chart_id: String,
    pub chart_type: ChartType,
    pub title: String,
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
    pub updated_at: u64,
}
/// 数据集
#[derive(Debug, Clone)]
pub struct Dataset {
    pub label: String,
    pub data: Vec<f64>,
    pub color: String,
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_dashboard_creation() {
        let data_store: _ = Arc::new(Mutex::new(DataStore::with_default_config()));
        let alert_system: _ = Arc::new(Mutex::new(AlertSystem::with_default_config()));
        let dashboard: _ = WebDashboard::with_default_config(data_store, alert_system);
        assert!(dashboard.get_layout().is_ok());
    }
    #[test]
    fn test_get_dashboard_data() {
        let data_store: _ = Arc::new(Mutex::new(DataStore::with_default_config()));
        let alert_system: _ = Arc::new(Mutex::new(AlertSystem::with_default_config()));
        let dashboard: _ = WebDashboard::with_default_config(data_store, alert_system);
        assert!(dashboard.get_dashboard_data().is_ok());
    }
    #[test]
    fn test_generate_html() {
        let data_store: _ = Arc::new(Mutex::new(DataStore::with_default_config()));
        let alert_system: _ = Arc::new(Mutex::new(AlertSystem::with_default_config()));
        let dashboard: _ = WebDashboard::with_default_config(data_store, alert_system);
        let html: _ = dashboard.generate_html().unwrap();
        assert!(html.contains("<html"));
        assert!(html.contains("Amber Monitoring Dashboard"));
        assert!(html.contains("</html>"));
    }
    #[test]
    fn test_escape_html() {
        let data_store: _ = Arc::new(Mutex::new(DataStore::with_default_config()));
        let alert_system: _ = Arc::new(Mutex::new(AlertSystem::with_default_config()));
        let dashboard: _ = WebDashboard::with_default_config(data_store, alert_system);
        let input: _ = "<script>alert('xss')</script>";
        let output: _ = dashboard.escape_html(input);
        assert!(!output.contains("<script>"));
        assert!(output.contains("&lt));script&gt;"));
    }
    #[test]
    fn test_update_layout() {
        let data_store: _ = Arc::new(Mutex::new(DataStore::with_default_config()));
        let alert_system: _ = Arc::new(Mutex::new(AlertSystem::with_default_config()));
        let dashboard: _ = WebDashboard::with_default_config(data_store, alert_system);
        let layout: _ = DashboardLayout {
            name: "Test Layout".to_string(),
            description: "Test description".to_string(),
            charts: Vec::new(),
            config: WebDashboard::create_default_layout_config(),
        };
        assert!(dashboard.update_layout(layout.clone()).is_ok());
        let retrieved_layout: _ = dashboard.get_layout().unwrap();
        assert_eq!(retrieved_layout.name, "Test Layout");
    }
    #[test]
    fn test_chart_config_creation() {
        let charts: _ = WebDashboard::create_default_charts();
        assert_eq!(charts.len(), 4);
        assert_eq!(charts[0].id, "cpu_usage");
        assert_eq!(charts[1].id, "memory_usage");
        assert_eq!(charts[2].id, "execution_time");
        assert_eq!(charts[3].id, "cache_hit_rate");
    }
    #[test]
    fn test_connection_stats() {
        let data_store: _ = Arc::new(Mutex::new(DataStore::with_default_config()));
        let alert_system: _ = Arc::new(Mutex::new(AlertSystem::with_default_config()));
        let dashboard: _ = WebDashboard::with_default_config(data_store, alert_system);
        // 增加连接
        dashboard.update_connection_count(1).unwrap();
        dashboard.update_connection_count(2).unwrap();
        let stats: _ = dashboard.get_connection_stats().unwrap();
        assert_eq!(stats.current_connections, 3);
        assert_eq!(stats.total_connections, 3);
        assert_eq!(stats.peak_connections, 3);
        // 减少连接
        dashboard.update_connection_count(-1).unwrap();
        let stats: _ = dashboard.get_connection_stats().unwrap();
        assert_eq!(stats.current_connections, 2);
        assert_eq!(stats.disconnected_count, 1);
    }
}
