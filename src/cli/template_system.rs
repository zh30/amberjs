// Template System Module
// Stage 91 Phase 4.3 - 快速启动模板系统
//
// 实现高级模板功能：
// - 模板变量替换引擎
// - 目录结构生成器
// - 模板注册表
/// - 依赖安装集成
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use serde::{Deserialize, Serialize};
use super::output_formatter::OutputFormatter;
// ============================================================================
// 核心类型定义
// ============================================================================
/// 包管理器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Amber,
}
impl PackageManager {
    /// 从 lockfile 名称检测包管理器
    pub fn from_lockfile(filename: &str) -> Option<Self> {
        match filename {
            "package-lock.json" => Some(Self::Npm),
            "yarn.lock" => Some(Self::Yarn),
            "pnpm-lock.yaml" => Some(Self::Pnpm),
            "bun.lockb" => Some(Self::Bun),
            "amberjs.lock" => Some(Self::Amber),
            _ => None,
        }
    }
    /// 检测项目中使用的包管理器
    pub fn detect(project_path: &Path) -> Self {
        let lockfiles: _ = ["bun.lockb", "pnpm-lock.yaml", "yarn.lock", "package-lock.json"];
        for lockfile in lockfiles {
            if project_path.join(lockfile).exists() {
                if let Some(pm) = Self::from_lockfile(lockfile) {
                    return pm;
                }
            }
        }
        // 默认使用 Amber
        Self::Amber
    }
    /// 获取安装命令
    pub fn install_command(&self) -> &'static str {
        match self {
            Self::Npm => "npm install",
            Self::Yarn => "yarn install",
            Self::Pnpm => "pnpm install",
            Self::Bun => "bun install",
            Self::Amber => "amber install",
        }
    }
    /// 生成添加依赖命令
    pub fn add_command(&self, deps: &[String], dev: bool) -> String {
        let deps_str: _ = deps.join(" ");
        match self {
            Self::Npm => {
                if dev {
                    format!("npm install -D {}", deps_str)
                } else {
                    format!("npm install {}", deps_str)
                }
            }
            Self::Yarn => {
                if dev {
                    format!("yarn add -D {}", deps_str)
                } else {
                    format!("yarn add {}", deps_str)
                }
            }
            Self::Pnpm => {
                if dev {
                    format!("pnpm add -D {}", deps_str)
                } else {
                    format!("pnpm add {}", deps_str)
                }
            }
            Self::Bun => {
                if dev {
                    format!("bun add -d {}", deps_str)
                } else {
                    format!("bun add {}", deps_str)
                }
            }
            Self::Amber => {
                if dev {
                    format!("amber add -D {}", deps_str)
                } else {
                    format!("amber add {}", deps_str)
                }
            }
        }
    }
}
/// 文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// 相对路径
    pub path: String,
    /// 文件内容（支持模板变量）
    pub content: String,
    /// 是否为可执行文件
    #[serde(default)]
    pub executable: bool,
}
/// 目录结构定义
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryStructure {
    /// 要创建的目录列表
    pub directories: Vec<String>,
    /// 要创建的文件列表
    pub files: Vec<FileEntry>,
}
/// 项目模板定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 目录列表
    pub directories: Vec<String>,
    /// 文件列表（路径 -> 内容）
    pub files: Vec<(String, String)>,
    /// 依赖列表
    pub dependencies: Vec<String>,
    /// 开发依赖列表
    pub dev_dependencies: Vec<String>,
    /// npm scripts
    pub scripts: HashMap<String, String>,
    /// 模板标签
    #[serde(default)]
    pub tags: Vec<String>,
}
impl Default for ProjectTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            directories: vec!["src".to_string()],
            files: vec![],
            dependencies: vec![],
            dev_dependencies: vec![],
            scripts: HashMap::new(),
            tags: vec![],
        }
    }
}
// ============================================================================
// 模板变量替换引擎
// ============================================================================
/// 模板引擎
pub struct TemplateEngine {
    /// 变量映射
    variables: HashMap<String, String>,
}
impl TemplateEngine {
    /// 创建新的模板引擎
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
    /// 设置变量
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }
    /// 批量设置变量
    pub fn set_all(&mut self, vars: HashMap<String, String>) -> &mut Self {
        self.variables.extend(vars);
        self
    }
    /// 替换模板中的变量 {{variable}}
    pub fn render(&self, template: &str) -> String {
        let mut result = template.to_string();
        for (key, value) in &self.variables {
            let pattern: _ = format!("{{{{{}}}}}", key);
            result = result.clone().replace(&pattern, value);
        }
        result
    }
    /// 处理条件内容 {{#if var}}...{{/if}}
    pub fn process_conditionals(&self, template: &str) -> String {
        let mut result = template.to_string();
        // 处理 {{#if var}}...{{/if}}
        let if_pattern: _ = Regex::new(r"\{\{#if (\w+)\}\}([\s\S]*?)\{\{/if\}\}").unwrap();
        result = if_pattern
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name: _ = &caps[1];
                let content: _ = &caps[2];
                if self
                    .variables
                    .get(var_name)
                    .map(|v| v == "true" || !v.is_empty())
                    .unwrap_or(false)
                {
                    content.to_string()
                } else {
                    String::new()
                }
            })
            .to_string();
        // 处理 {{#unless var}}...{{/unless}}
        let unless_pattern: _ = Regex::new(r"\{\{#unless (\w+)\}\}([\s\S]*?)\{\{/unless\}\}").unwrap();
        result = unless_pattern
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name: _ = &caps[1];
                let content: _ = &caps[2];
                if self
                    .variables
                    .get(var_name)
                    .map(|v| v == "true" || !v.is_empty())
                    .unwrap_or(false)
                {
                    String::new()
                } else {
                    content.to_string()
                }
            })
            .to_string();
        result
    }
    /// 完整渲染（条件 + 变量）
    pub fn render_full(&self, template: &str) -> String {
        let processed: _ = self.process_conditionals(template);
        self.render(&processed)
    }
}
impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}
// ============================================================================
// 模板注册表
// ============================================================================
/// 模板注册表
pub struct TemplateRegistry {
    templates: HashMap<String, ProjectTemplate>,
}
impl TemplateRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }
    /// 创建带内置模板的注册表
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register_builtin_templates();
        registry
    }
    /// 注册模板
    pub fn register(&mut self, template: ProjectTemplate) {
        self.templates.insert(template.name.clone(), template);
    }
    /// 获取模板
    pub fn get(&self, name: &str) -> Option<&ProjectTemplate> {
        self.templates.get(name)
    }
    /// 列出所有模板
    pub fn list(&self) -> Vec<&ProjectTemplate> {
        self.templates.values().collect()
    }
    /// 按标签过滤模板
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&ProjectTemplate> {
        self.templates
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .collect()
    }
    /// 注册内置模板
    pub fn register_builtin_templates(&mut self) {
        // Basic JavaScript
        self.register(Self::create_basic_template());
        // TypeScript
        self.register(Self::create_typescript_template());
        // Web API
        self.register(Self::create_webapi_template());
        // CLI Tool
        self.register(Self::create_cli_template());
        // Fullstack
        self.register(Self::create_fullstack_template());
        // Monorepo
        self.register(Self::create_monorepo_template());
        // Library
        self.register(Self::create_library_template());
        // Worker
        self.register(Self::create_worker_template());
    }
    fn create_basic_template() -> ProjectTemplate {
        let mut scripts = HashMap::new();
        scripts.insert("start".to_string(), "amber run src/index.js".to_string());
        scripts.insert(
            "dev".to_string(),
            "amber run --watch src/index.js".to_string(),
        );
        scripts.insert("test".to_string(), "amber test".to_string());
        ProjectTemplate {
            name: "basic".to_string(),
            description: "Basic JavaScript project with minimal setup".to_string(),
            directories: vec!["src".to_string()],
            files: vec![
                (
                    "src/index.js".to_string(),
                    r#"// {{project_name}}
// Created with `amber init`
console.log("🚀 Welcome to {{project_name}}!");
async function main() {
    console.log("Edit src/index.js to get started.");
}
main();
"#
                    .to_string(),
                ),
                (
                    "src/index.test.js".to_string(),
                    r#"import { describe, it, expect } from 'amber:test';
describe('{{project_name}}', () => {
    it('should pass basic test', () => {
        expect(1 + 1).toBe(2);
    });
});
"#
                    .to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec![],
            scripts,
            tags: vec!["javascript".to_string(), "beginner".to_string()],
        }
    }
    fn create_typescript_template() -> ProjectTemplate {
        let mut scripts = HashMap::new();
        scripts.insert("start".to_string(), "amber run src/index.ts".to_string());
        scripts.insert(
            "dev".to_string(),
            "amber run --watch src/index.ts".to_string(),
        );
        scripts.insert(
            "build".to_string(),
            "amber bundle src/index.ts --outfile dist/index.js".to_string(),
        );
        scripts.insert("test".to_string(), "amber test".to_string());
        ProjectTemplate {
            name: "typescript".to_string(),
            description: "TypeScript project with type checking".to_string(),
            directories: vec!["src".to_string(), "dist".to_string()],
            files: vec![
                (
                    "src/index.ts".to_string(),
                    r#"// {{project_name}}
// TypeScript project created with `amber init --template typescript`
interface Config {
    name: string;
    version: string;
}
const config: Config = {
    name: "{{project_name}}",
    version: "{{version}}"
};
async function main(): Promise<void> {
    console.log(`🚀 Welcome to ${config.name} v${config.version}!`);
}
main().catch(console.error);
"#
                    .to_string(),
                ),
                (
                    "tsconfig.json".to_string(),
                    r#"{
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
    "rootDir": "./src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#
                    .to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec!["typescript".to_string()],
            scripts,
            tags: vec!["typescript".to_string()],
        }
    }
    fn create_webapi_template() -> ProjectTemplate {
        let mut scripts = HashMap::new();
        scripts.insert("start".to_string(), "amber run src/server.ts".to_string());
        scripts.insert(
            "dev".to_string(),
            "amber run --watch src/server.ts".to_string(),
        );
        scripts.insert("test".to_string(), "amber test".to_string());
        ProjectTemplate {
            name: "web-api".to_string(),
            description: "Web API server with HTTP routing".to_string(),
            directories: vec![
                "src".to_string(),
                "src/routes".to_string(),
                "src/middleware".to_string(),
            ],
            files: vec![
                (
                    "src/server.ts".to_string(),
                    r#"// {{project_name}} - Web API Server
const PORT = Number(process.env.PORT) || 3000;
interface Route {
    method: string;
    path: string;
    handler: (req: Request) => Response | Promise<Response>;
}
const routes: Route[] = [
    {
        method: 'GET',
        path: '/',
        handler: () => Response.json({ message: 'Welcome to {{project_name}}!' })
    },
    {
        method: 'GET',
        path: '/health',
        handler: () => Response.json({ status: 'ok', timestamp: Date.now() })
    }
];
console.log(`🚀 {{project_name}} server starting on port ${PORT}...`);
console.log(`📍 Health: http://localhost:${PORT}/health`);
"#
                    .to_string(),
                ),
                (
                    "src/routes/api.ts".to_string(),
                    r#"// API Routes for {{project_name}}
export const apiRoutes = {
    getUsers: () => Response.json([{ id: 1, name: 'User' }]),
    getHealth: () => Response.json({ status: 'healthy' })
};
"#
                    .to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec!["typescript".to_string()],
            scripts,
            tags: vec!["api".to_string(), "server".to_string(), "typescript".to_string()],
        }
    }
    fn create_cli_template() -> ProjectTemplate {
        let mut scripts = HashMap::new();
        scripts.insert("start".to_string(), "amber run src/cli.ts".to_string());
        scripts.insert(
            "build".to_string(),
            "amber bundle src/cli.ts --outfile dist/cli.js".to_string(),
        );
        scripts.insert("test".to_string(), "amber test".to_string());
        ProjectTemplate {
            name: "cli-tool".to_string(),
            description: "CLI tool with argument parsing".to_string(),
            directories: vec!["src".to_string(), "dist".to_string()],
            files: vec![(
                "src/cli.ts".to_string(),
                r#"#!/usr/bin/env amber
// {{project_name}} - CLI Tool
const args = process.argv.slice(2);
const command = args[0];
const commands: Record<string, () => void> = {
    help: () => console.log(`
{{project_name}} - A CLI tool
Usage:
  {{project_name}} <command>
Commands:
  help      Show this help
  version   Show version
`),
    version: () => console.log('{{version}}'),
};
const handler = commands[command] || commands.help;
handler();
"#
                .to_string(),
            )],
            dependencies: vec![],
            dev_dependencies: vec!["typescript".to_string()],
            scripts,
            tags: vec!["cli".to_string(), "tool".to_string()],
        }
    }
    fn create_fullstack_template() -> ProjectTemplate {
        let mut scripts = HashMap::new();
        scripts.insert(
            "dev".to_string(),
            "amber run --watch src/server/index.ts".to_string(),
        );
        scripts.insert(
            "build".to_string(),
            "amber bundle src/client/index.ts --outfile dist/client.js".to_string(),
        );
        ProjectTemplate {
            name: "fullstack".to_string(),
            description: "Full-stack web application".to_string(),
            directories: vec![
                "src/client".to_string(),
                "src/server".to_string(),
                "src/shared".to_string(),
                "public".to_string(),
            ],
            files: vec![
                (
                    "src/server/index.ts".to_string(),
                    "// Server entry\nconsole.log('Server starting...');".to_string(),
                ),
                (
                    "src/client/index.ts".to_string(),
                    "// Client entry\nconsole.log('Client loaded');".to_string(),
                ),
                (
                    "src/shared/types.ts".to_string(),
                    "// Shared types\nexport interface User { id: number; name: string; }"
                        .to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec!["typescript".to_string()],
            scripts,
            tags: vec!["fullstack".to_string(), "web".to_string()],
        }
    }
    fn create_monorepo_template() -> ProjectTemplate {
        ProjectTemplate {
            name: "monorepo".to_string(),
            description: "Monorepo with workspaces".to_string(),
            directories: vec!["packages".to_string(), "apps".to_string()],
            files: vec![
                (
                    "package.json".to_string(),
                    r#"{
  "name": "{{project_name}}",
  "private": true,
  "workspaces": ["packages/*", "apps/*"]
}
"#
                    .to_string(),
                ),
                (
                    "packages/.gitkeep".to_string(),
                    "".to_string(),
                ),
                (
                    "apps/.gitkeep".to_string(),
                    "".to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec![],
            scripts: HashMap::new(),
            tags: vec!["monorepo".to_string(), "workspace".to_string()],
        }
    }
    fn create_library_template() -> ProjectTemplate {
        let mut scripts = HashMap::new();
        scripts.insert(
            "build".to_string(),
            "amber bundle src/index.ts --outfile dist/index.js".to_string(),
        );
        scripts.insert("test".to_string(), "amber test".to_string());
        ProjectTemplate {
            name: "library".to_string(),
            description: "Reusable library package".to_string(),
            directories: vec!["src".to_string(), "tests".to_string(), "dist".to_string()],
            files: vec![
                (
                    "src/index.ts".to_string(),
                    r#"// {{project_name}} - Library
export function greet(name: string): string {
    return `Hello, ${name}!`;
}
export default { greet };
"#
                    .to_string(),
                ),
                (
                    "tsconfig.json".to_string(),
                    r#"{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "declaration": true,
    "outDir": "./dist"
  }
}
"#
                    .to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec!["typescript".to_string()],
            scripts,
            tags: vec!["library".to_string(), "package".to_string()],
        }
    }
    fn create_worker_template() -> ProjectTemplate {
        ProjectTemplate {
            name: "worker".to_string(),
            description: "Edge/Cloudflare Worker".to_string(),
            directories: vec!["src".to_string()],
            files: vec![
                (
                    "src/worker.ts".to_string(),
                    r#"// {{project_name}} - Edge Worker
export default {
    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);
        if (url.pathname === '/') {
            return new Response('Hello from {{project_name}}!');
        }
        return new Response('Not Found', { status: 404 });
    }
};
"#
                    .to_string(),
                ),
                (
                    "wrangler.toml".to_string(),
                    r#"name = "{{project_name}}"
main = "src/worker.ts"
compatibility_date = "2024-01-01"
"#
                    .to_string(),
                ),
            ],
            dependencies: vec![],
            dev_dependencies: vec![],
            scripts: HashMap::new(),
            tags: vec!["worker".to_string(), "edge".to_string(), "serverless".to_string()],
        }
    }
}
impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}
// ============================================================================
// 目录结构生成器
// ============================================================================
/// 目录结构生成器
pub struct DirectoryGenerator {
    formatter: OutputFormatter,
}
impl DirectoryGenerator {
    /// 创建新的生成器
    pub fn new() -> Self {
        Self {
            formatter: OutputFormatter::new(),
        }
    }
    /// 生成目录结构
    pub fn generate(
        &self,
        base_path: &Path,
        structure: &DirectoryStructure,
        engine: &TemplateEngine,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let mut created = Vec::new();
        // 创建目录
        for dir in &structure.directories {
            let full_path: _ = base_path.join(dir);
            if !full_path.exists() {
                fs::create_dir_all(&full_path)?;
                created.push(full_path);
            }
        }
        // 创建文件
        for file in &structure.files {
            let full_path: _ = base_path.join(&file.path);
            // 确保父目录存在
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            // 渲染内容
            let content: _ = engine.render_full(&file.content);
            fs::write(&full_path, content)?;
            // 设置可执行权限
            #[cfg(unix)]
            if file.executable {
                use std::os::unix::fs::PermissionsExt;
use std::collections::{BTreeMap, HashMap};
                let mut perms = fs::metadata(&full_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&full_path, perms)?;
            }
            created.push(full_path);
        }
        Ok(created)
    }
    /// 从 ProjectTemplate 生成
    pub fn generate_from_template(
        &self,
        base_path: &Path,
        template: &ProjectTemplate,
        engine: &TemplateEngine,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let structure: _ = DirectoryStructure {
            directories: template.directories.clone(),
            files: template
                .files
                .iter()
                .map(|(path, content)| FileEntry {
                    path: path.clone(),
                    content: content.clone(),
                    executable: path.ends_with(".sh") || path.contains("/bin/"),
                })
                .collect(),
        };
        self.generate(base_path, &structure, engine)
    }
}
impl Default for DirectoryGenerator {
    fn default() -> Self {
        Self::new()
    }
}
// ============================================================================
// 依赖安装器
// ============================================================================
/// 依赖安装器
pub struct DependencyInstaller {
    package_manager: PackageManager,
    formatter: OutputFormatter,
}
impl DependencyInstaller {
    /// 创建新的安装器
    pub fn new(project_path: &Path) -> Self {
        Self {
            package_manager: PackageManager::detect(project_path),
            formatter: OutputFormatter::new(),
        }
    }
    /// 使用指定的包管理器创建
    pub fn with_package_manager(pm: PackageManager) -> Self {
        Self {
            package_manager: pm,
            formatter: OutputFormatter::new(),
        }
    }
    /// 安装依赖
    pub fn install(&self, project_path: &Path) -> anyhow::Result<()> {
        let cmd: _ = self.package_manager.install_command();
        self.formatter
            .progress_start(&format!("Running {}...", cmd));
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let output: _ = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(project_path)
            .output()?;
        if output.status.success() {
            self.formatter.progress_done();
            Ok(())
        } else {
            let stderr: _ = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Install failed: {}", stderr))
        }
    }
    /// 添加依赖
    pub fn add_dependencies(
        &self,
        project_path: &Path,
        deps: &[String],
        dev: bool,
    ) -> anyhow::Result<()> {
        if deps.is_empty() {
            return Ok(());
        }
        let cmd: _ = self.package_manager.add_command(deps, dev);
        self.formatter.progress_start(&format!("Running {}...", cmd));
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let output: _ = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(project_path)
            .output()?;
        if output.status.success() {
            self.formatter.progress_done();
            Ok(())
        } else {
            let stderr: _ = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Add dependencies failed: {}", stderr))
        }
    }
}
// ============================================================================
// 模板实例化配置
// ============================================================================
/// 模板实例化配置
#[derive(Debug)]
pub struct TemplateInstantiationConfig {
    /// 模板名称
    pub template_name: String,
    /// 项目路径
    pub project_path: PathBuf,
    /// 模板变量
    pub variables: HashMap<String, String>,
    /// 是否安装依赖
    pub install_deps: bool,
    /// 包管理器
    pub package_manager: Option<PackageManager>,
}
/// 模板实例化器
pub struct TemplateInstantiator {
    registry: TemplateRegistry,
    formatter: OutputFormatter,
}
impl TemplateInstantiator {
    /// 创建新的实例化器
    pub fn new() -> Self {
        Self {
            registry: TemplateRegistry::with_builtins(),
            formatter: OutputFormatter::new(),
        }
    }
    /// 实例化模板
    pub fn instantiate(&self, config: &TemplateInstantiationConfig) -> anyhow::Result<()> {
        // 获取模板
        let template: _ = self
            .registry
            .get(&config.template_name)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", config.template_name))?;
        self.formatter.title(&format!(
            "Creating {} project: {}",
            template.name,
            config
                .variables
                .get("project_name")
                .unwrap_or(&"unnamed".to_string())));
        // 创建引擎并设置变量
        let mut engine = TemplateEngine::new();
        engine.set_all(config.variables.clone());
        // 默认变量
        if !config.variables.contains_key("version") {
            engine.set("version", "0.1.0");
        }
        // 创建目录结构
        self.formatter.progress_start("Creating directory structure...");
        let generator: _ = DirectoryGenerator::new();
        let created: _ = generator.generate_from_template(&config.project_path, template, &engine)?;
        self.formatter.progress_done();
        // 生成 package.json
        self.formatter.progress_start("Generating package.json...");
        self.generate_package_json(&config.project_path, template, &engine)?;
        self.formatter.progress_done();
        // 生成 .gitignore
        self.formatter.progress_start("Creating .gitignore...");
        self.generate_gitignore(&config.project_path)?;
        self.formatter.progress_done();
        // 安装依赖
        if config.install_deps {
            let pm: _ = config
                .package_manager
                .unwrap_or_else(|| PackageManager::detect(&config.project_path));
            let installer: _ = DependencyInstaller::with_package_manager(pm);
            if !template.dependencies.is_empty() || !template.dev_dependencies.is_empty() {
                self.formatter.progress_start("Installing dependencies...");
                installer.install(&config.project_path)?;
                self.formatter.progress_done();
            }
        }
        // 打印成功信息
        self.formatter.success("\nProject created successfully!");
        self.formatter.info(&format!(
            "Created {} files in {}",
            created.len(),
            config.project_path.display()));
        Ok(())
    }
    fn generate_package_json(
        &self,
        path: &Path,
        template: &ProjectTemplate,
        engine: &TemplateEngine,
    ) -> anyhow::Result<()> {
        let project_name: _ = engine.render("{{project_name}}");
        let version: _ = engine.render("{{version}}");
        let mut package = serde_json::json!({
            "name": project_name,
            "version": version,
            "type": "module",
            "scripts": template.scripts,
        });
        if !template.dependencies.is_empty() {
            let deps: HashMap<String, String> = template
                .dependencies
                .iter()
                .map(|d| (d.clone(), "*".to_string()))
                .collect();
            package["dependencies"] = serde_json::to_value(deps)?;
        }
        if !template.dev_dependencies.is_empty() {
            let deps: HashMap<String, String> = template
                .dev_dependencies
                .iter()
                .map(|d| (d.clone(), "*".to_string()))
                .collect();
            package["devDependencies"] = serde_json::to_value(deps)?;
        }
        let content: _ = serde_json::to_string_pretty(&package)?;
        fs::write(path.join("package.json"), content)?;
        Ok(())
    }
    fn generate_gitignore(&self, path: &Path) -> anyhow::Result<()> {
        let content: _ = r#"# Dependencies
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
# Logs
*.log
# Cache
.cache/
.amberjs-cache/
"#;
        fs::write(path.join(".gitignore"), content)?;
        Ok(())
    }
    /// 列出可用模板
    pub fn list_templates(&self) -> Vec<(&str, &str)> {
        self.registry
            .list()
            .iter()
            .map(|t| (t.name.as_str(), t.description.as_str()))
            .collect()
    }
}
impl Default for TemplateInstantiator {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
use anyhow::{Result, Error};
    #[test]
    fn test_template_engine_basic() {
        let mut engine = TemplateEngine::new();
        engine.set("name", "World");
        assert_eq!(engine.render("Hello, {{name}}!"), "Hello, World!");
    }
    #[test]
    fn test_template_engine_conditionals() {
        let mut engine = TemplateEngine::new();
        engine.set("typescript", "true");
        let template: _ = "{{#if typescript}}TS{{/if}}";
        assert_eq!(engine.process_conditionals(template), "TS");
    }
    #[test]
    fn test_package_manager_detect() {
        let temp_dir: _ = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(
            PackageManager::detect(temp_dir.path()),
            PackageManager::Yarn
        );
    }
    #[test]
    fn test_template_registry() {
        let registry: _ = TemplateRegistry::with_builtins();
        assert!(registry.get("basic").is_some());
        assert!(registry.get("typescript").is_some());
        assert!(registry.get("fullstack").is_some());
    }
    #[test]
    fn test_directory_generator() {
        let temp_dir: _ = TempDir::new().unwrap();
        let structure: _ = DirectoryStructure {
            directories: vec!["src".to_string(), "tests".to_string()],
            files: vec![FileEntry {
                path: "src/index.ts".to_string(),
                content: "console.log('hello');".to_string(),
                executable: false,
            }],
        };
        let generator: _ = DirectoryGenerator::new();
        let engine: _ = TemplateEngine::new();
        let result: _ = generator.generate(temp_dir.path(), &structure, &engine);
        assert!(result.is_ok());
        assert!(temp_dir.path().join("src").exists());
        assert!(temp_dir.path().join("src/index.ts").exists());
    }
}