// Info Command Module
// Stage 91 Phase 4.1 - 系统信息命令
//
/// 实现 `amberjs info` 命令，显示运行时和系统信息
use std::env;
use std::path::Path;
use super::output_formatter::OutputFormatter;
/// 系统信息结构
#[derive(Debug, Default)]
pub struct SystemInfo {
    /// Amber 版本
    pub amberjs_version: String,
    /// 操作系统
    pub os: String,
    /// CPU 架构
    pub arch: String,
    /// CPU 核心数
    pub cpu_count: usize,
    /// 总内存 (MB)
    pub total_memory_mb: u64,
    /// 可用内存 (MB)
    pub available_memory_mb: u64,
    /// V8 版本
    pub v8_version: String,
    /// Rust 版本
    pub rust_version: String,
    /// 工作目录
    pub cwd: String,
    /// Home 目录
    pub home_dir: String,
    /// 临时目录
    pub temp_dir: String,
    /// 是否在 CI 环境
    pub is_ci: bool,
    /// 是否在 Docker 中
    pub is_docker: bool,
    /// 已安装的全局包数量
    pub global_packages: usize,
}
impl SystemInfo {
    /// 收集系统信息
    pub fn collect() -> Self {
        let mut info = Self::default();
        // Amber 版本
        info.amberjs_version = env!("CARGO_PKG_VERSION").to_string();
        // 操作系统信息
        info.os = Self::get_os_info();
        info.arch = std::env::consts::ARCH.to_string();
        // CPU 信息
        info.cpu_count = num_cpus::get();
        // 内存信息 (简化版本)
        info.total_memory_mb = Self::get_total_memory_mb();
        info.available_memory_mb = Self::get_available_memory_mb();
        // V8 版本
        info.v8_version = Self::get_v8_version();
        // Rust 版本
        info.rust_version = Self::get_rust_version();
        // 目录信息
        info.cwd = env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        info.home_dir = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        info.temp_dir = env::temp_dir().to_string_lossy().to_string();
        // 环境检测
        info.is_ci = Self::detect_ci();
        info.is_docker = Self::detect_docker();
        info
    }
    fn get_os_info() -> String {
        let os_name: _ = std::env::consts::OS;
        let family: _ = std::env::consts::FAMILY;
        // 尝试获取更详细的版本信息
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
            {
                if output.status.success() {
                    let version: _ = String::from_utf8_lossy(&output.stdout);
                    return format!("macOS {}", version.trim());
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        let name: _ = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                        return name.to_string();
                    }
                }
            }
        }
        format!("{} ({})", os_name, family)
    }
    fn get_total_memory_mb() -> u64 {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
            {
                if output.status.success() {
                    if let Ok(bytes) = String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .parse::<u64>()
                    {
                        return bytes / (1024 * 1024);
                    }
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return kb / 1024;
                            }
                        }
                    }
                }
            }
        }
        0
    }
    fn get_available_memory_mb() -> u64 {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("vm_stat").output() {
                if output.status.success() {
                    let content: _ = String::from_utf8_lossy(&output.stdout);
                    let mut free_pages = 0u64;
                    for line in content.lines() {
                        if line.starts_with("Pages free:") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if let Some(pages) = parts.last() {
                                if let Ok(p) = pages.trim_end_matches('.').parse::<u64>() {
                                    free_pages = p;
                                }
                            }
                        }
                    }
                    // macOS page size is typically 4096 bytes
                    return (free_pages * 4096) / (1024 * 1024);
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemAvailable:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return kb / 1024;
                            }
                        }
                    }
                }
            }
        }
        0
    }
    fn get_v8_version() -> String {
        // V8 版本通常在编译时确定
        // rusty_v8 0.22 对应 V8 ~10.x
        "10.x (rusty_v8 0.22)".to_string()
    }
    fn get_rust_version() -> String {
        // 尝试获取 rustc 版本
        if let Ok(output) = std::process::Command::new("rustc")
            .arg("--version")
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
        "unknown".to_string()
    }
    fn detect_ci() -> bool {
        // 检测常见的 CI 环境变量
        env::var("CI").is_ok()
            || env::var("GITHUB_ACTIONS").is_ok()
            || env::var("GITLAB_CI").is_ok()
            || env::var("CIRCLECI").is_ok()
            || env::var("TRAVIS").is_ok()
            || env::var("JENKINS_URL").is_ok()
    }
    fn detect_docker() -> bool {
        // 检测是否在 Docker 容器中
        Path::new("/.dockerenv").exists()
            || std::fs::read_to_string("/proc/1/cgroup")
                .map(|s| s.contains("docker"))
                .unwrap_or(false)
    }
}
/// Info 命令执行器
pub struct InfoCommand {
    formatter: OutputFormatter,
    verbose: bool,
}
impl InfoCommand {
    /// 创建新的 info 命令
    pub fn new(verbose: bool) -> Self {
        Self {
            formatter: OutputFormatter::with_verbose(verbose),
            verbose,
        }
    }
    /// 执行 info 命令
    pub fn execute(&self) -> anyhow::Result<()> {
        let info: _ = SystemInfo::collect();
        self.formatter.print_banner();
        // Runtime Information
        self.formatter.title("Runtime Information");
        self.formatter
            .key_value_with_icon("📦", "Amber Version", &info.amberjs_version);
        self.formatter
            .key_value_with_icon("⚙️", "V8 Engine", &info.v8_version);
        self.formatter
            .key_value_with_icon("🦀", "Rust", &info.rust_version);
        // System Information
        self.formatter.title("System Information");
        self.formatter
            .key_value_with_icon("💻", "Operating System", &info.os);
        self.formatter
            .key_value_with_icon("🔧", "Architecture", &info.arch);
        self.formatter
            .key_value_with_icon("🧠", "CPU Cores", &info.cpu_count.to_string());
        if info.total_memory_mb > 0 {
            self.formatter.key_value_with_icon(
                "💾",
                "Memory",
                &format!(
                    "{} MB total, {} MB available",
                    info.total_memory_mb, info.available_memory_mb
                ),
            );
        }
        // Environment
        self.formatter.title("Environment");
        self.formatter
            .key_value_with_icon("📂", "Working Directory", &info.cwd);
        self.formatter
            .key_value_with_icon("🏠", "Home Directory", &info.home_dir);
        if self.verbose {
            self.formatter
                .key_value_with_icon("📁", "Temp Directory", &info.temp_dir);
        }
        // Environment Detection
        self.formatter.title("Environment Detection");
        if info.is_ci {
            self.formatter
                .key_value_with_icon("🔄", "CI Environment", "Yes");
        } else {
            self.formatter
                .key_value_with_icon("🔄", "CI Environment", "No");
        }
        if info.is_docker {
            self.formatter
                .key_value_with_icon("🐳", "Docker Container", "Yes");
        } else if self.verbose {
            self.formatter
                .key_value_with_icon("🐳", "Docker Container", "No");
        }
        // Features
        self.formatter.title("Available Features");
        self.print_features();
        println!();
        Ok(())
    }
    fn print_features(&self) {
        let features: _ = [
            ("TypeScript", true, "Native TypeScript execution"),
            ("ESM Modules", true, "ES Module support"),
            ("CommonJS", true, "CommonJS module support"),
            ("WebSocket", true, "WebSocket client/server"),
            ("Fetch API", true, "Global fetch function"),
            ("Web APIs", true, "Standard Web APIs"),
            ("File System", true, "Node.js fs compatibility"),
            ("Workers", true, "Web Workers support"),
            ("WASM", true, "WebAssembly support"),
            ("Hot Reload", true, "File watching and hot reload"),
        ];
        for (name, enabled, desc) in features {
            let status: _ = if enabled { "✓" } else { "✗" };
            let status_color: _ = if enabled { "\x1b[32m" } else { "\x1b[31m" };
            if self.formatter.color_enabled {
                println!(
                    "  {}{}\x1b[0m {} {}",
                    status_color,
                    status,
                    name,
                    if self.verbose {
                        format!("- {}", desc)
                    } else {
                        String::new()
                    }
                );
            } else {
                println!(
                    "  {} {} {}",
                    status,
                    name,
                    if self.verbose {
                        format!("- {}", desc)
                    } else {
                        String::new()
                    }
                );
            }
        }
    }
    /// 输出 JSON 格式的信息
    pub fn execute_json(&self) -> anyhow::Result<()> {
        let info: _ = SystemInfo::collect();
        let json: _ = serde_json::json!({
            "amberjs": {
                "version": info.amberjs_version,
                "v8_version": info.v8_version,
            },
            "system": {
                "os": info.os,
                "arch": info.arch,
                "cpu_count": info.cpu_count,
                "total_memory_mb": info.total_memory_mb,
                "available_memory_mb": info.available_memory_mb,
            },
            "environment": {
                "cwd": info.cwd,
                "home_dir": info.home_dir,
                "temp_dir": info.temp_dir,
                "is_ci": info.is_ci,
                "is_docker": info.is_docker,
            }
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
use std::collections::{HashMap, BTreeMap};
use std::fs::File;
use anyhow::{Result, Error};
use std::time::Duration;
    #[test]
    fn test_system_info_collect() {
        let info: _ = SystemInfo::collect();
        assert!(!info.amberjs_version.is_empty());
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_count > 0);
    }
    #[test]
    fn test_info_command_execute() {
        let cmd: _ = InfoCommand::new(false);
        assert!(cmd.execute().is_ok());
    }
    #[test]
    fn test_info_command_json() {
        let cmd: _ = InfoCommand::new(false);
        assert!(cmd.execute_json().is_ok());
    }
}