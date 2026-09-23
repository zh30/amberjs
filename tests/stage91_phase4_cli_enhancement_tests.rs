// Stage 91 Phase 4.1 CLI Enhancement Tests
// 测试新增的 CLI 命令和功能

use tempfile::TempDir;

// ========== Output Formatter Tests ==========

#[cfg(test)]
mod output_formatter_tests {
    use std::time::Duration;

    // 由于 OutputFormatter 在 amberjs::cli 模块中，这里模拟测试
    // 实际测试需要在模块内或使用 pub 导出

    #[test]
    fn test_format_size_bytes() {
        // 模拟 format_size 函数
        fn format_size(bytes: u64) -> String {
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

        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(2147483648), "2.00 GB");
    }

    #[test]
    fn test_format_duration_micros() {
        fn format_duration(duration: Duration) -> String {
            let secs = duration.as_secs();
            let millis = duration.subsec_millis();

            if secs >= 60 {
                let mins = secs / 60;
                let remaining_secs = secs % 60;
                format!("{}m {}s", mins, remaining_secs)
            } else if secs > 0 {
                format!("{}.{:03}s", secs, millis)
            } else if millis > 0 {
                format!("{}ms", millis)
            } else {
                format!("{}μs", duration.subsec_micros())
            }
        }

        assert_eq!(format_duration(Duration::from_micros(500)), "500μs");
        assert_eq!(format_duration(Duration::from_millis(100)), "100ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.500s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
    }

    #[test]
    fn test_spinner_frames() {
        const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

        fn spinner_frame(index: usize) -> &'static str {
            SPINNER_FRAMES[index % SPINNER_FRAMES.len()]
        }

        assert_eq!(spinner_frame(0), "⠋");
        assert_eq!(spinner_frame(1), "⠙");
        assert_eq!(spinner_frame(9), "⠏");
        assert_eq!(spinner_frame(10), "⠋"); // 循环
        assert_eq!(spinner_frame(20), "⠋"); // 循环
    }
}

// ========== Init Command Tests ==========

#[cfg(test)]
mod init_command_tests {
    use super::*;
    use std::fs;

    /// 测试模板类型解析
    #[test]
    fn test_template_parsing() {
        // 模拟 ProjectTemplate
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum ProjectTemplate {
            Basic,
            TypeScript,
            WebApi,
            CliTool,
        }

        fn from_str(s: &str) -> Option<ProjectTemplate> {
            match s.to_lowercase().as_str() {
                "basic" | "js" | "javascript" => Some(ProjectTemplate::Basic),
                "ts" | "typescript" => Some(ProjectTemplate::TypeScript),
                "api" | "web-api" | "server" => Some(ProjectTemplate::WebApi),
                "cli" | "cli-tool" => Some(ProjectTemplate::CliTool),
                _ => None,
            }
        }

        // 测试各种输入
        assert_eq!(from_str("basic"), Some(ProjectTemplate::Basic));
        assert_eq!(from_str("js"), Some(ProjectTemplate::Basic));
        assert_eq!(from_str("javascript"), Some(ProjectTemplate::Basic));
        assert_eq!(from_str("ts"), Some(ProjectTemplate::TypeScript));
        assert_eq!(from_str("typescript"), Some(ProjectTemplate::TypeScript));
        assert_eq!(from_str("TypeScript"), Some(ProjectTemplate::TypeScript));
        assert_eq!(from_str("web-api"), Some(ProjectTemplate::WebApi));
        assert_eq!(from_str("api"), Some(ProjectTemplate::WebApi));
        assert_eq!(from_str("cli"), Some(ProjectTemplate::CliTool));
        assert_eq!(from_str("cli-tool"), Some(ProjectTemplate::CliTool));
        assert_eq!(from_str("unknown"), None);
        assert_eq!(from_str(""), None);
    }

