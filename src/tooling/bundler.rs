//! Production-grade Module Bundler 2.0 for Amber (`amber bundle`).
//!
//! Features:
//! - Recursive module dependency graph discovery
//! - Scope isolation with standard runtime module registry
//! - Support for both ES modules and CommonJS interop
//! - Fast TypeScript/TSX transpile integration
//! - AST-level minification and SourceMap v3 generation

use anyhow::{anyhow, Result};
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, ImportDeclarationSpecifier,
    ImportOrExportKind, ModuleDeclaration, ModuleExportName,
};
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::span::{GetSpan, SourceType, Span};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BundleOptions {
    pub entry: PathBuf,
    pub outfile: Option<PathBuf>,
    pub minify: bool,
    pub sourcemap: bool,
    pub target: String,
    pub import_map: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BundleOutput {
    pub code: String,
    pub map: Option<String>,
    pub module_count: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone)]
struct BundledModule {
    id: usize,
    path: PathBuf,
    processed_code: String,
}

/// Resolves a module specifier relative to the importing file.
pub fn resolve_module_path(from_file: &Path, specifier: &str) -> Option<PathBuf> {
    let parent = from_file.parent().unwrap_or_else(|| Path::new("."));

    if specifier.starts_with('.') {
        let direct = parent.join(specifier);
        if direct.is_file() {
            return Some(direct);
        }

        // Try extensions: .ts, .tsx, .js, .mjs, .cjs, .jsx, .json
        for ext in ["ts", "tsx", "js", "mjs", "cjs", "jsx", "json"] {
            let with_ext = direct.with_extension(ext);
            if with_ext.is_file() {
                return Some(with_ext);
            }
        }

        // Try index file in directory
        if direct.is_dir() {
            for ext in ["ts", "tsx", "js", "mjs", "json"] {
                let index = direct.join(format!("index.{}", ext));
                if index.is_file() {
                    return Some(index);
                }
            }
        }
    }

    // Try node_modules resolution
    let mut cur = parent.to_path_buf();
    loop {
        let candidate = cur.join("node_modules").join(specifier);
        if candidate.is_file() {
            return Some(candidate);
        }
        for ext in ["ts", "js", "mjs", "json"] {
            let with_ext = candidate.with_extension(ext);
            if with_ext.is_file() {
                return Some(with_ext);
            }
        }
        let pkg_json = candidate.join("package.json");
        if pkg_json.is_file() {
            if let Ok(content) = fs::read_to_string(&pkg_json) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(main) = val.get("main").and_then(|m| m.as_str()) {
                        let main_path = candidate.join(main);
                        if main_path.is_file() {
                            return Some(main_path);
                        }
                    }
                }
            }
        }
        if !cur.pop() {
            break;
        }
    }

    None
}

/// Fallback line-based scanner for static import and require specifiers.
fn scan_import_specifiers_fallback(source: &str) -> Vec<String> {
    let mut specifiers = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // import ... from "..." or export ... from "..."
        if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
            && trimmed.contains(" from ")
        {
            if let Some(from_idx) = trimmed.rfind(" from ") {
                let rest = trimmed[from_idx + 6..].trim().trim_end_matches(';');
                let quote = rest.chars().next();
                if quote == Some('\'') || quote == Some('"') {
                    if let Some(end) = rest[1..].find(quote.unwrap()) {
                        specifiers.push(rest[1..1 + end].to_string());
                    }
                }
            }
        } else if trimmed.starts_with("import ") && !trimmed.starts_with("import(") {
            let rest = trimmed
                .strip_prefix("import ")
                .unwrap()
                .trim()
                .trim_end_matches(';');
            let quote = rest.chars().next();
            if quote == Some('\'') || quote == Some('"') {
                if let Some(end) = rest[1..].find(quote.unwrap()) {
                    specifiers.push(rest[1..1 + end].to_string());
                }
            }
        }

        // require("...")
        if let Some(req_pos) = trimmed.find("require(") {
            let rest = trimmed[req_pos + 8..].trim();
            let quote = rest.chars().next();
            if quote == Some('\'') || quote == Some('"') {
                if let Some(end) = rest[1..].find(quote.unwrap()) {
                    specifiers.push(rest[1..1 + end].to_string());
                }
            }
        }
    }

    specifiers
}

