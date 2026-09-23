// Init Command Module
// Stage 91 Phase 4.1 - 项目初始化命令
//
/// 实现 `amber init` 命令，用于快速初始化项目
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use super::output_formatter::OutputFormatter;
/// 项目模板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    /// 基础 JavaScript 项目
    Basic,
    /// TypeScript 项目
    TypeScript,
    /// Web API 服务器
    WebApi,
    /// CLI 工具
    CliTool,
}
impl ProjectTemplate {
    /// 从字符串解析模板类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "basic" | "js" | "javascript" => Some(Self::Basic),
            "ts" | "typescript" => Some(Self::TypeScript),
            "api" | "web-api" | "server" => Some(Self::WebApi),
            "cli" | "cli-tool" => Some(Self::CliTool),
            _ => None,
        }
    }
    /// 获取模板描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::Basic => "Basic JavaScript project with minimal setup",
            Self::TypeScript => "TypeScript project with type checking",
            Self::WebApi => "Web API server with HTTP routing",
            Self::CliTool => "CLI tool with argument parsing",
        }
    }
}
/// Init 命令配置
#[derive(Debug)]
pub struct InitConfig {
    /// 项目目录
    pub project_dir: String,
    /// 项目名称
    pub project_name: String,
    /// 使用的模板
    pub template: ProjectTemplate,
    /// 是否为 Git 初始化
    pub git_init: bool,
    /// 是否安装依赖
    pub install_deps: bool,
}
impl Default for InitConfig {
    fn default() -> Self {
        Self {
            project_dir: ".".to_string(),
            project_name: "my-amberjs-app".to_string(),
            template: ProjectTemplate::Basic,
            git_init: true,
            install_deps: false,
        }
    }
}
/// Init 命令执行器
pub struct InitCommand {
    config: InitConfig,
    formatter: OutputFormatter,
}
impl InitCommand {
    /// 创建新的 init 命令
    pub fn new(config: InitConfig) -> Self {
        Self {
            config,
            formatter: OutputFormatter::new(),
        }
    }
    /// 执行 init 命令
    pub fn execute(&self) -> anyhow::Result<()> {
        self.formatter.title("Amber Project Initialization");
        let project_path: _ = Path::new(&self.config.project_dir);
        // 1. 创建项目目录
        self.formatter
            .progress_start("Creating project directory...");
        self.create_project_directory(project_path)?;
        self.formatter.progress_done();
        // 2. 生成 package.json
        self.formatter.progress_start("Generating package.json...");
        self.generate_package_json(project_path)?;
        self.formatter.progress_done();
        // 3. 根据模板生成文件
        self.formatter.progress_start(&format!(
            "Creating {} template files...",
            self.template_name()));
        self.generate_template_files(project_path)?;
        self.formatter.progress_done();
        // 4. 如果是 TypeScript 项目，生成 tsconfig.json
        if self.config.template == ProjectTemplate::TypeScript
            || self.config.template == ProjectTemplate::WebApi
        {
            self.formatter.progress_start("Generating tsconfig.json...");
            self.generate_tsconfig(project_path)?;
            self.formatter.progress_done();
        }
        // 5. 生成 .gitignore
        self.formatter.progress_start("Creating .gitignore...");
        self.generate_gitignore(project_path)?;
        self.formatter.progress_done();
        // 6. Git 初始化
        if self.config.git_init {
            self.formatter
                .progress_start("Initializing git repository...");
            self.init_git(project_path)?;
            self.formatter.progress_done();
        }
        // 完成消息
        self.print_success_message(project_path);
        Ok(())
    }
    fn template_name(&self) -> &'static str {
        match self.config.template {
            ProjectTemplate::Basic => "Basic JavaScript",
            ProjectTemplate::TypeScript => "TypeScript",
            ProjectTemplate::WebApi => "Web API",
            ProjectTemplate::CliTool => "CLI Tool",
        }
    }
    fn create_project_directory(&self, path: &Path) -> anyhow::Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        // 创建 src 目录
        fs::create_dir_all(path.join("src"))?;
        Ok(())
    }
    fn generate_package_json(&self, path: &Path) -> anyhow::Result<()> {
        let package_json: _ = self.create_package_json();
        let content: _ = serde_json::to_string_pretty(&package_json)?;
        fs::write(path.join("package.json"), content)?;
        Ok(())
    }
    fn create_package_json(&self) -> serde_json::Value {
        let main_file: _ = match self.config.template {
            ProjectTemplate::TypeScript | ProjectTemplate::WebApi => "src/index.ts",
            _ => "src/index.js",
        };
        let scripts: _ = match self.config.template {
            ProjectTemplate::Basic => serde_json::json!({
                "start": "amber run src/index.js",
                "dev": "amber run --watch src/index.js",
                "test": "amber test"
            }),
            ProjectTemplate::TypeScript => serde_json::json!({
                "start": "amber run src/index.ts",
                "dev": "amber run --watch src/index.ts",
                "build": "amber bundle src/index.ts --outfile dist/index.js",
                "test": "amber test"
            }),
            ProjectTemplate::WebApi => serde_json::json!({
                "start": "amber run src/index.ts",
                "dev": "amber run --watch src/index.ts",
                "build": "amber bundle src/index.ts --outfile dist/server.js",
                "test": "amber test"
            }),
            ProjectTemplate::CliTool => serde_json::json!({
                "start": "amber run src/cli.js",
                "build": "amber bundle src/cli.js --outfile dist/cli.js --target node",
                "test": "amber test"
            }),
        };
        serde_json::json!({
            "name": self.config.project_name,
            "version": "0.1.0",
            "main": main_file,
            "type": "module",
            "scripts": scripts,
            "keywords": [],
            "author": "",
            "license": "MIT",
            "devDependencies": {},
            "dependencies": {}
        })
    }
    fn generate_template_files(&self, path: &Path) -> anyhow::Result<()> {
        match self.config.template {
            ProjectTemplate::Basic => self.generate_basic_template(path),
            ProjectTemplate::TypeScript => self.generate_typescript_template(path),
            ProjectTemplate::WebApi => self.generate_webapi_template(path),
            ProjectTemplate::CliTool => self.generate_cli_template(path),
        }
    }
    fn generate_basic_template(&self, path: &Path) -> anyhow::Result<()> {
        let index_content: _ = r#"// Amber - Basic JavaScript Project
// Created with `amber init`
console.log("🚀 Welcome to Amber!");
console.log("Edit src/index.js to get started.");
// Example: Define a simple function
function greet(name) {
    return `Hello, ${name}! 👋`;
}
// Run the function
const message = greet("Developer");
console.log(message);
// Example: Async operation
async function main() {
    console.log("\n📦 Running async operation...");
    await new Promise(resolve => setTimeout(resolve, 100));
    console.log("✅ Done!");
}
main();
"#;
        fs::write(path.join("src/index.js"), index_content)?;
        // 创建示例测试文件
        let test_content: _ = r#"// Example test file
// Run with: amber test
import { describe, it, expect } from 'amber:test';
describe('Basic Tests', () => {
    it('should pass a simple test', () => {
        expect(1 + 1).toBe(2);
    });
    it('should handle strings', () => {
        expect('hello').toContain('ell');
    });
});
"#;
        fs::write(path.join("src/index.test.js"), test_content)?;
        Ok(())
    }
    fn generate_typescript_template(&self, path: &Path) -> anyhow::Result<()> {
        let index_content: _ = r#"// Amber - TypeScript Project
// Created with `amber init --template typescript`
interface User {
    id: number;
    name: string;
    email: string;
}
function greet(user: User): string {
    return `Hello, ${user.name}! Your email is ${user.email}`;
}
async function main(): Promise<void> {
    console.log("🚀 Welcome to Amber TypeScript!");
    const user: User = {
        id: 1,
        name: "Developer",
        email: "dev@example.com"
    };
    console.log(greet(user));
    console.log("✅ TypeScript is working!");
}
main().catch(console.error);
"#;
        fs::write(path.join("src/index.ts"), index_content)?;
        // 创建 TypeScript 测试文件
        let test_content: _ = r#"// TypeScript test file
import { describe, it, expect } from 'amber:test';
interface Calculator {
    add(a: number, b: number): number;
}
const calc: Calculator = {
    add: (a, b) => a + b
};
describe('TypeScript Tests', () => {
    it('should perform type-safe addition', () => {
        expect(calc.add(2, 3)).toBe(5);
    });
    it('should handle type inference', () => {
        const result = calc.add(10, 20);
        expect(typeof result).toBe('number');
    });
});
"#;
        fs::write(path.join("src/index.test.ts"), test_content)?;
        Ok(())
    }
    fn generate_webapi_template(&self, path: &Path) -> anyhow::Result<()> {
        let index_content: _ = r#"// Amber - Web API Server
// Created with `amber init --template web-api`
interface Route {
    method: string;
    path: string;
    handler: (req: Request) => Response | Promise<Response>;
}
const routes: Route[] = [
    {
        method: 'GET',
        path: '/',
        handler: () => new Response(JSON.stringify({ message: 'Welcome to Amber API!' }), {
            headers: { 'Content-Type': 'application/json' }
        })
    },
    {
        method: 'GET',
        path: '/health',
        handler: () => new Response(JSON.stringify({ status: 'ok', timestamp: Date.now() }), {
            headers: { 'Content-Type': 'application/json' }
        })
    },
    {
        method: 'GET',
        path: '/api/users',
        handler: () => {
            const users = [
                { id: 1, name: 'Alice' },
                { id: 2, name: 'Bob' }
            ];
            return new Response(JSON.stringify(users), {
                headers: { 'Content-Type': 'application/json' }
            });
        }
    }
];
const PORT = Number(process.env.PORT) || 3000;
console.log(`🚀 Starting Amber API server on port ${PORT}...`);
console.log(`📍 Health check: http://localhost:${PORT}/health`);
console.log(`📍 API endpoint: http://localhost:${PORT}/api/users`);
// Note: This is a template. Actual server implementation depends on Amber HTTP module.
console.log('\n✅ Server template ready!');
console.log('💡 Implement HTTP server using Amber fetch/serve APIs.');
"#;
        fs::write(path.join("src/index.ts"), index_content)?;
        // 创建 API 测试
        let test_content: _ = r#"// API endpoint tests
import { describe, it, expect } from 'amber:test';
describe('API Tests', () => {
    it('should return valid JSON structure', () => {
        const response = { message: 'test' };
        expect(response).toHaveProperty('message');
    });
    it('should handle health check format', () => {
        const health = { status: 'ok', timestamp: Date.now() };
        expect(health.status).toBe('ok');
        expect(typeof health.timestamp).toBe('number');
    });
});
"#;
        fs::write(path.join("src/api.test.ts"), test_content)?;
        Ok(())
    }
    fn generate_cli_template(&self, path: &Path) -> anyhow::Result<()> {
        let cli_content: _ = r#"#!/usr/bin/env amber
// Amber - CLI Tool Template
// Created with `amber init --template cli-tool`
const args = process.argv.slice(2);
const command = args[0];
function printHelp() {
    console.log(`
  🛠️  My CLI Tool
  Usage:
    my-cli <command> [options]
  Commands:
    hello <name>    Say hello to someone
    version         Show version
    help            Show this help
  Examples:
    my-cli hello World
    my-cli version
    `);
}
function handleHello(name) {
    if (!name) {
        console.error('Error: Please provide a name');
        process.exit(1);
    }
    console.log(`👋 Hello, ${name}!`);
}
function handleVersion() {
    console.log('v0.1.0');
}
// Main command router
switch (command) {
    case 'hello':
        handleHello(args[1]);
        break;
    case 'version':
    case '-v':
    case '--version':
        handleVersion();
        break;
    case 'help':
    case '-h':
    case '--help':
    case undefined:
        printHelp();
        break;
    default:
        console.error(`Unknown command: ${command}`);
        printHelp();
        process.exit(1);
}
"#;
        fs::write(path.join("src/cli.js"), cli_content)?;
        // CLI 测试
        let test_content: _ = r#"// CLI command tests
import { describe, it, expect } from 'amber:test';
describe('CLI Commands', () => {
    it('should parse version flag', () => {
        const flags = ['-v', '--version', 'version'];
        flags.forEach(flag => {
            expect(['version', '-v', '--version']).toContain(flag);
        });
    });
    it('should parse help flag', () => {
        const flags = ['-h', '--help', 'help'];
        expect(flags.length).toBe(3);
    });
});
"#;
        fs::write(path.join("src/cli.test.js"), test_content)?;
        Ok(())
    }
    fn generate_tsconfig(&self, path: &Path) -> anyhow::Result<()> {
        let tsconfig: _ = serde_json::json!({
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
        let content: _ = serde_json::to_string_pretty(&tsconfig)?;
        fs::write(path.join("tsconfig.json"), content)?;
        Ok(())
    }
    fn generate_gitignore(&self, path: &Path) -> anyhow::Result<()> {
        let gitignore_content: _ = r#"# Dependencies
node_modules/
# Build output
dist/
build/
*.js.map
# Environment
.env
.env.local
.env.*.local
# IDE
.vscode/
.idea/
*.swp
*.swo
# OS
.DS_Store
Thumbs.db
# Logs
*.log
npm-debug.log*
# Coverage
coverage/
# Cache
.cache/
.amberjs-cache/
"#;
        fs::write(path.join(".gitignore"), gitignore_content)?;
        Ok(())
    }
    fn init_git(&self, path: &Path) -> anyhow::Result<()> {
        // 检查是否已经是 git 仓库
        if path.join(".git").exists() {
            return Ok(());
        }
        // 尝试初始化 git
        let output: _ = std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output();
        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(_) => {
                // git init 失败，但不是致命错误
                Ok(())
            }
            Err(_) => {
                // git 不可用，跳过
                Ok(())
            }
        }
    }
    fn print_success_message(&self, path: &Path) {
        self.formatter
            .success("\nProject initialized successfully!");
        println!();
        self.formatter.subtitle("Next steps:");
        let project_dir: _ = if self.config.project_dir == "." {
            String::new()
        } else {
            format!("cd {} && ", self.config.project_dir)
        };
        self.formatter.numbered_item(
            1,
            &format!("{}amber run src/index.{}", project_dir, self.main_ext()),
        );
        self.formatter
            .numbered_item(2, "Edit src/ files to build your project");
        self.formatter
            .numbered_item(3, "Run tests with: amber test");
        println!();
        self.formatter.info(&format!(
            "Template: {} ({})",
            self.template_name(),
            self.config.template.description()));
        self.formatter.info("Documentation: https://amberjs.dev/docs");
        println!();
    }
    fn main_ext(&self) -> &'static str {
        match self.config.template {
            ProjectTemplate::TypeScript | ProjectTemplate::WebApi => "ts",
            _ => "js",
        }
    }
}
/// 交互式 init (从终端获取输入)
pub fn interactive_init(formatter: &OutputFormatter) -> anyhow::Result<InitConfig> {
    formatter.print_banner();
    formatter.title("Create a new Amber project");
    let mut config = InitConfig::default();
    // 1. 项目名称
    print!("  Project name ({}): ", config.project_name);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input: _ = input.trim();
    if !input.is_empty() {
        config.project_name = input.to_string();
        config.project_dir = input.to_string();
    }
    // 2. 模板选择
    println!();
    println!("  Select a template:");
    println!("    1. basic      - Basic JavaScript project");
    println!("    2. typescript - TypeScript project");
    println!("    3. web-api    - Web API server");
    println!("    4. cli-tool   - CLI tool");
    println!();
    print!("  Template (1-4) [1]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input: _ = input.trim();
    config.template = match input {
        "2" | "typescript" | "ts" => ProjectTemplate::TypeScript,
        "3" | "web-api" | "api" => ProjectTemplate::WebApi,
        "4" | "cli-tool" | "cli" => ProjectTemplate::CliTool,
        _ => ProjectTemplate::Basic,
    };
    // 3. Git 初始化
    print!("  Initialize git? (Y/n): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    config.git_init = !input.trim().eq_ignore_ascii_case("n");
    println!();
    Ok(config)
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
use std::collections::{HashMap, BTreeMap};
use anyhow::{Result, Error};
    #[test]
    fn test_project_template_from_str() {
        assert_eq!(
            ProjectTemplate::from_str("basic"),
            Some(ProjectTemplate::Basic)
        );
        assert_eq!(
            ProjectTemplate::from_str("typescript"),
            Some(ProjectTemplate::TypeScript)
        );
        assert_eq!(
            ProjectTemplate::from_str("ts"),
            Some(ProjectTemplate::TypeScript)
        );
        assert_eq!(
            ProjectTemplate::from_str("web-api"),
            Some(ProjectTemplate::WebApi)
        );
        assert_eq!(
            ProjectTemplate::from_str("cli"),
            Some(ProjectTemplate::CliTool)
        );
        assert_eq!(ProjectTemplate::from_str("unknown"), None);
    }
    #[test]
    fn test_init_basic_project() {
        let temp_dir: _ = TempDir::new().unwrap();
        let config: _ = InitConfig {
            project_dir: temp_dir.path().to_string_lossy().to_string(),
            project_name: "test-project".to_string(),
            template: ProjectTemplate::Basic,
            git_init: false,
            install_deps: false,
        };
        let cmd: _ = InitCommand::new(config);
        assert!(cmd.execute().is_ok());
        // 验证文件创建
        assert!(temp_dir.path().join("package.json").exists());
        assert!(temp_dir.path().join("src/index.js").exists());
        assert!(temp_dir.path().join(".gitignore").exists());
    }
    #[test]
    fn test_init_typescript_project() {
        let temp_dir: _ = TempDir::new().unwrap();
        let config: _ = InitConfig {
            project_dir: temp_dir.path().to_string_lossy().to_string(),
            project_name: "ts-project".to_string(),
            template: ProjectTemplate::TypeScript,
            git_init: false,
            install_deps: false,
        };
        let cmd: _ = InitCommand::new(config);
        assert!(cmd.execute().is_ok());
        // 验证 TypeScript 特定文件
        assert!(temp_dir.path().join("tsconfig.json").exists());
        assert!(temp_dir.path().join("src/index.ts").exists());
    }
}