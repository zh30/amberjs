//! oxc-backed TypeScript/TSX transpile for the default Beejs runtime path.
//!
//! This is transpile-only: parse + transform + emit. It does not run `tsc`
//! type-checking. Target is ES2022 so `using` / Stage-3 decorators downlevel
//! to something rusty_v8 0.22 can execute.

use std::path::Path;

use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenOptions, CodegenReturn};
use oxc::diagnostics::OxcDiagnostic;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{
    DecoratorOptions, HelperLoaderMode, JsxOptions, JsxRuntime, TransformOptions, Transformer,
};

use crate::compiler::{CompilationOutput, ErrorSeverity, TypeScriptError};

/// Cache-busting id mixed into the transpile cache key.
pub const BACKEND_ID: &str = "oxc-0.147.2";

pub fn transpile(source: &str, file_name: &str) -> Result<CompilationOutput, String> {
    let allocator = Allocator::default();
    let source_path = Path::new(file_name);
    let source_type = source_type_from_file_name(file_name);

    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if parser_ret.panicked || !parser_ret.diagnostics.is_empty() {
        return Err(format_oxc_diagnostics(
            file_name,
            source,
            parser_ret.diagnostics.iter(),
        ));
    }

    let mut program = parser_ret.program;
    let semantic_ret = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .with_check_syntax_error(true)
        .with_enum_eval(true)
        .build(&program);
    if !semantic_ret.diagnostics.is_empty() {
        return Err(format_oxc_diagnostics(
            file_name,
            source,
            semantic_ret.diagnostics.iter(),
        ));
    }

    let scoping = semantic_ret.semantic.into_scoping();
    let mut transform_options = TransformOptions::from_target("es2022")
        .map_err(|error| format!("invalid oxc transform target: {error}"))?;
    transform_options.jsx = classic_jsx_options();
    // Keep unused value imports (they may have side effects). Only `import type` is erased.
    transform_options.typescript.only_remove_type_imports = true;
    // Inline helpers panic in oxc 0.147. External emit uses `babelHelpers.*`.
    transform_options.helper_loader.mode = HelperLoaderMode::External;
    transform_options.decorator = DecoratorOptions {
        legacy: true,
        emit_decorator_metadata: false,
        strict_null_checks: true,
    };

    let transform_ret = Transformer::new(&allocator, source_path, &transform_options)
        .build_with_scoping(scoping, &mut program);
    if !transform_ret.diagnostics.is_empty() {
        return Err(format_oxc_diagnostics(
            file_name,
            source,
            transform_ret.diagnostics.iter(),
        ));
    }

    let CodegenReturn { code, map, .. } = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some(source_path.to_path_buf()),
            ..CodegenOptions::default()
        })
        .build(&program);

    let source_map = map.map(|generated| generated.to_json_string());
    let js_code = if code.contains("babelHelpers") {
        format!("{}\n{}", BABEL_HELPERS_PRELUDE.trim(), code)
    } else {
        code
    };
    Ok(CompilationOutput {
        js_code,
        source_map,
        diagnostics: Vec::new(),
    })
}

