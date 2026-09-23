// CLI Output Formatter Module
// Stage 91 Phase 4.1 - 开发者体验提升
//
// 提供彩色终端输出、进度条、表格格式化等功能

use std::collections::{BTreeMap, HashMap};
use std::io::{Write, self};
use std::time::Duration;

/// ANSI 颜色代码
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const UNDERLINE: &str = "\x1b[4m";
    // 前景色
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
    // 亮色
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
}
/// 输出格式化器
pub struct OutputFormatter {
    /// 是否启用颜色输出
    pub color_enabled: bool,
    /// 是否为 verbose 模式
    pub verbose: bool,
}
impl Default for OutputFormatter {
    fn default() -> Self {
        Self {
            color_enabled: Self::detect_color_support(),
            verbose: false,
        }
    }
}
impl OutputFormatter {
    /// 创建新的格式化器
    pub fn new() -> Self {
        Self::default()
    }
    /// 创建带 verbose 模式的格式化器
    pub fn with_verbose(verbose: bool) -> Self {
        Self {
            color_enabled: Self::detect_color_support(),
            verbose,
        }
    }
    /// 检测终端是否支持颜色
    fn detect_color_support() -> bool {
        // 检查环境变量
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }
        if let Ok(term) = std::env::var("TERM") {
            if term == "dumb" {
                return false;
            }
        }
        // 检查是否为 TTY
        atty::is(atty::Stream::Stdout)
    }
    /// 应用颜色格式
    fn colorize(&self, text: &str, color: &str) -> String {
        if self.color_enabled {
            format!("{}{}{}", color, text, colors::RESET)
        } else {
            text.to_string()
        }
    }
    // ========== 消息输出方法 ==========
    /// 打印成功消息
    pub fn success(&self, message: &str) {
        println!(
            "{} {}",
            self.colorize("✓", colors::GREEN),
            self.colorize(message, colors::GREEN)
        );
    }
    /// 打印错误消息
    pub fn error(&self, message: &str) {
        eprintln!(
            "{} {}",
            self.colorize("✗", colors::RED),
            self.colorize(message, colors::RED)
        );
    }
    /// 打印警告消息
    pub fn warning(&self, message: &str) {
        println!(
            "{} {}",
            self.colorize("⚠", colors::YELLOW),
            self.colorize(message, colors::YELLOW)
        );
    }
    /// 打印信息消息
    pub fn info(&self, message: &str) {
        println!("{} {}", self.colorize("ℹ", colors::BLUE), message);
    }
    /// 打印调试消息 (仅在 verbose 模式)
    pub fn debug(&self, message: &str) {
        if self.verbose {
            println!(
                "{} {}",
                self.colorize("🔍", colors::DIM),
                self.colorize(message, colors::DIM)
            );
        }
    }
    /// 打印标题
    pub fn title(&self, title: &str) {
        println!();
        println!(
            "{}",
            self.colorize(
                &format!("  {} ", title),
                &format!("{}{}", colors::BOLD, colors::BRIGHT_CYAN)
            )
        );
        println!(
            "{}",
            self.colorize(&"─".repeat(title.len() + 4), colors::DIM)
        );
    }
    /// 打印子标题
    pub fn subtitle(&self, subtitle: &str) {
        println!();
        println!("  {}", self.colorize(subtitle, colors::BOLD));
    }
    // ========== 数据格式化方法 ==========
    /// 格式化键值对
    pub fn key_value(&self, key: &str, value: &str) {
        let formatted_key: _ = self.colorize(key, colors::CYAN);
        println!("  {}: {}", formatted_key, value);
    }
    /// 格式化键值对 (带图标)
    pub fn key_value_with_icon(&self, icon: &str, key: &str, value: &str) {
        let formatted_key: _ = self.colorize(key, colors::CYAN);
        println!("  {} {}: {}", icon, formatted_key, value);
    }
    /// 打印列表项
    pub fn list_item(&self, item: &str) {
        println!("  {} {}", self.colorize("•", colors::DIM), item);
    }
    /// 打印编号列表项
    pub fn numbered_item(&self, num: usize, item: &str) {
        println!(
            "  {}. {}",
            self.colorize(&num.to_string(), colors::YELLOW),
            item
        );
    }
    /// 格式化文件大小
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
    /// 格式化持续时间
    pub fn format_duration(duration: Duration) -> String {
        let secs: _ = duration.as_secs();
        let millis: _ = duration.subsec_millis();
        if secs >= 60 {
            let mins: _ = secs / 60;
            let remaining_secs: _ = secs % 60;
            format!("{}m {}s", mins, remaining_secs)
        } else if secs > 0 {
            format!("{}.{:03}s", secs, millis)
        } else if millis > 0 {
            format!("{}ms", millis)
        } else {
            format!("{}μs", duration.subsec_micros())
        }
    }
    // ========== 表格输出 ==========
    /// 打印简单表格
    pub fn table(&self, headers: &[&str], rows: &[Vec<String>]) {
        // 计算列宽
        let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() && cell.len() > col_widths[i] {
                    col_widths[i] = cell.len();
                }
            }
        }
        // 打印表头
        let header_line: String = headers
            .iter()
            .zip(&col_widths)
            .map(|(h, w)| format!("{:width$}", h, width = *w))
            .collect::<Vec<_>>()
            .join("  ");
        println!("  {}", self.colorize(&header_line, colors::BOLD));
        // 打印分隔线
        let separator: String = col_widths
            .iter()
            .map(|w| "─".repeat(*w))
            .collect::<Vec<_>>()
            .join("──");
        println!("  {}", self.colorize(&separator, colors::DIM));
        // 打印数据行
        for row in rows {
            let row_line: String = row
                .iter()
                .zip(&col_widths)
                .map(|(cell, w)| format!("{:width$}", cell, width = *w))
                .collect::<Vec<_>>()
                .join("  ");
            println!("  {}", row_line);
        }
    }
    // ========== 进度显示 ==========
    /// 简单的进度指示器
    pub fn progress_start(&self, message: &str) {
        print!("{} {} ", self.colorize("⏳", colors::YELLOW), message);
        let _: _ = io::stdout().flush();
    }
    /// 进度完成
    pub fn progress_done(&self) {
        println!("{}", self.colorize("✓", colors::GREEN));
    }
    /// 进度失败
    pub fn progress_fail(&self) {
        println!("{}", self.colorize("✗", colors::RED));
    }
    /// Spinner 动画帧
    const SPINNER_FRAMES: &'static [&'static str] =
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    /// 获取 spinner 帧
    pub fn spinner_frame(index: usize) -> &'static str {
        Self::SPINNER_FRAMES[index % Self::SPINNER_FRAMES.len()]
    }
    // ========== 框架输出 ==========
    /// 打印带边框的消息框
    pub fn box_message(&self, title: &str, content: &[&str]) {
        let max_len: _ = content
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max(title.len());
        let border_width: _ = max_len + 4;
        // 顶部边框
        println!("  ╭{}╮", "─".repeat(border_width));
        // 标题
        println!(
            "  │ {}{}{} │",
            self.colorize(title, colors::BOLD),
            " ".repeat(max_len - title.len()),
            " ".repeat(2)
        );
        // 分隔线
        println!("  ├{}┤", "─".repeat(border_width));
        // 内容
        for line in content {
            println!("  │  {}{} │", line, " ".repeat(max_len - line.len() + 1));
        }
        // 底部边框
        println!("  ╰{}╯", "─".repeat(border_width));
    }
    // ========== 版本信息 ==========
    /// 打印 Amber 版本横幅
    pub fn print_banner(&self) {
        let banner: _ = r#"
   ____            _
  |  _ \          (_)
  | |_) | ___  ___ _ ___
  |  _ < / _ \/ _ \ / __|
  | |_) |  __/  __/ \__ \
  |____/ \___|\___| |___/
                 _/ |
                |__/
