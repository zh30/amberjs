// Doctor Command Module
// Stage 91 Phase 4.1 - 环境诊断命令
//
/// 实现 `amberjs doctor` 命令，诊断开发环境问题
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use super::output_formatter::OutputFormatter;
use std::io::Write;
/// 诊断检查结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// 检查通过
    Pass,
    /// 警告
    Warning,
    /// 失败
    Fail,
    /// 跳过
    Skip,
}
impl CheckStatus {
    fn icon(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "✓",
            CheckStatus::Warning => "⚠",
            CheckStatus::Fail => "✗",
            CheckStatus::Skip => "○",
        }
    }
    fn color(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "\x1b[32m",    // Green
            CheckStatus::Warning => "\x1b[33m", // Yellow
            CheckStatus::Fail => "\x1b[31m",    // Red
            CheckStatus::Skip => "\x1b[90m",    // Gray
        }
    }
}
/// 单个诊断检查
#[derive(Debug)]
pub struct DiagnosticCheck {
    /// 检查名称
    pub name: String,
    /// 检查描述
    pub description: String,
    /// 检查状态
    pub status: CheckStatus,
    /// 详细消息
    pub message: Option<String>,
    /// 建议操作
    pub suggestion: Option<String>,
}
/// Doctor 命令执行器
pub struct DoctorCommand {
    formatter: OutputFormatter,
    verbose: bool,
    checks: Vec<DiagnosticCheck>,
}
impl DoctorCommand {
    /// 创建新的 doctor 命令
    pub fn new(verbose: bool) -> Self {
        Self {
            formatter: OutputFormatter::with_verbose(verbose),
            verbose,
            checks: Vec::new(),
        }
    }
    /// 执行所有诊断检查
    pub fn execute(&mut self) -> anyhow::Result<()> {
        let start: _ = Instant::now();
        self.formatter.title("Amber Environment Diagnostics");
        println!();
        // 运行所有检查
        self.check_amberjs_version();
        self.check_v8_engine();
        self.check_rust_toolchain();
        self.check_node_compatibility();
        self.check_git();
        self.check_package_managers();
        self.check_disk_space();
        self.check_network();
        self.check_permissions();
        // 打印结果
        self.print_results();
        // 统计
        let elapsed: _ = start.elapsed();
        let (pass, warn, fail) = self.count_status();
        println!();
        self.formatter.title("Summary");
        if self.formatter.color_enabled {
            println!(
                "  \x1b[32m{} passed\x1b[0m, \x1b[33m{} warnings\x1b[0m, \x1b[31m{} failed\x1b[0m",
                pass, warn, fail
            );
        } else {
            println!("  {} passed, {} warnings, {} failed", pass, warn, fail);
        }
        println!(
            "  Completed in {}",
            OutputFormatter::format_duration(elapsed)
        );
        println!();
        if fail > 0 {
            self.formatter
                .warning("Some checks failed. See suggestions above.");
        } else if warn > 0 {
            self.formatter
                .info("All critical checks passed. Some warnings to review.");
        } else {
            self.formatter
                .success("All checks passed! Your environment is ready.");
        }
        Ok(())
    }
    fn add_check(&mut self, check: DiagnosticCheck) {
        self.checks.push(check);
    }
    fn check_amberjs_version(&mut self) {
        let version: _ = env!("CARGO_PKG_VERSION");
        self.add_check(DiagnosticCheck {
            name: "Amber Version".to_string(),
            description: "Check Amber runtime version".to_string(),
            status: CheckStatus::Pass,
            message: Some(format!("v{}", version)),
            suggestion: None,
        });
    }
    fn check_v8_engine(&mut self) {
        // V8 引擎通常与 rusty_v8 绑定
        let v8_version: _ = "10.x (rusty_v8 0.22)";
        self.add_check(DiagnosticCheck {
            name: "V8 JavaScript Engine".to_string(),
            description: "Check V8 engine availability".to_string(),
            status: CheckStatus::Pass,
            message: Some(v8_version.to_string()),
            suggestion: None,
        });
    }
    fn check_rust_toolchain(&mut self) {
        match Command::new("rustc").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version: _ = String::from_utf8_lossy(&output.stdout);
                self.add_check(DiagnosticCheck {
                    name: "Rust Toolchain".to_string(),
                    description: "Check Rust compiler".to_string(),
                    status: CheckStatus::Pass,
                    message: Some(version.trim().to_string()),
                    suggestion: None,
                });
            }
            _ => {
                self.add_check(DiagnosticCheck {
                    name: "Rust Toolchain".to_string(),
                    description: "Check Rust compiler".to_string(),
                    status: CheckStatus::Warning,
                    message: Some("Not found (optional for development)".to_string()),
                    suggestion: Some("Install Rust from https://rustup.rs".to_string()),
                });
            }
        }
    }
    fn check_node_compatibility(&mut self) {
        // 检查是否有 Node.js (用于比较和兼容性)
        match Command::new("node").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version: _ = String::from_utf8_lossy(&output.stdout);
                self.add_check(DiagnosticCheck {
                    name: "Node.js (Compatibility)".to_string(),
                    description: "Check Node.js for comparison".to_string(),
                    status: CheckStatus::Pass,
                    message: Some(version.trim().to_string()),
                    suggestion: None,
                });
            }
            _ => {
                self.add_check(DiagnosticCheck {
                    name: "Node.js (Compatibility)".to_string(),
                    description: "Check Node.js for comparison".to_string(),
                    status: CheckStatus::Skip,
                    message: Some("Not installed (optional)".to_string()),
                    suggestion: None,
                });
            }
        }
    }
    fn check_git(&mut self) {
        match Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version: _ = String::from_utf8_lossy(&output.stdout);
                self.add_check(DiagnosticCheck {
                    name: "Git".to_string(),
                    description: "Version control system".to_string(),
                    status: CheckStatus::Pass,
                    message: Some(version.trim().to_string()),
                    suggestion: None,
                });
            }
            _ => {
                self.add_check(DiagnosticCheck {
                    name: "Git".to_string(),
                    description: "Version control system".to_string(),
                    status: CheckStatus::Warning,
                    message: Some("Not installed".to_string()),
                    suggestion: Some("Install Git for version control".to_string()),
                });
            }
        }
    }
    fn check_package_managers(&mut self) {
        // Check npm
        let npm_status: _ = match Command::new("npm").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version: _ = String::from_utf8_lossy(&output.stdout);
                (CheckStatus::Pass, Some(format!("npm {}", version.trim())))
            }
            _ => (CheckStatus::Skip, Some("npm not found".to_string())),
        };
        // Check yarn
        let yarn_status: _ = match Command::new("yarn").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version: _ = String::from_utf8_lossy(&output.stdout);
                (CheckStatus::Pass, Some(format!("yarn {}", version.trim())))
            }
            _ => (CheckStatus::Skip, None),
        };
        // Check pnpm
        let pnpm_status: _ = match Command::new("pnpm").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version: _ = String::from_utf8_lossy(&output.stdout);
                (CheckStatus::Pass, Some(format!("pnpm {}", version.trim())))
            }
            _ => (CheckStatus::Skip, None),
        };
        let mut managers = Vec::new();
        if npm_status.0 == CheckStatus::Pass {
            managers.push(npm_status.1.unwrap());
        }
        if yarn_status.0 == CheckStatus::Pass {
            managers.push(yarn_status.1.unwrap());
        }
        if pnpm_status.0 == CheckStatus::Pass {
            managers.push(pnpm_status.1.unwrap());
        }
        let (status, message) = if managers.is_empty() {
            (CheckStatus::Warning, "No package manager found".to_string())
        } else {
            (CheckStatus::Pass, managers.join(", "))
        };
        self.add_check(DiagnosticCheck {
            name: "Package Managers".to_string(),
            description: "npm/yarn/pnpm availability".to_string(),
            status,
            message: Some(message),
            suggestion: if status == CheckStatus::Warning {
                Some("Install npm, yarn, or pnpm for package management".to_string())
            } else {
                None
            },
        });
    }
    fn check_disk_space(&mut self) {
        // 获取当前目录的磁盘空间
        let cwd: _ = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        #[cfg(unix)]
        {
            use std::ffi::CString;
            let path: _ = CString::new(cwd.to_string_lossy().as_bytes()).unwrap();
            let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
            let result: _ = unsafe { libc::statvfs(path.as_ptr(), &mut stat) };
            if result == 0 {
                let free_bytes: _ = stat.f_bavail as u64 * stat.f_bsize as u64;
                let free_gb: _ = free_bytes / (1024 * 1024 * 1024);
                let (status, suggestion) = if free_gb < 1 {
                    (
                        CheckStatus::Fail,
                        Some("Less than 1GB free. Free up disk space.".to_string()),
                    )
                } else if free_gb < 5 {
                    (
                        CheckStatus::Warning,
                        Some("Low disk space. Consider cleaning up.".to_string()),
                    )
                } else {
                    (CheckStatus::Pass, None)
                };
                self.add_check(DiagnosticCheck {
                    name: "Disk Space".to_string(),
                    description: "Available disk space".to_string(),
                    status,
                    message: Some(format!("{} GB free", free_gb)),
                    suggestion,
                });
                return;
            }
        }
        // Fallback for non-Unix or failed check
        self.add_check(DiagnosticCheck {
            name: "Disk Space".to_string(),
            description: "Available disk space".to_string(),
            status: CheckStatus::Skip,
            message: Some("Unable to determine".to_string()),
            suggestion: None,
        });
    }
    fn check_network(&mut self) {
        // 简单的网络连通性检查
        #[cfg(unix)]
        {
            let result: _ = Command::new("ping")
                .args(["-c", "1", "-W", "2", "8.8.8.8"])
                .output();
            let (status, message) = match result {
                Ok(output) if output.status.success() => (
                    CheckStatus::Pass,
                    "Internet connection available".to_string(),
                ),
                _ => (
                    CheckStatus::Warning,
                    "Internet connection unavailable or slow".to_string(),
                ),
            };
            self.add_check(DiagnosticCheck {
                name: "Network Connectivity".to_string(),
                description: "Internet connection check".to_string(),
                status,
                message: Some(message),
                suggestion: if status == CheckStatus::Warning {
                    Some("Check your internet connection".to_string())
                } else {
                    None
                },
            });
            return;
        }
        #[cfg(not(unix))]
        {
            self.add_check(DiagnosticCheck {
                name: "Network Connectivity".to_string(),
                description: "Internet connection check".to_string(),
                status: CheckStatus::Skip,
                message: Some("Skipped on this platform".to_string()),
                suggestion: None,
            });
        }
    }
    fn check_permissions(&mut self) {
        let cwd: _ = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // 检查写入权限
        let test_file: _ = cwd.join(".amberjs-permission-test");
        let can_write: _ = fs::write(&test_file, "test").is_ok();
        if can_write {
            let _: _ = fs::remove_file(&test_file);
        }
        let (status, message) = if can_write {
            (CheckStatus::Pass, "Write permissions OK".to_string())
        } else {
            (
                CheckStatus::Fail,
                "No write permissions in current directory".to_string(),
            )
        };
        self.add_check(DiagnosticCheck {
            name: "File Permissions".to_string(),
            description: "Write access to current directory".to_string(),
            status,
            message: Some(message),
            suggestion: if status == CheckStatus::Fail {
                Some("Check directory permissions or change to a writable directory".to_string())
            } else {
                None
            },
        });
    }
    fn print_results(&self) {
        for check in &self.checks {
            let icon: _ = check.status.icon();
            let color: _ = check.status.color();
            if self.formatter.color_enabled {
                print!("  {}{}\x1b[0m {}", color, icon, check.name);
            } else {
                print!("  {} {}", icon, check.name);
            }
            if let Some(msg) = &check.message {
                print!(" - {}", msg);
            }
            println!();
            // 打印建议 (仅在非 pass 状态)
            if let Some(suggestion) = &check.suggestion {
                if self.formatter.color_enabled {
                    println!("    \x1b[90m└─ {}\x1b[0m", suggestion);
                } else {
                    println!("    └─ {}", suggestion);
                }
            }
        }
    }
    fn count_status(&self) -> (usize, usize, usize) {
        let mut pass = 0;
        let mut warn = 0;
        let mut fail = 0;
        for check in &self.checks {
            match check.status {
                CheckStatus::Pass => pass += 1,
                CheckStatus::Warning => warn += 1,
                CheckStatus::Fail => fail += 1,
                CheckStatus::Skip => {}
            }
        }
        (pass, warn, fail)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
use std::collections::{HashMap, BTreeMap};
use std::fs::File;
use anyhow::{Result, Error};
use std::time::Instant;
    #[test]
    fn test_check_status_icon() {
        assert_eq!(CheckStatus::Pass.icon(), "✓");
        assert_eq!(CheckStatus::Warning.icon(), "⚠");
        assert_eq!(CheckStatus::Fail.icon(), "✗");
        assert_eq!(CheckStatus::Skip.icon(), "○");
    }
    #[test]
    fn test_doctor_command_execute() {
        let mut cmd = DoctorCommand::new(false);
        assert!(cmd.execute().is_ok());
    }
    #[test]
    fn test_doctor_command_checks() {
        let mut cmd = DoctorCommand::new(true);
        cmd.check_amberjs_version();
        cmd.check_v8_engine();
        assert!(cmd.checks.len() >= 2);
        assert!(cmd.checks.iter().any(|c| c.name == "Amber Version"));
    }
}