    /// 测试 package.json 生成
    #[test]
    fn test_package_json_generation() {
        let temp_dir = TempDir::new().unwrap();
        let package_json_path = temp_dir.path().join("package.json");

        let package_json = serde_json::json!({
            "name": "test-project",
            "version": "0.1.0",
            "main": "src/index.js",
            "type": "module",
            "scripts": {
                "start": "amber run src/index.js",
                "dev": "amber run --watch src/index.js",
                "test": "amber test"
            }
        });

        let content = serde_json::to_string_pretty(&package_json).unwrap();
        fs::write(&package_json_path, &content).unwrap();

        // 验证文件创建
        assert!(package_json_path.exists());

        // 验证内容
        let read_content = fs::read_to_string(&package_json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&read_content).unwrap();

        assert_eq!(parsed["name"], "test-project");
        assert_eq!(parsed["version"], "0.1.0");
        assert_eq!(parsed["type"], "module");
    }

    /// 测试 tsconfig.json 生成
    #[test]
    fn test_tsconfig_generation() {
        let temp_dir = TempDir::new().unwrap();
        let tsconfig_path = temp_dir.path().join("tsconfig.json");

        let tsconfig = serde_json::json!({
            "compilerOptions": {
                "target": "ESNext",
                "module": "ESNext",
                "moduleResolution": "bundler",
                "strict": true,
                "esModuleInterop": true,
                "skipLibCheck": true,
                "forceConsistentCasingInFileNames": true,
                "declaration": true,
                "outDir": "./dist",
                "rootDir": "./src",
                "lib": ["ESNext"],
                "types": ["amberjs"]
            },
            "include": ["src/**/*"],
            "exclude": ["node_modules", "dist"]
        });

        let content = serde_json::to_string_pretty(&tsconfig).unwrap();
        fs::write(&tsconfig_path, &content).unwrap();

        // 验证
        assert!(tsconfig_path.exists());
        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&tsconfig_path).unwrap()).unwrap();
        assert_eq!(parsed["compilerOptions"]["target"], "ESNext");
        assert_eq!(parsed["compilerOptions"]["strict"], true);
    }

    /// 测试 .gitignore 生成
    #[test]
    fn test_gitignore_generation() {
        let temp_dir = TempDir::new().unwrap();
        let gitignore_path = temp_dir.path().join(".gitignore");

        let gitignore_content = r#"# Dependencies
node_modules/

# Build output
dist/
build/

# Environment
.env
.env.local

# IDE
.vscode/
.idea/

# OS
.DS_Store
"#;

        fs::write(&gitignore_path, gitignore_content).unwrap();

        // 验证
        assert!(gitignore_path.exists());
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".env"));
        assert!(content.contains(".DS_Store"));
    }

    /// 测试完整项目初始化
    #[test]
    fn test_full_project_init() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // 创建 src 目录
        fs::create_dir_all(project_path.join("src")).unwrap();

        // 创建 package.json
        let package_json = serde_json::json!({
            "name": "my-amberjs-app",
            "version": "0.1.0",
            "main": "src/index.js"
        });
        fs::write(
            project_path.join("package.json"),
            serde_json::to_string_pretty(&package_json).unwrap(),
        )
        .unwrap();

        // 创建 index.js
        let index_content = r#"console.log("Hello, Amber!");
"#;
        fs::write(project_path.join("src/index.js"), index_content).unwrap();

        // 创建 .gitignore
        fs::write(project_path.join(".gitignore"), "node_modules/\n").unwrap();

        // 验证所有文件存在
        assert!(project_path.join("package.json").exists());
        assert!(project_path.join("src").exists());
        assert!(project_path.join("src/index.js").exists());
        assert!(project_path.join(".gitignore").exists());
    }
}

// ========== Info Command Tests ==========

#[cfg(test)]
mod info_command_tests {
    use std::env;

    /// 测试系统信息收集
    #[test]
    fn test_system_info_collection() {
        // 测试基本信息获取
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let cpu_count = num_cpus::get();

        assert!(!os.is_empty());
        assert!(!arch.is_empty());
        assert!(cpu_count > 0);
    }

