// This file is part of the Amber project
// Use of this source code is governed by a BSD-style license
// that can be found in the LICENSE file.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Source code analyzer for extracting API information
#[derive(Debug, Clone)]
pub struct SourceAnalyzer {
    source_files: Vec<PathBuf>,
    module_map: HashMap<String, ModuleInfo>,
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub functions: Vec<FunctionInfo>,
    pub structs: Vec<StructInfo>,
    pub traits: Vec<TraitInfo>,
    pub enums: Vec<EnumInfo>,
    pub constants: Vec<ConstantInfo>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub signature: String,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub description: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub name: String,
    pub methods: Vec<FunctionInfo>,
    pub description: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
    pub description: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone)]
pub struct ConstantInfo {
    pub name: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub param_type: String,
    pub description: Option<String>,
}

/// Template engine for generating documentation
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    templates: HashMap<String, String>,
}

impl SourceAnalyzer {
    /// Create a new source analyzer
    pub fn new(source_dir: &Path) -> Result<Self, String> {
        let mut source_files = Vec::new();
        Self::collect_rust_files(source_dir, &mut source_files)?;

        Ok(Self {
            source_files,
            module_map: HashMap::new(),
        })
    }

    /// Recursively collect all .rs files
    fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();

                if path.is_dir() {
                    // Skip certain directories
                    let skip_dirs = ["target", "node_modules", ".git"];
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !skip_dirs.contains(&name) {
                            Self::collect_rust_files(&path, files)?;
                        }
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    /// Analyze all source files and extract API information
    pub fn analyze_sources(&mut self) -> Result<(), String> {
        println!("🔍 Analyzing {} source files...", self.source_files.len());

        for file_path in &self.source_files {
            let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
            let module_info = self::analyze_file(&content, file_path)?;
            self.module_map.insert(module_info.name.clone(), module_info);
        }

        println!("✅ Analyzed {} modules", self.module_map.len());
        Ok(())
    }

    /// Analyze a single source file
    fn analyze_file(content: &str, path: &Path) -> Result<ModuleInfo, String> {
        let module_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut traits = Vec::new();
        let mut enums = Vec::new();
        let mut constants = Vec::new();

        // Extract module-level documentation
        let mut description = None;
        let lines: Vec<&str> = content.lines().collect();
        for line in &lines {
            let line = line.trim();
            if line.starts_with("//!") {
                let doc = line.trim_start_matches("//!").trim();
                if description.is_none() {
                    description = Some(doc.to_string());
                } else if let Some(ref mut desc) = description {
                    desc.push_str("\n");
                    desc.push_str(doc);
                }
            }
        }

        // Extract functions
        for line in &lines {
            let line = line.trim();
            if line.starts_with("pub fn ") || line.starts_with("fn ") {
                let is_pub = line.starts_with("pub");
                if let Some(func) = extract_function_info(line, is_pub) {
                    functions.push(func);
                }
            }
        }

        // Extract structs
        for line in &lines {
            let line = line.trim();
            if line.starts_with("pub struct ") || line.starts_with("struct ") {
                let is_pub = line.starts_with("pub");
                if let Some(struc) = extract_struct_info(line, is_pub) {
                    structs.push(struc);
                }
            }
        }

        // Extract traits
        for line in &lines {
            let line = line.trim();
            if line.starts_with("pub trait ") || line.starts_with("trait ") {
                let is_pub = line.starts_with("pub");
                if let Some(trait_info) = extract_trait_info(line, is_pub) {
                    traits.push(trait_info);
                }
            }
        }

        // Extract enums
        for line in &lines {
            let line = line.trim();
            if line.starts_with("pub enum ") || line.starts_with("enum ") {
                let is_pub = line.starts_with("pub");
                if let Some(enum_info) = extract_enum_info(line, is_pub) {
                    enums.push(enum_info);
                }
            }
        }

        // Extract constants
        for line in &lines {
            let line = line.trim();
            if line.starts_with("pub const ") || line.starts_with("const ") {
                let is_pub = line.starts_with("pub");
                if let Some(const_info) = extract_constant_info(line, is_pub) {
                    constants.push(const_info);
                }
            }
        }

        Ok(ModuleInfo {
            name: module_name,
            path: path.to_path_buf(),
            functions,
            structs,
            traits,
            enums,
            constants,
            description,
        })
    }

    /// Get all analyzed modules
    pub fn get_modules(&self) -> &HashMap<String, ModuleInfo> {
        &self.module_map
    }
}

/// Extract function information from a line
fn extract_function_info(line: &str, is_pub: bool) -> Option<FunctionInfo> {
    // Simple parser for function signatures
    let name_start = if is_pub { "pub fn " } else { "fn " }.len();
    if let Some(name_end) = line.find('(') {
        let name = &line[name_start..name_end].trim();
        let signature = line.to_string();
        let visibility = if is_pub { "public" } else { "private" }.to_string();

        Some(FunctionInfo {
            name: name.to_string(),
            signature,
            description: None,
            parameters: Vec::new(), // TODO: Parse parameters
            return_type: None,      // TODO: Parse return type
            visibility,
        })
    } else {
        None
    }
}

/// Extract struct information from a line
fn extract_struct_info(line: &str, is_pub: bool) -> Option<StructInfo> {
    let name_start = if is_pub { "pub struct " } else { "struct " }.len();
    if let Some(name_end) = line.find('{').or_else(|| line.find(';')) {
        let name = &line[name_start..name_end].trim();
        let visibility = if is_pub { "public" } else { "private" }.to_string();

        Some(StructInfo {
            name: name.to_string(),
            fields: Vec::new(), // TODO: Parse fields
            description: None,
            visibility,
        })
    } else {
        None
    }
}

/// Extract trait information from a line
fn extract_trait_info(line: &str, is_pub: bool) -> Option<TraitInfo> {
    let name_start = if is_pub { "pub trait " } else { "trait " }.len();
    if let Some(name_end) = line.find('{') {
        let name = &line[name_start..name_end].trim();
        let visibility = if is_pub { "public" } else { "private" }.to_string();

        Some(TraitInfo {
            name: name.to_string(),
            methods: Vec::new(), // TODO: Parse methods
            description: None,
            visibility,
        })
    } else {
        None
    }
}

/// Extract enum information from a line
fn extract_enum_info(line: &str, is_pub: bool) -> Option<EnumInfo> {
    let name_start = if is_pub { "pub enum " } else { "enum " }.len();
    if let Some(name_end) = line.find('{') {
        let name = &line[name_start..name_end].trim();
        let visibility = if is_pub { "public" } else { "private" }.to_string();

        Some(EnumInfo {
            name: name.to_string(),
            variants: Vec::new(), // TODO: Parse variants
            description: None,
            visibility,
        })
    } else {
        None
    }
}

/// Extract constant information from a line
fn extract_constant_info(line: &str, is_pub: bool) -> Option<ConstantInfo> {
    let name_start = if is_pub { "pub const " } else { "const " }.len();
    if let Some(name_end) = line.find('=') {
        let name = &line[name_start..name_end].trim();
        let value = line[name_end + 1..].trim().to_string();
        let visibility = if is_pub { "public" } else { "private" }.to_string();

        Some(ConstantInfo {
            name: name.to_string(),
            value,
            description: None,
        })
    } else {
        None
    }
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // HTML template for module documentation
        templates.insert(
            "module".to_string(),
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{module_name}} - Amber API Documentation</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; max-width: 1200px; margin: 0 auto; padding: 20px; }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 10px; margin-bottom: 30px; }
        .module-info { background: #f8f9fa; padding: 20px; border-radius: 8px; margin-bottom: 30px; }
        .section { margin-bottom: 30px; }
        .section h2 { color: #333; border-bottom: 2px solid #667eea; padding-bottom: 10px; }
        .item { background: white; padding: 15px; margin: 10px 0; border-radius: 5px; border-left: 4px solid #667eea; }
        .signature { background: #f4f4f4; padding: 10px; border-radius: 3px; font-family: 'Monaco', 'Courier New', monospace; }
        .visibility { display: inline-block; padding: 3px 8px; border-radius: 3px; font-size: 0.85em; }
        .public { background: #d4edda; color: #155724; }
        .private { background: #f8d7da; color: #721c24; }
    </style>
</head>
<body>
    <div class="header">
        <h1>{{module_name}}</h1>
        <p>{{module_description}}</p>
    </div>

    <div class="module-info">
        <h2>Module Information</h2>
        <p><strong>Path:</strong> {{module_path}}</p>
        <p><strong>Functions:</strong> {{function_count}}</p>
        <p><strong>Structs:</strong> {{struct_count}}</p>
        <p><strong>Traits:</strong> {{trait_count}}</p>
        <p><strong>Enums:</strong> {{enum_count}}</p>
    </div>

    {{#if functions}}
    <div class="section">
        <h2>Functions</h2>
        {{#each functions}}
        <div class="item">
            <h3>{{this.name}} <span class="visibility {{this.visibility}}">{{this.visibility}}</span></h3>
            <div class="signature">{{this.signature}}</div>
        </div>
        {{/each}}
    </div>
    {{/if}}

    {{#if structs}}
    <div class="section">
        <h2>Structs</h2>
        {{#each structs}}
        <div class="item">
            <h3>{{this.name}} <span class="visibility {{this.visibility}}">{{this.visibility}}</span></h3>
            <div class="signature">struct {{this.name}}</div>
        </div>
        {{/each}}
    </div>
    {{/if}}

    {{#if traits}}
    <div class="section">
        <h2>Traits</h2>
        {{#each traits}}
        <div class="item">
            <h3>{{this.name}} <span class="visibility {{this.visibility}}">{{this.visibility}}</span></h3>
            <div class="signature">trait {{this.name}}</div>
        </div>
        {{/each}}
    </div>
    {{/if}}

    {{#if enums}}
    <div class="section">
        <h2>Enums</h2>
        {{#each enums}}
        <div class="item">
            <h3>{{this.name}} <span class="visibility {{this.visibility}}">{{this.visibility}}</span></h3>
            <div class="signature">enum {{this.name}}</div>
        </div>
        {{/each}}
    </div>
    {{/if}}

    <footer style="text-align: center; margin-top: 50px; padding: 20px; background: #f8f9fa; border-radius: 8px;">
        <p>Generated by Amber Doc Generator | <a href="index.html">API Index</a></p>
    </footer>
</body>
</html>"#.to_string(),
        );

        // Index template
        templates.insert(
            "index".to_string(),
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Amber API Documentation</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; max-width: 1200px; margin: 0 auto; padding: 20px; }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 10px; margin-bottom: 30px; text-align: center; }
        .search-box { width: 100%; padding: 15px; font-size: 16px; border: 2px solid #667eea; border-radius: 5px; margin: 20px 0; }
        .module-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 20px; }
        .module-card { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); transition: transform 0.2s; }
        .module-card:hover { transform: translateY(-5px); box-shadow: 0 4px 8px rgba(0,0,0,0.2); }
        .module-card h3 { margin-top: 0; color: #667eea; }
        .stats { display: flex; gap: 15px; margin-top: 15px; }
        .stat { background: #f8f9fa; padding: 8px 12px; border-radius: 5px; font-size: 0.9em; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Amber API Documentation</h1>
        <p>High-performance JavaScript/TypeScript Runtime</p>
    </div>

    <input type="text" class="search-box" placeholder="Search modules..." onkeyup="filterModules(this.value)">

    <div class="module-grid" id="moduleGrid">
        {{#each modules}}
        <div class="module-card" data-module-name="{{this.name}}">
            <h3><a href="{{this.name}}.html">{{this.name}}</a></h3>
            <p>{{this.description}}</p>
            <div class="stats">
                <span class="stat">{{this.functions.length}} functions</span>
                <span class="stat">{{this.structs.length}} structs</span>
                <span class="stat">{{this.traits.length}} traits</span>
            </div>
        </div>
        {{/each}}
    </div>

    <script>
        function filterModules(search) {
            const modules = document.querySelectorAll('.module-card');
            modules.forEach(module => {
                const name = module.dataset.moduleName.toLowerCase();
                if (name.includes(search.toLowerCase())) {
                    module.style.display = 'block';
                } else {
                    module.style.display = 'none';
                }
            });
        }
    </script>

    <footer style="text-align: center; margin-top: 50px; padding: 20px; background: #f8f9fa; border-radius: 8px;">
        <p>Generated by Amber Doc Generator | Version {{version}}</p>
    </footer>
</body>
</html>"#.to_string(),
        );

        Self { templates }
    }

    /// Render a template with data
    pub fn render(&self, template_name: &str, data: &serde_json::Value) -> Result<String, String> {
        let template = self
            .templates
            .get(template_name)
            .ok_or_else(|| format!("Template '{}' not found", template_name))?;

        // Simple template rendering - replace {{placeholders}}
        let mut result = template.clone();
        if let Some(obj) = data.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "".to_string(),
                    serde_json::Value::Array(arr) => {
                        // Handle arrays - create simple list
                        if key == "functions" || key == "structs" || key == "traits" || key == "enums" {
                            arr.iter()
                                .filter_map(|v| v.as_object())
                                .map(|obj| {
                                    format!(
                                        "<li>{} ({} fields)</li>",
                                        obj.get("name").unwrap_or(&serde_json::Value::String("Unknown".to_string())).as_str().unwrap_or("Unknown"),
                                        obj.get("fields").unwrap_or(&serde_json::Value::Array(Vec::new())).as_array().map_or(0, |f| f.len())
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            format!("[{}]", arr.len())
                        }
                    }
                    serde_json::Value::Object(_) => {
                        if value.is_null() {
                            "None".to_string()
                        } else {
                            "Object".to_string()
                        }
                    }
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        Ok(result)
    }
}

/// Documentation generator main struct
#[derive(Debug)]
pub struct DocGenerator {
    source_analyzer: SourceAnalyzer,
    template_engine: TemplateEngine,
    output_dir: PathBuf,
}

impl DocGenerator {
    /// Create a new documentation generator
    pub fn new(source_dir: &Path, output_dir: &Path) -> Result<Self, String> {
        let source_analyzer = SourceAnalyzer::new(source_dir)?;
        let template_engine = TemplateEngine::new();

        // Create output directory
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
        }

        Ok(Self {
            source_analyzer,
            template_engine,
            output_dir: output_dir.to_path_buf(),
        })
    }

    /// Generate all documentation
    pub async fn generate_api_docs(&mut self) -> Result<(), String> {
        println!("🚀 Starting API documentation generation...");

        // Analyze sources
        self.source_analyzer.analyze_sources()?;

        let modules = self.source_analyzer.get_modules();

        // Generate module documentation
        for (_name, module_info) in modules {
            self::generate_module_doc(&self.template_engine, module_info, &self.output_dir)?;
        }

        // Generate index
        self::generate_index(&self.template_engine, modules, &self.output_dir)?;

        println!("✅ Documentation generated in {}", self.output_dir.display());

        Ok(())
    }
}

/// Generate documentation for a single module
fn generate_module_doc(
    template_engine: &TemplateEngine,
    module_info: &ModuleInfo,
    output_dir: &Path,
) -> Result<(), String> {
    let module_data = serde_json::json!({
        "module_name": module_info.name,
        "module_description": module_info.description,
        "module_path": module_info.path.display().to_string(),
        "function_count": module_info.functions.len(),
        "struct_count": module_info.structs.len(),
        "trait_count": module_info.traits.len(),
        "enum_count": module_info.enums.len(),
        "functions": module_info.functions.iter().map(|f| {
            serde_json::json!({
                "name": f.name,
                "signature": f.signature,
                "visibility": f.visibility,
                "description": f.description
            })
        }).collect::<Vec<_>>(),
        "structs": module_info.structs.iter().map(|s| {
            serde_json::json!({
                "name": s.name,
                "visibility": s.visibility,
                "description": s.description,
                "fields": s.fields.len()
            })
        }).collect::<Vec<_>>(),
        "traits": module_info.traits.iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "visibility": t.visibility,
                "description": t.description
            })
        }).collect::<Vec<_>>(),
        "enums": module_info.enums.iter().map(|e| {
            serde_json::json!({
                "name": e.name,
                "visibility": e.visibility,
                "description": e.description
            })
        }).collect::<Vec<_>>()
    });

    let html = template_engine.render("module", &module_data)?;
    let file_path = output_dir.join(format!("{}.html", module_info.name));
    fs::write(&file_path, html).map_err(|e| e.to_string())?;

    println!("  📄 Generated: {}.html", module_info.name);

    Ok(())
}

/// Generate index page
fn generate_index(
    template_engine: &TemplateEngine,
    modules: &HashMap<String, ModuleInfo>,
    output_dir: &Path,
) -> Result<(), String> {
    let modules_data = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "modules": modules.values().map(|m| {
            serde_json::json!({
                "name": m.name,
                "description": m.description,
                "functions": m.functions,
                "structs": m.structs,
                "traits": m.traits,
                "enums": m.enums
            })
        }).collect::<Vec<_>>()
    });

    let html = template_engine.render("index", &modules_data)?;
    let file_path = output_dir.join("index.html");
    fs::write(&file_path, html).map_err(|e| e.to_string())?;

    println!("  📄 Generated: index.html");

    Ok(())
}
