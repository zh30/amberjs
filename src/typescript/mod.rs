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

/// Transpile many files. Each item is `(source, file_name)`.
///
/// Results stay in input order. More than one file is split across threads;
/// a single file uses the same path as [`compile_typescript`].
pub fn compile_typescript_batch(files: &[(&str, &str)]) -> Vec<Result<CompilationOutput, String>> {
    if files.len() <= 1 {
        return files
            .iter()
            .map(|(source, file_name)| compile_typescript(source, file_name))
            .collect();
    }
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .clamp(1, files.len());
    let chunk_size = files.len().div_ceil(workers);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for chunk in files.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                chunk
                    .iter()
                    .map(|(source, file_name)| compile_typescript(source, file_name))
                    .collect::<Vec<_>>()
            }));
        }
        let mut compiled = Vec::with_capacity(files.len());
        for handle in handles {
            compiled.extend(handle.join().expect("TypeScript transpile thread panicked"));
        }
        compiled
    })
}

#[cfg(test)]
mod batch_tests {
    use super::*;

    #[test]
    fn batch_preserves_order_and_matches_sequential() {
        let files = [
            ("export const a: number = 1;\n", "batch_a.ts"),
            ("export const b: string = \"two\";\n", "batch_b.ts"),
            ("export const c: boolean = true;\n", "batch_c.ts"),
        ];
        let batched = compile_typescript_batch(&files);
        assert_eq!(batched.len(), files.len());
        for (index, (source, file_name)) in files.iter().enumerate() {
            let batched_js = batched[index].as_ref().expect("batch item").js_code.clone();
            let sequential = compile_typescript(source, file_name).expect("sequential");
            assert_eq!(batched_js, sequential.js_code);
            assert!(sequential.source_map.is_some());
        }
        assert!(batched[0].as_ref().unwrap().js_code.contains('1'));
        assert!(batched[1].as_ref().unwrap().js_code.contains("two"));
        assert!(batched[2].as_ref().unwrap().js_code.contains("true"));
        cache::clear_cache();
    }
}