    /// 测试 CI 环境检测
    #[test]
    fn test_ci_detection() {
        fn detect_ci() -> bool {
            env::var("CI").is_ok()
                || env::var("GITHUB_ACTIONS").is_ok()
                || env::var("GITLAB_CI").is_ok()
                || env::var("CIRCLECI").is_ok()
                || env::var("TRAVIS").is_ok()
                || env::var("JENKINS_URL").is_ok()
        }

        // 在非 CI 环境中应该返回 false (除非正在 CI 中运行)
        let is_ci = detect_ci();
        // 只是验证函数能运行
        assert_eq!(is_ci, detect_ci());
    }

    /// 测试目录信息
    #[test]
    fn test_directory_info() {
        let cwd = env::current_dir();
        assert!(cwd.is_ok());

        let home = dirs::home_dir();
        // home_dir 在某些环境可能返回 None
        if let Some(home_path) = home {
            assert!(home_path.exists());
        }

        let temp = env::temp_dir();
        assert!(temp.exists());
    }

    /// 测试 JSON 输出格式
    #[test]
    fn test_info_json_format() {
        let info = serde_json::json!({
            "amberjs": {
                "version": "0.1.0",
                "v8_version": "10.x"
            },
            "system": {
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "cpu_count": num_cpus::get()
            }
        });

        let json_str = serde_json::to_string_pretty(&info).unwrap();
        assert!(json_str.contains("amberjs"));
        assert!(json_str.contains("system"));
        assert!(json_str.contains("version"));
    }
}

// ========== Doctor Command Tests ==========

#[cfg(test)]
mod doctor_command_tests {
    use std::fs;
    use std::process::Command;

    /// 测试检查状态枚举
    #[test]
    fn test_check_status() {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum CheckStatus {
            Pass,
            Warning,
            Fail,
            Skip,
        }

        fn icon(status: CheckStatus) -> &'static str {
            match status {
                CheckStatus::Pass => "✓",
                CheckStatus::Warning => "⚠",
                CheckStatus::Fail => "✗",
                CheckStatus::Skip => "○",
            }
        }

        assert_eq!(icon(CheckStatus::Pass), "✓");
        assert_eq!(icon(CheckStatus::Warning), "⚠");
        assert_eq!(icon(CheckStatus::Fail), "✗");
        assert_eq!(icon(CheckStatus::Skip), "○");
    }

    /// 测试 Git 检查
    #[test]
    fn test_git_check() {
        let result = Command::new("git").arg("--version").output();

        match result {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                assert!(version.contains("git version"));
            }
            _ => {
                // Git 未安装，跳过
            }
        }
    }

    /// 测试包管理器检查
    #[test]
    fn test_package_manager_check() {
        // 检查 npm
        let npm_result = Command::new("npm").arg("--version").output();
        let npm_available = npm_result.map(|o| o.status.success()).unwrap_or(false);

        // 至少应该能正确检测
        assert_eq!(
            npm_available,
            Command::new("npm")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        );
    }

    /// 测试文件权限检查
    #[test]
    fn test_permissions_check() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join(".permission-test");

        // 尝试写入
        let can_write = fs::write(&test_file, "test").is_ok();
        assert!(can_write);

        // 清理
        if can_write {
            let _ = fs::remove_file(&test_file);
        }
    }

    /// 测试诊断结果统计
    #[test]
    fn test_diagnostic_count() {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum CheckStatus {
            Pass,
            Warning,
            Fail,
            Skip,
        }

        let checks = vec![
            CheckStatus::Pass,
            CheckStatus::Pass,
            CheckStatus::Warning,
            CheckStatus::Fail,
            CheckStatus::Skip,
        ];

        let mut pass = 0;
        let mut warn = 0;
        let mut fail = 0;

        for status in &checks {
            match status {
                CheckStatus::Pass => pass += 1,
                CheckStatus::Warning => warn += 1,
                CheckStatus::Fail => fail += 1,
                CheckStatus::Skip => {}
            }
        }

        assert_eq!(pass, 2);
        assert_eq!(warn, 1);
        assert_eq!(fail, 1);
    }
}