/// Minimal `babelHelpers` surface so oxc External emit can run on V8 0.22.
const BABEL_HELPERS_PRELUDE: &str = r#"
(function (global) {
  var helpers = global.babelHelpers || (global.babelHelpers = {});
  if (typeof Symbol.dispose !== "symbol") {
    Symbol.dispose = Symbol.for("Symbol.dispose");
  }
  if (typeof Symbol.asyncDispose !== "symbol") {
    Symbol.asyncDispose = Symbol.for("Symbol.asyncDispose");
  }
  if (typeof helpers.decorate !== "function") {
    helpers.decorate = function (decorators, target, key, desc) {
      var extra = arguments.length;
      var result = extra < 3 ? target : desc === null ? (desc = Object.getOwnPropertyDescriptor(target, key)) : desc;
      var decorator;
      for (var i = decorators.length - 1; i >= 0; i--) {
        decorator = decorators[i];
        if (decorator) {
          result = extra < 3
            ? decorator(result)
            : extra > 3
              ? decorator(target, key, result)
              : decorator(target, key) || result;
        }
      }
      if (extra > 3 && result) Object.defineProperty(target, key, result);
      return result;
    };
  }
  if (typeof helpers.usingCtx !== "function") {
    helpers.usingCtx = function () {
      var stack = [];
      return {
        u: function (value) {
          if (value != null) stack.push({ value: value, async: false });
          return value;
        },
        a: function (value) {
          if (value != null) stack.push({ value: value, async: true });
          return value;
        },
        e: undefined,
        d: function () {
          var error = this.e;
          while (stack.length) {
            var item = stack.pop();
            try {
              var dispose = item.async
                ? item.value[Symbol.asyncDispose]
                : item.value[Symbol.dispose];
              if (typeof dispose === "function") {
                var result = dispose.call(item.value);
                if (item.async && result && typeof result.then === "function") {
                  throw new Error("await using requires an async dispose helper");
                }
              }
            } catch (thrown) {
              if (error == null) error = thrown;
            }
          }
          if (error != null) throw error;
        }
      };
    };
  }
})(typeof globalThis !== "undefined" ? globalThis : this);
"#;

fn classic_jsx_options() -> JsxOptions {
    let mut jsx = JsxOptions::enable();
    jsx.runtime = JsxRuntime::Classic;
    jsx.development = false;
    jsx.jsx_source_plugin = false;
    jsx.jsx_self_plugin = false;
    jsx.refresh = None;
    jsx
}

fn source_type_from_file_name(file_name: &str) -> SourceType {
    let path = Path::new(file_name);
    if let Ok(source_type) = SourceType::from_path(path) {
        return source_type;
    }
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".tsx") {
        SourceType::tsx()
    } else if lower.ends_with(".jsx") {
        SourceType::jsx()
    } else if lower.ends_with(".mts") || lower.ends_with(".cts") || lower.ends_with(".ts") {
        SourceType::ts()
    } else {
        // execute_code / eval snippets that look like TypeScript have no path.
        SourceType::ts()
    }
}

fn format_oxc_diagnostics<'a, I>(file_name: &str, source: &str, diagnostics: I) -> String
where
    I: IntoIterator<Item = &'a OxcDiagnostic>,
{
    let messages: Vec<String> = diagnostics
        .into_iter()
        .map(|diagnostic| format_one_diagnostic(file_name, source, diagnostic))
        .collect();
    if messages.is_empty() {
        format!("TypeScript compilation failed in {file_name}")
    } else {
        messages.join("; ")
    }
}

fn format_one_diagnostic(file_name: &str, source: &str, diagnostic: &OxcDiagnostic) -> String {
    let message = diagnostic.message.to_string();
    if let Some((line, column)) = first_label_line_column(source, diagnostic) {
        format!("{file_name}:{line}:{column}: {message}")
    } else {
        format!("{file_name}: {message}")
    }
}

fn first_label_line_column(source: &str, diagnostic: &OxcDiagnostic) -> Option<(u32, u32)> {
    let label = diagnostic.labels.first()?;
    Some(offset_to_line_column(source, label.offset() as usize))
}

fn offset_to_line_column(source: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(source.len());
    let mut line = 1u32;
    let mut column = 1u32;
    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

/// Convert an oxc diagnostic into the historical Beejs error struct.
#[allow(dead_code)]
pub fn diagnostic_to_error(
    file_name: &str,
    source: &str,
    diagnostic: &OxcDiagnostic,
) -> TypeScriptError {
    let (line, column) = first_label_line_column(source, diagnostic).unwrap_or((1, 1));
    TypeScriptError {
        code: 1000,
        message: diagnostic.message.to_string(),
        file: Some(file_name.to_string()),
        line: Some(line),
        column: Some(column),
        severity: ErrorSeverity::Error,
    }
}