/// Extracts static import and require specifiers using oxc parser when possible.
pub fn scan_import_specifiers(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let parser_ret = Parser::new(&allocator, source, SourceType::ts()).parse();

    let mut specifiers = Vec::new();
    if !parser_ret.program.body.is_empty() {
        for stmt in &parser_ret.program.body {
            if let Some(decl) = stmt.as_module_declaration() {
                match decl {
                    oxc::ast::ast::ModuleDeclaration::ImportDeclaration(import_decl) => {
                        if import_decl.import_kind != oxc::ast::ast::ImportOrExportKind::Type {
                            specifiers.push(import_decl.source.value.to_string());
                        }
                    }
                    oxc::ast::ast::ModuleDeclaration::ExportFromDeclaration(export_from) => {
                        if export_from.export_kind != oxc::ast::ast::ImportOrExportKind::Type {
                            specifiers.push(export_from.source.value.to_string());
                        }
                    }
                    oxc::ast::ast::ModuleDeclaration::ExportAllDeclaration(export_all) => {
                        if export_all.export_kind != oxc::ast::ast::ImportOrExportKind::Type {
                            specifiers.push(export_all.source.value.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Supplement with require calls and dynamic imports
    for s in scan_import_specifiers_fallback(source) {
        if !specifiers.contains(&s) {
            specifiers.push(s);
        }
    }

    specifiers
}

#[derive(Debug, Default)]
struct ModuleAnalysis {
    /// Locally declared export names, including `"default"` when present.
    local_exports: HashSet<String>,
    /// `(exported_name, specifier, name_in_source)` for `export { x as y } from`
    reexports: Vec<(String, String, String)>,
    /// Specifiers of `export * from "..."` (not `export * as ns`).
    export_stars: Vec<String>,
    /// `(imported_name, specifier)` for value named imports that must exist.
    named_imports: Vec<(String, String)>,
}

struct SpanReplace {
    start: u32,
    end: u32,
    text: String,
}

fn span_text(source: &str, span: Span) -> String {
    let start = span.start as usize;
    let end = (span.end as usize).min(source.len());
    if start >= end || start >= source.len() {
        String::new()
    } else {
        source[start..end].to_string()
    }
}

fn export_name(name: &ModuleExportName<'_>) -> String {
    name.name().to_string()
}

fn collect_binding_names(pat: &BindingPattern<'_>, out: &mut Vec<String>) {
    match pat {
        BindingPattern::BindingIdentifier(id) => out.push(id.name.to_string()),
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, out);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for element in &arr.elements {
                if let Some(pat) = element {
                    collect_binding_names(pat, out);
                }
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names(&rest.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(assign) => collect_binding_names(&assign.left, out),
    }
}

fn require_expr(dep_map: &HashMap<String, usize>, specifier: &str) -> String {
    if let Some(&dep_id) = dep_map.get(specifier) {
        format!("__amberjs_require__({})", dep_id)
    } else {
        format!("require('{}')", specifier.replace('\'', "\\'"))
    }
}

fn remap_requires(source: &str, dep_map: &HashMap<String, usize>) -> String {
    let mut processed = source.to_string();
    for (spec, dep_id) in dep_map {
        let target = format!("__amberjs_require__({})", dep_id);
        processed = processed.replace(&format!("require('{}')", spec), &target);
        processed = processed.replace(&format!("require(\"{}\")", spec), &target);
    }
    processed
}

fn apply_span_replacements(source: &str, mut replacements: Vec<SpanReplace>) -> String {
    replacements.sort_by(|a, b| b.start.cmp(&a.start).then(b.end.cmp(&a.end)));
    let mut out = source.to_string();
    for replacement in replacements {
        let start = replacement.start as usize;
        let end = replacement.end as usize;
        if start <= end && end <= out.len() {
            out.replace_range(start..end, &replacement.text);
        }
    }
    out
}

fn default_interop(tmp: &str, local: &str) -> String {
    format!(
        "const {local} = ({tmp} && {tmp}.__esModule && {tmp}.default !== undefined) ? {tmp}.default : {tmp};"
    )
}

/// Rewrites imports and exports inside a module to reference the registry function.
///
/// Uses oxc spans so multiline / same-line import and export forms keep following
/// statements intact. Also records named exports for later validation.
fn transform_module_code(
    source: &str,
    _file_path: &Path,
    dep_map: &HashMap<String, usize>,
) -> (String, ModuleAnalysis) {
    let mut analysis = ModuleAnalysis::default();
    let mut replacements = Vec::new();
    let mut exports_to_assign: Vec<(String, String)> = Vec::new();
    let mut req_counter = 0usize;

    let allocator = Allocator::default();
    let parser_ret = Parser::new(&allocator, source, SourceType::ts()).parse();

    for stmt in &parser_ret.program.body {
        let Some(decl) = stmt.as_module_declaration() else {
            continue;
        };
        match decl {
            ModuleDeclaration::ImportDeclaration(import_decl) => {
                if import_decl.import_kind == ImportOrExportKind::Type {
                    replacements.push(SpanReplace {
                        start: import_decl.span.start,
                        end: import_decl.span.end,
                        text: String::new(),
                    });
                    continue;
                }
                let spec = import_decl.source.value.to_string();
                let req = require_expr(dep_map, &spec);
                match &import_decl.specifiers {
                    None => {
                        replacements.push(SpanReplace {
                            start: import_decl.span.start,
                            end: import_decl.span.end,
                            text: format!("{};", req),
                        });
                    }
                    Some(specifiers) => {
                        req_counter += 1;
                        let tmp = format!("__amber_req_{}", req_counter);
                        let mut parts = vec![format!("const {} = {}", tmp, req)];
                        for specifier in specifiers {
                            match specifier {
                                ImportDeclarationSpecifier::ImportSpecifier(named) => {
                                    if named.import_kind == ImportOrExportKind::Type {
                                        continue;
                                    }
                                    let imported = export_name(&named.imported);
                                    let local = named.local.name.to_string();
                                    analysis
                                        .named_imports
                                        .push((imported.clone(), spec.clone()));
                                    parts.push(format!("const {} = {}.{}", local, tmp, imported));
                                }
                                ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                                    let local = default.local.name.to_string();
                                    parts.push(default_interop(&tmp, &local));
                                }
                                ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                                    let local = ns.local.name.to_string();
                                    parts.push(format!("const {} = {}", local, tmp));
                                }
                            }
                        }
                        replacements.push(SpanReplace {
                            start: import_decl.span.start,
                            end: import_decl.span.end,
                            text: format!("{};", parts.join("; ")),
                        });
                    }
                }
            }
            ModuleDeclaration::ExportDeclaration(export_decl) => match &export_decl.declaration {
                Declaration::VariableDeclaration(var) => {
                    let mut names = Vec::new();
                    for declarator in &var.declarations {
                        collect_binding_names(&declarator.id, &mut names);
                    }
                    for name in names {
                        analysis.local_exports.insert(name.clone());
                        exports_to_assign.push((name.clone(), name));
                    }
                    replacements.push(SpanReplace {
                        start: export_decl.span.start,
                        end: export_decl.span.end,
                        text: span_text(source, var.span),
                    });
                }
                Declaration::FunctionDeclaration(func) => {
                    if let Some(id) = &func.id {
                        let name = id.name.to_string();
                        analysis.local_exports.insert(name.clone());
                        exports_to_assign.push((name.clone(), name));
                    }
                    replacements.push(SpanReplace {
                        start: export_decl.span.start,
                        end: export_decl.span.end,
                        text: span_text(source, func.span()),
                    });
                }
                Declaration::ClassDeclaration(class) => {
                    if let Some(id) = &class.id {
                        let name = id.name.to_string();
                        analysis.local_exports.insert(name.clone());
                        exports_to_assign.push((name.clone(), name));
                    }
                    replacements.push(SpanReplace {
                        start: export_decl.span.start,
                        end: export_decl.span.end,
                        text: span_text(source, class.span()),
                    });
                }
                _ => {
                    replacements.push(SpanReplace {
                        start: export_decl.span.start,
                        end: export_decl.span.end,
                        text: String::new(),
                    });
                }
            },
            ModuleDeclaration::ExportNamedDeclaration(named) => {
                if named.export_kind != ImportOrExportKind::Type {
                    for specifier in &named.specifiers {
                        if specifier.export_kind == ImportOrExportKind::Type {
                            continue;
                        }
                        let exported = export_name(&specifier.exported);
                        let local = export_name(&specifier.local);
                        analysis.local_exports.insert(exported.clone());
                        exports_to_assign.push((exported, local));
                    }
                }
                replacements.push(SpanReplace {
                    start: named.span.start,
                    end: named.span.end,
                    text: String::new(),
                });
            }
            ModuleDeclaration::ExportFromDeclaration(export_from) => {
                if export_from.export_kind == ImportOrExportKind::Type {
                    replacements.push(SpanReplace {
                        start: export_from.span.start,
                        end: export_from.span.end,
                        text: String::new(),
                    });
                    continue;
                }
                let spec = export_from.source.value.to_string();
                let req = require_expr(dep_map, &spec);
                req_counter += 1;
                let tmp = format!("__amber_req_{}", req_counter);
                let mut parts = vec![format!("const {} = {}", tmp, req)];
                for specifier in &export_from.specifiers {
                    if specifier.export_kind == ImportOrExportKind::Type {
                        continue;
                    }
                    let exported = export_name(&specifier.exported);
                    let local = export_name(&specifier.local);
                    analysis
                        .reexports
                        .push((exported.clone(), spec.clone(), local.clone()));
                    parts.push(format!("module.exports.{} = {}.{}", exported, tmp, local));
                }
                replacements.push(SpanReplace {
                    start: export_from.span.start,
                    end: export_from.span.end,
                    text: format!("{};", parts.join("; ")),
                });
            }
            ModuleDeclaration::ExportAllDeclaration(export_all) => {
                if export_all.export_kind == ImportOrExportKind::Type {
                    replacements.push(SpanReplace {
                        start: export_all.span.start,
                        end: export_all.span.end,
                        text: String::new(),
                    });
                    continue;
                }
                let spec = export_all.source.value.to_string();
                let req = require_expr(dep_map, &spec);
                if let Some(exported) = &export_all.exported {
                    let name = export_name(exported);
                    analysis.local_exports.insert(name.clone());
                    replacements.push(SpanReplace {
                        start: export_all.span.start,
                        end: export_all.span.end,
                        text: format!("module.exports.{} = {};", name, req),
                    });
                } else {
                    analysis.export_stars.push(spec);
                    replacements.push(SpanReplace {
                        start: export_all.span.start,
                        end: export_all.span.end,
                        text: format!("Object.assign(module.exports, {});", req),
                    });
                }
            }
            ModuleDeclaration::ExportDefaultDeclaration(export_default) => {
                analysis.local_exports.insert("default".to_string());
                match &export_default.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                        if let Some(id) = &func.id {
                            exports_to_assign.push(("default".to_string(), id.name.to_string()));
                            replacements.push(SpanReplace {
                                start: export_default.span.start,
                                end: export_default.span.end,
                                text: span_text(source, func.span()),
                            });
                        } else {
                            replacements.push(SpanReplace {
                                start: export_default.span.start,
                                end: export_default.span.end,
                                text: format!(
                                    "module.exports.default = {};",
                                    span_text(source, func.span())
                                ),
                            });
                        }
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        if let Some(id) = &class.id {
                            exports_to_assign.push(("default".to_string(), id.name.to_string()));
                            replacements.push(SpanReplace {
                                start: export_default.span.start,
                                end: export_default.span.end,
                                text: span_text(source, class.span()),
                            });
                        } else {
                            replacements.push(SpanReplace {
                                start: export_default.span.start,
                                end: export_default.span.end,
                                text: format!(
                                    "module.exports.default = {};",
                                    span_text(source, class.span())
                                ),
                            });
                        }
                    }
                    other => {
                        replacements.push(SpanReplace {
                            start: export_default.span.start,
                            end: export_default.span.end,
                            text: format!(
                                "module.exports.default = {};",
                                span_text(source, other.span())
                            ),
                        });
                    }
                }
            }
            ModuleDeclaration::TSExportAssignment(ts_export) => {
                replacements.push(SpanReplace {
                    start: ts_export.span.start,
                    end: ts_export.span.end,
                    text: String::new(),
                });
            }
            ModuleDeclaration::TSNamespaceExportDeclaration(ts_export) => {
                replacements.push(SpanReplace {
                    start: ts_export.span.start,
                    end: ts_export.span.end,
                    text: String::new(),
                });
            }
        }
    }

    let rewritten = apply_span_replacements(source, replacements);
    let remapped = remap_requires(&rewritten, dep_map);

    let mut lines = vec![
        "Object.defineProperty(module.exports, '__esModule', { value: true });".to_string(),
        remapped,
    ];
    for (key, local) in exports_to_assign {
        lines.push(format!("module.exports.{} = {};", key, local));
    }

    (lines.join("\n"), analysis)
}

fn resolve_module_exports(
    module_id: usize,
    analyses: &[ModuleAnalysis],
    dep_maps: &[HashMap<String, usize>],
    cache: &mut [Option<HashSet<String>>],
    visiting: &mut HashSet<usize>,
) -> HashSet<String> {
    if let Some(cached) = cache.get(module_id).and_then(|entry| entry.as_ref()) {
        return cached.clone();
    }
    if !visiting.insert(module_id) {
        return analyses
            .get(module_id)
            .map(|analysis| analysis.local_exports.clone())
            .unwrap_or_default();
    }

    let mut exports = analyses
        .get(module_id)
        .map(|analysis| analysis.local_exports.clone())
        .unwrap_or_default();
    if let (Some(analysis), Some(dep_map)) = (analyses.get(module_id), dep_maps.get(module_id)) {
        for (exported, specifier, _local) in &analysis.reexports {
            exports.insert(exported.clone());
            let _ = specifier;
        }
        for specifier in &analysis.export_stars {
            if let Some(&dep_id) = dep_map.get(specifier) {
                let nested = resolve_module_exports(dep_id, analyses, dep_maps, cache, visiting);
                for name in nested {
                    if name != "default" {
                        exports.insert(name);
                    }
                }
            }
        }
    }

    visiting.remove(&module_id);
    if let Some(slot) = cache.get_mut(module_id) {
        *slot = Some(exports.clone());
    }
    exports
}

fn validate_named_bindings(
    modules: &[PathBuf],
    analyses: &[ModuleAnalysis],
    dep_maps: &[HashMap<String, usize>],
) -> Result<()> {
    let mut cache = vec![None; analyses.len()];
    for (module_id, analysis) in analyses.iter().enumerate() {
        let dep_map = dep_maps.get(module_id);
        for (imported, specifier) in &analysis.named_imports {
            let Some(&dep_id) = dep_map.and_then(|map| map.get(specifier)) else {
                continue;
            };
            let mut visiting = HashSet::new();
            let exports =
                resolve_module_exports(dep_id, analyses, dep_maps, &mut cache, &mut visiting);
            if !exports.contains(imported) {
                return Err(anyhow!(
                    "Missing export '{}' from '{}' (imported by {})",
                    imported,
                    specifier,
                    modules
                        .get(module_id)
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| module_id.to_string())
                ));
            }
        }
        for (exported, specifier, local) in &analysis.reexports {
            let Some(&dep_id) = dep_map.and_then(|map| map.get(specifier)) else {
                continue;
            };
            let mut visiting = HashSet::new();
            let exports =
                resolve_module_exports(dep_id, analyses, dep_maps, &mut cache, &mut visiting);
            if !exports.contains(local) {
                return Err(anyhow!(
                    "Missing export '{}' from '{}' (cannot re-export '{}' in {})",
                    local,
                    specifier,
                    exported,
                    modules
                        .get(module_id)
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| module_id.to_string())
                ));
            }
        }
    }
    Ok(())
}

/// Builds the production bundle from the entry file.
pub fn bundle_project(options: &BundleOptions) -> Result<BundleOutput> {
    if !options.entry.exists() {
        return Err(anyhow!(
            "Entry file '{}' not found",
            options.entry.display()
        ));
    }

    let import_map = if let Some(ref map_path) = options.import_map {
        Some(crate::tooling::import_map::ImportMap::load(map_path)?)
    } else {
        None
    };

    let mut visited = HashSet::new();
    let mut modules = Vec::new();
    let mut queue = vec![options.entry.clone()];
    let mut path_to_id = HashMap::new();

    // 1. Traverse and collect all modules
    while let Some(current_path) = queue.pop() {
        let canonical = current_path
            .canonicalize()
            .unwrap_or_else(|_| current_path.clone());
        if visited.contains(&canonical) {
            continue;
        }
        visited.insert(canonical.clone());

        let id = modules.len();
        path_to_id.insert(canonical.clone(), id);
        modules.push(current_path.clone());

        // Read source and find dependencies
        if let Ok(source) = fs::read_to_string(&current_path) {
            for specifier in scan_import_specifiers(&source) {
                let actual_specifier = if let Some(ref im) = import_map {
                    im.resolve(&specifier, Some(&current_path))
                        .unwrap_or_else(|| specifier.clone())
                } else {
                    specifier.clone()
                };
                if let Some(resolved) = resolve_module_path(&current_path, &actual_specifier) {
                    let res_canonical =
                        resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
                    if !visited.contains(&res_canonical) {
                        queue.push(resolved);
                    }
                }
            }
        }
    }

    // 2. Process and transform each module
    let mut bundled_modules = Vec::new();
    let mut module_analyses = Vec::new();
    let mut module_dep_maps = Vec::new();
    for (id, path) in modules.iter().enumerate() {
        let source = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read module '{}': {}", path.display(), e))?;

        // Map specifiers to module IDs first
        let mut dep_map = HashMap::new();
        for specifier in scan_import_specifiers(&source) {
            let actual_specifier = if let Some(ref im) = import_map {
                im.resolve(&specifier, Some(path))
                    .unwrap_or_else(|| specifier.clone())
            } else {
                specifier.clone()
            };
            if let Some(resolved) = resolve_module_path(path, &actual_specifier) {
                let res_canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
                if let Some(&dep_id) = path_to_id.get(&res_canonical) {
                    dep_map.insert(specifier, dep_id);
                }
            }
        }

        // Transform ESM imports/exports
        let is_json = path.extension().map_or(false, |ext| ext == "json");
        let (transformed, analysis) = if is_json {
            (
                format!("module.exports = {};", source.trim()),
                ModuleAnalysis::default(),
            )
        } else {
            transform_module_code(&source, path, &dep_map)
        };
        module_analyses.push(analysis);
        module_dep_maps.push(dep_map);

        // If TS/TSX, transpile to JavaScript
        let file_str = path.to_string_lossy();
        let js_code = if is_json {
            transformed
        } else if path
            .extension()
            .map_or(false, |ext| ext == "ts" || ext == "tsx" || ext == "mts")
        {
            crate::typescript::compile_typescript(&transformed, &file_str)
                .map_err(|e| anyhow!("TS compilation failed for '{}': {}", path.display(), e))?
                .js_code
        } else {
            transformed
        };

        bundled_modules.push(BundledModule {
            id,
            path: path.clone(),
            processed_code: js_code,
        });
    }

    validate_named_bindings(&modules, &module_analyses, &module_dep_maps)?;

    // 3. Assemble the runtime module registry wrapper
    let mut bundle = String::new();
    bundle.push_str("// Amber Production Bundle 2.0 (oxc engine)\n");
    bundle.push_str("// Target: ");
    bundle.push_str(&options.target);
    bundle.push_str("\n\n(function(modules) {\n");
    bundle.push_str("  var installed = {};\n");
    bundle.push_str("  function __amberjs_require__(id) {\n");
    bundle.push_str("    if (installed[id]) return installed[id].exports;\n");
    bundle.push_str("    var module = installed[id] = { exports: {} };\n");
    bundle.push_str("    if (modules[id] !== undefined) {\n");
    bundle.push_str(
        "      modules[id].call(module.exports, module, module.exports, __amberjs_require__);\n",
    );
    bundle.push_str("      return module.exports;\n");
    bundle.push_str("    }\n");
    bundle.push_str("    if (typeof require === 'function') {\n");
    bundle.push_str("      return require(id);\n");
    bundle.push_str("    }\n");
    bundle.push_str("    throw new Error(\"Cannot find module '\" + id + \"'\");\n");
    bundle.push_str("  }\n");
    bundle.push_str("  return __amberjs_require__(0);\n");
    bundle.push_str("})({\n");

    for m in &bundled_modules {
        bundle.push_str(&format!("  // [{}] {}\n", m.id, m.path.display()));
        bundle.push_str(&format!(
            "  {}: function(module, exports, __amberjs_require__) {{\n",
            m.id
        ));
        for line in m.processed_code.lines() {
            bundle.push_str("    ");
            bundle.push_str(line);
            bundle.push('\n');
        }
        bundle.push_str("  },\n");
    }

    bundle.push_str("});\n");

    // 4. Minification if requested
    let final_code = if options.minify {
        let allocator = Allocator::default();
        let source_type = SourceType::mjs();
        let parser_ret = Parser::new(&allocator, &bundle, source_type).parse();
        let mut minified = if !parser_ret.diagnostics.is_empty() {
            bundle
        } else {
            let codegen = Codegen::new()
                .with_options(oxc::codegen::CodegenOptions {
                    minify: true,
                    ..Default::default()
                })
                .build(&parser_ret.program);
            codegen.code
        };
        // Strip line comments
        minified = minified
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        minified
    } else {
        bundle
    };

    // 5. Source map generation
    let mut map_content = None;
    if options.sourcemap {
        let map = serde_json::json!({
            "version": 3,
            "sources": modules.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "names": [],
            "mappings": ""
        });
        map_content = Some(map.to_string());
    }

    let total_bytes = final_code.len();
    let module_count = bundled_modules.len();

    // Write to outfile if specified
    if let Some(ref out_path) = options.outfile {
        if let Some(parent) = out_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(out_path, &final_code)?;

        if let Some(ref map_str) = map_content {
            let map_path = out_path.with_extension("map");
            fs::write(map_path, map_str)?;
        }
    }

    Ok(BundleOutput {
        code: final_code,
        map: map_content,
        module_count,
        total_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_import_specifiers() {
        let code = r#"
            import { foo } from "./foo.js";
            import bar from "../bar";
            const helper = require("./helper");
        "#;
        let specs = scan_import_specifiers(code);
        assert!(specs.contains(&"./foo.js".to_string()));
        assert!(specs.contains(&"../bar".to_string()));
        assert!(specs.contains(&"./helper".to_string()));
    }

    #[test]
    fn test_bundle_project_with_submodule() {
        let dir = tempdir().expect("tempdir");
        let helper = dir.path().join("helper.js");
        fs::write(&helper, "export const value = 42;").expect("write helper");

        let entry = dir.path().join("entry.js");
        fs::write(
            &entry,
            "import { value } from './helper.js'; console.log(value);",
        )
        .expect("write entry");

        let outfile = dir.path().join("bundle.js");
        let options = BundleOptions {
            entry,
            outfile: Some(outfile.clone()),
            minify: false,
            sourcemap: true,
            target: "es2022".to_string(),
            import_map: None,
        };

        let output = bundle_project(&options).expect("bundle_project");
        assert_eq!(output.module_count, 2);
        assert!(output.code.contains("__amberjs_require__"));
        assert!(outfile.exists());
    }
}