"#;
        println!("{}", self.colorize(banner, colors::BRIGHT_CYAN));
        println!(
            "  {} {} - {}",
            self.colorize("Amber", colors::BOLD),
            self.colorize("v0.1.0", colors::BRIGHT_GREEN),
            self.colorize(
                "High-performance JavaScript/TypeScript Runtime",
                colors::DIM
            )
        );
        println!();
    }
}
/// 简单的 atty 替代实现
mod atty {
    pub enum Stream {
        Stdout,
    }
    pub fn is(_stream: Stream) -> bool {
        // 在大多数 Unix 系统上检查 stdout 是否为 TTY
        #[cfg(unix)]
        {
            unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
        }
        #[cfg(windows)]
        {
            // Windows 上默认返回 true
            true
        }
        #[cfg(not(any(unix, windows)))]
        {
            true
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_format_size() {
        assert_eq!(OutputFormatter::format_size(512), "512 B");
        assert_eq!(OutputFormatter::format_size(1024), "1.00 KB");
        assert_eq!(OutputFormatter::format_size(1536), "1.50 KB");
        assert_eq!(OutputFormatter::format_size(1048576), "1.00 MB");
        assert_eq!(OutputFormatter::format_size(1073741824), "1.00 GB");
    }
    #[test]
    fn test_format_duration() {
        assert_eq!(
            OutputFormatter::format_duration(Duration::from_micros(500)),
            "500μs"
        );
        assert_eq!(
            OutputFormatter::format_duration(Duration::from_millis(100)),
            "100ms"
        );
        assert_eq!(
            OutputFormatter::format_duration(Duration::from_secs(5)),
            "5.000s"
        );
        assert_eq!(
            OutputFormatter::format_duration(Duration::from_secs(65)),
            "1m 5s"
        );
    }
    #[test]
    fn test_output_formatter_new() {
        let formatter: _ = OutputFormatter::new();
        assert!(!formatter.verbose);
    }
    #[test]
    fn test_output_formatter_with_verbose() {
        let formatter: _ = OutputFormatter::with_verbose(true);
        assert!(formatter.verbose);
    }
    #[test]
    fn test_spinner_frames() {
        assert_eq!(OutputFormatter::spinner_frame(0), "⠋");
        assert_eq!(OutputFormatter::spinner_frame(10), "⠋"); // 循环
    }
}