// TypeScript 支持模块
//
// Default product path: oxc transpile (TypeScript 6.0 syntax, transpile-only).
// The historical self-hosted compiler remains for its own unit tests.
pub mod cache;
pub mod compiler;
pub mod detect;
pub mod oxc_backend;

pub use compiler::{
    CompilationOutput, ErrorSeverity, TypeScriptCompiler, TypeScriptCompilerConfig,
    TypeScriptError, TypeScriptModule, TypeScriptTarget,
};
pub use detect::{looks_like_jsx_source, looks_like_typescript_source};

/// `//# sourceMappingURL=data:...` comment so V8 can attribute stacks to `.ts`.
pub fn source_mapping_url_comment(map_json: &str) -> String {
    use base64::Engine;
    format!(
        "\n//# sourceMappingURL=data:application/json;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(map_json.as_bytes())
    )
}

/// 快速编译 TypeScript 源代码（oxc 后端，带内容哈希缓存）
pub fn compile_typescript(source: &str, file_name: &str) -> Result<CompilationOutput, String> {
    if let Some(hit) = cache::get_cached(source, file_name) {
        return Ok(hit);
    }
    let output = oxc_backend::transpile(source, file_name)?;
    cache::put_cached(source, file_name, &output);
    Ok(output)
}

/// 快速编译 TypeScript 文件
pub fn compile_typescript_file(file_path: &std::path::Path) -> Result<CompilationOutput, String> {
    let source = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let file_name = file_path.to_string_lossy().to_string();
    compile_typescript(&source, &file_name)
}