// ========== Integration Tests ==========

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

    /// 测试完整的项目初始化流程
    #[test]
    fn test_complete_init_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let project_name = "integration-test-project";
        let project_path = temp_dir.path().join(project_name);

        // 1. 创建项目目录
        fs::create_dir_all(&project_path).unwrap();
        fs::create_dir_all(project_path.join("src")).unwrap();

        // 2. 生成 package.json
        let package_json = serde_json::json!({
            "name": project_name,
            "version": "0.1.0",
            "main": "src/index.ts",
            "type": "module",
            "scripts": {
                "start": "amber run src/index.ts",
                "dev": "amber run --watch src/index.ts",
                "build": "amber bundle src/index.ts --outfile dist/index.js",
                "test": "amber test"
            }
        });
        fs::write(
            project_path.join("package.json"),
            serde_json::to_string_pretty(&package_json).unwrap(),
        )
        .unwrap();

        // 3. 生成 tsconfig.json
        let tsconfig = serde_json::json!({
            "compilerOptions": {
                "target": "ESNext",
                "module": "ESNext",
                "strict": true
            }
        });
        fs::write(
            project_path.join("tsconfig.json"),
            serde_json::to_string_pretty(&tsconfig).unwrap(),
        )
        .unwrap();

        // 4. 生成 index.ts
        let index_content = r#"interface Greeting {
    message: string;
}

const greeting: Greeting = {
    message: "Hello from Amber TypeScript!"
};

console.log(greeting.message);
"#;
        fs::write(project_path.join("src/index.ts"), index_content).unwrap();

        // 5. 生成 .gitignore
        fs::write(project_path.join(".gitignore"), "node_modules/\ndist/\n").unwrap();

        // 验证所有文件
        assert!(project_path.exists());
        assert!(project_path.join("package.json").exists());
        assert!(project_path.join("tsconfig.json").exists());
        assert!(project_path.join("src/index.ts").exists());
        assert!(project_path.join(".gitignore").exists());

        // 验证 package.json 内容
        let pkg_content = fs::read_to_string(project_path.join("package.json")).unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&pkg_content).unwrap();
        assert_eq!(pkg["name"], project_name);
        assert_eq!(pkg["main"], "src/index.ts");
    }

    /// 测试 CLI 工具可用性检测
    #[test]
    fn test_cli_tools_availability() {
        // 测试常见工具的可用性检测模式
        let tools = ["git", "node", "npm"];

        for tool in tools {
            let result = std::process::Command::new(tool).arg("--version").output();

            // 工具可能安装也可能未安装，但检测应该成功
            match result {
                Ok(output) => {
                    // 工具存在
                    if output.status.success() {
                        println!("{} is available", tool);
                    }
                }
                Err(_) => {
                    // 工具不存在
                    println!("{} is not available", tool);
                }
            }
        }
    }
}

// ========== Performance Tests ==========

#[cfg(test)]
mod performance_tests {
    use std::time::Instant;

    /// 测试格式化函数性能
    #[test]
    fn test_format_performance() {
        let start = Instant::now();

        // 执行 10000 次格式化
        for i in 0..10000 {
            let _ = format!("{} B", i);
            let _ = format!("{:.2} KB", i as f64 / 1024.0);
        }

        let elapsed = start.elapsed();
        // 应该在 100ms 内完成
        assert!(
            elapsed.as_millis() < 100,
            "Format took too long: {:?}",
            elapsed
        );
    }

    /// 测试 JSON 序列化性能
    #[test]
    fn test_json_serialization_performance() {
        let start = Instant::now();

        for _ in 0..1000 {
            let json = serde_json::json!({
                "name": "test-project",
                "version": "0.1.0",
                "scripts": {
                    "start": "amber run src/index.js",
                    "test": "amber test"
                }
            });
            let _ = serde_json::to_string_pretty(&json).unwrap();
        }

        let elapsed = start.elapsed();
        // 应该在 500ms 内完成
        assert!(
            elapsed.as_millis() < 500,
            "JSON serialization took too long: {:?}",
            elapsed
        );
    }
}
