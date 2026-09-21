

use anyhow::{Context, Result, anyhow};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use rusty_v8 as v8;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

mod typescript;
mod nodejs_v8;
// V8 Platform - shared across all isolates for better performance
static PLATFORM: once_cell::sync::OnceCell<v8::SharedPtr<v8::Platform>> = once_cell::sync::OnceCell::new();
static V8_INITIALIZED: once_cell::sync::OnceCell<bool> = once_cell::sync::OnceCell::new();
/// Initialize V8 engine
fn initialize_v8() -> Result<()> {
    // Initialize V8 only once
    if V8_INITIALIZED.get().copied().unwrap_or(false) {
        return Ok(());
    }
    // Create and set the platform
    let platform: _ = v8::new_default_platform(0, false)
        .map_err(|e| anyhow!("Failed to create V8 platform: {}", e))?
        .make_shared();
    // Initialize V8
    v8::V8::initialize_platform(platform.clone())
        .map_err(|e| anyhow!("Failed to initialize V8 platform: {}", e))?;
    v8::V8::initialize()
        .map_err(|e| anyhow!("Failed to initialize V8: {}", e))?;
    // Store globally
    let _: _ = PLATFORM.set(platform);
    let _: _ = V8_INITIALIZED.set(true);
    Ok(())
}
/// Cleanup V8 engine (should be called at program exit)
pub fn cleanup_v8() {
    unsafe {
        v8::V8::dispose();
    }
    v8::V8::dispose_platform();
}
/// Amber Runtime - High-performance JavaScript/TypeScript execution engine using V8
pub struct Runtime {
    stack_size: usize,
    max_heap: usize,
    execution_count: Arc<AtomicUsize>,
    verbose: bool,
    /// Reused V8 isolate for better performance
    isolate: Arc<Mutex<v8::OwnedIsolate>>,
    /// Reused V8 context for better performance
    context: v8::Global<v8::Context>,
}
impl Runtime {
    /// Create a new Amber runtime instance
    pub fn new(
        stack_size: usize,
        max_heap: usize,
        verbose: bool,
    ) -> Result<Self> {
        initialize_v8()?;
        if verbose {
            let version: _ = v8::V8::get_version();
            println!("Runtime created with:");
            println!("  Stack size: {} bytes", stack_size);
            println!("  Max heap: {} bytes", max_heap);
            println!("  V8 Engine: Initializing (version {})...", version);
        }
        // Create V8 isolate with custom parameters
        let mut create_params = v8::CreateParams::default();
        // Note: In a full implementation, you'd set heap limits here
        // create_params.set_heap_limits(max_heap, max_heap);
        let mut isolate = v8::Isolate::new(create_params)
            .map_err(|e| anyhow!("Failed to create V8 isolate: {}", e))?;
        // Create a context for script execution
        let scope: _ = &mut v8::HandleScope::new(&mut isolate);
        let context: _ = v8::Context::new(scope, Default::default());
        let context: _ = v8::Global::new(scope, context);
        Ok(Self {
            stack_size,
            max_heap,
            execution_count: Arc::new(Mutex::new(AtomicUsize::new(0)))
            verbose,
            isolate: Arc::new(Mutex::new(isolate)))
            context,
        })
    }
    /// Execute a JavaScript/TypeScript file
    pub fn execute_file(&self, path: &PathBuf) -> Result<String> {
        if self.verbose {
            println!("Executing file: {}", path.display());
        }
        let code: _ = fs::read_to_string(path)
            .context(format!("Failed to read file: {}", path.display())?;
        let base_dir: _ = path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        self.execute_code_with_context(&code, base_dir)
    }
    /// Execute JavaScript/TypeScript code with a specific base directory
    fn execute_code_with_context(&self, code: &str, _base_dir: PathBuf) -> Result<String> {
        if self.verbose {
            println!("Executing code: {} bytes", code.len());
        }
        // Check if this is TypeScript code
        let is_typescript: _ = code.contains(':')
            || code.contains("interface ")
            || code.contains("enum ")
            || code.contains("type ")
            || code.contains("namespace ");
        let code_to_execute: _ = if is_typescript {
            // Compile TypeScript to JavaScript
            if self.verbose {
                println!("Detected TypeScript code, compiling to JavaScript...");
            }
            let mut compiler = typescript::TypeScriptCompiler::new();
            match compiler.compile(code) {
                Ok(js_code) => {
                    if self.verbose {
                        println!("TypeScript compilation successful");
                    }
                    js_code
                }
                Err(e) => {
                    return Err(anyhow!("TypeScript compilation error: {}", e));
                }
            }
        } else {
            code.to_string()
        };
        // Acquire isolate lock
        let mut isolate = self.isolate.lock()
            .map_err(|e| anyhow!("Failed to acquire isolate lock: {}", e))?;
        // Create a handle scope for this execution
        let scope: _ = &mut v8::HandleScope::new(&mut **isolate);
        // Re-enter the context
        let context: _ = v8::Local::new(scope, &self.context);
        let scope: _ = &mut v8::ContextScope::new(scope, context);
        // Set up console API
        self.setup_console(scope)?;
        // Set up Node.js compatibility APIs
        nodejs_v8::setup_nodejs_apis(scope)?;
        // Compile and execute the script
        let source: _ = v8::String::new(scope, &code_to_execute)
            .ok_or_else(|| anyhow!("Failed to create V8 string"))?;
        // Use try-catch for error handling
        v8::tc_scope!(let scope, scope);
        let script: _ = match v8::Script::compile(scope, source, None) {
            Some(script) => script,
            None => {
                let exception: _ = scope.exception()
                    .unwrap_or_else(|| v8::String::new(scope, "Unknown compilation error").unwrap().into());
                let exception_str: _ = exception.to_string(scope)
                    .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                    .to_rust_string_lossy(scope);
                return Err(anyhow!("JavaScript compilation error: {}", exception_str));
            }
        };
        let result: _ = match script.run(scope) {
            Some(result) => result,
            None => {
                if scope.has_caught() {
                    let exception: _ = scope.exception()
                        .unwrap_or_else(|| v8::String::new(scope, "Unknown runtime error").unwrap().into());
                    let exception_str: _ = exception.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                        .to_rust_string_lossy(scope);
                    return Err(anyhow!("JavaScript execution error: {}", exception_str));
                } else {
                    return Err(anyhow!("Script execution returned no result"));
                }
            }
        };
        // Increment execution count
        self.execution_count.fetch_add(1, Ordering::SeqCst);
        if self.verbose {
            println!("Execution completed successfully");
        }
        // Convert result to string with better formatting
        let result_str: _ = if result.is_undefined() {
            "undefined".to_string()
        } else if result.is_null() {
            "null".to_string()
        } else if result.is_number() {
            if let Some(num) = result.to_integer(scope) {
                num.to_string()
            } else if let Some(num) = result.to_number(scope) {
                format!("{}", num)
            } else {
                "NaN".to_string()
            }
        } else if result.is_boolean() {
            if result.is_true() {
                "true".to_string()
            } else {
                "false".to_string()
            }
        } else if result.is_string() {
            let str_val: _ = result.to_string(scope)
                .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap());
            str_val.to_rust_string_lossy(scope)
        } else {
            // For objects, arrays, etc., use JSON.stringify equivalent
            let json_str: _ = v8::JSON::stringify(scope, result, None)
                .unwrap_or_else(|| v8::String::new(scope, "<unprintable>").unwrap());
            json_str.to_rust_string_lossy(scope)
        };
        Ok(result_str)
    }
    /// Set up console API for V8
    fn setup_console(&self, scope: &mut v8::ContextScope<v8::HandleScope>) -> Result<()> {
        let console: _ = v8::Object::new(scope);
        // console.log - standard output
        let console_log: _ = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            let mut output = String::new();
            for i in 0..args.length() {
                if i > 0 {
                    output.push(' ');
                }
                let arg: _ = args.get(i);
                // Use JSON.stringify for better formatting of complex objects
                let arg_str: _ = if arg.is_string() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_undefined() || arg.is_null() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "undefined").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_number() {
                    if let Some(num) = arg.to_integer(scope) {
                        num.to_string()
                    } else if let Some(num) = arg.to_number(scope) {
                        format!("{}", num)
                    } else {
                        "NaN".to_string()
                    }
                } else if arg.is_boolean() {
                    if arg.is_true() { "true".to_string() } else { "false".to_string() }
                } else {
                    // For objects, arrays, etc., use JSON.stringify equivalent
                    let json_str: _ = v8::JSON::stringify(scope, arg, None)
                        .unwrap_or_else(|| v8::String::new(scope, "<unprintable>").unwrap());
                    json_str.to_rust_string_lossy(scope)
                };
                output.push_str(&arg_str);
            }
            println!("{}", output);
            retval.set_undefined();
        });
        // console.error - error output (stderr)
        let console_error: _ = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            let mut output = String::new();
            for i in 0..args.length() {
                if i > 0 {
                    output.push(' ');
                }
                let arg: _ = args.get(i);
                let arg_str: _ = if arg.is_string() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_undefined() || arg.is_null() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "undefined").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_number() {
                    if let Some(num) = arg.to_integer(scope) {
                        num.to_string()
                    } else if let Some(num) = arg.to_number(scope) {
                        format!("{}", num)
                    } else {
                        "NaN".to_string()
                    }
                } else if arg.is_boolean() {
                    if arg.is_true() { "true".to_string() } else { "false".to_string() }
                } else {
                    let json_str: _ = v8::JSON::stringify(scope, arg, None)
                        .unwrap_or_else(|| v8::String::new(scope, "<unprintable>").unwrap());
                    json_str.to_rust_string_lossy(scope)
                };
                output.push_str(&arg_str);
            }
            eprintln!("{}", output);
            retval.set_undefined();
        });
        // console.warn - warning output (stderr)
        let console_warn: _ = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            let mut output = String::new();
            for i in 0..args.length() {
                if i > 0 {
                    output.push(' ');
                }
                let arg: _ = args.get(i);
                let arg_str: _ = if arg.is_string() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_undefined() || arg.is_null() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "undefined").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_number() {
                    if let Some(num) = arg.to_integer(scope) {
                        num.to_string()
                    } else if let Some(num) = arg.to_number(scope) {
                        format!("{}", num)
                    } else {
                        "NaN".to_string()
                    }
                } else if arg.is_boolean() {
                    if arg.is_true() { "true".to_string() } else { "false".to_string() }
                } else {
                    let json_str: _ = v8::JSON::stringify(scope, arg, None)
                        .unwrap_or_else(|| v8::String::new(scope, "<unprintable>").unwrap());
                    json_str.to_rust_string_lossy(scope)
                };
                output.push_str(&arg_str);
            }
            eprintln!("{}", output);
            retval.set_undefined();
        });
        // console.info - info output (stdout, same as log)
        let console_info: _ = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            let mut output = String::new();
            for i in 0..args.length() {
                if i > 0 {
                    output.push(' ');
                }
                let arg: _ = args.get(i);
                let arg_str: _ = if arg.is_string() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_undefined() || arg.is_null() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "undefined").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_number() {
                    if let Some(num) = arg.to_integer(scope) {
                        num.to_string()
                    } else if let Some(num) = arg.to_number(scope) {
                        format!("{}", num)
                    } else {
                        "NaN".to_string()
                    }
                } else if arg.is_boolean() {
                    if arg.is_true() { "true".to_string() } else { "false".to_string() }
                } else {
                    let json_str: _ = v8::JSON::stringify(scope, arg, None)
                        .unwrap_or_else(|| v8::String::new(scope, "<unprintable>").unwrap());
                    json_str.to_rust_string_lossy(scope)
                };
                output.push_str(&arg_str);
            }
            println!("{}", output);
            retval.set_undefined();
        });
        // console.debug - debug output
        let console_debug: _ = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            let mut output = String::new();
            for i in 0..args.length() {
                if i > 0 {
                    output.push(' ');
                }
                let arg: _ = args.get(i);
                let arg_str: _ = if arg.is_string() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "<error>").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_undefined() || arg.is_null() {
                    arg.to_string(scope)
                        .unwrap_or_else(|| v8::String::new(scope, "undefined").unwrap())
                        .to_rust_string_lossy(scope)
                } else if arg.is_number() {
                    if let Some(num) = arg.to_integer(scope) {
                        num.to_string()
                    } else if let Some(num) = arg.to_number(scope) {
                        format!("{}", num)
                    } else {
                        "NaN".to_string()
                    }
                } else if arg.is_boolean() {
                    if arg.is_true() { "true".to_string() } else { "false".to_string() }
                } else {
                    let json_str: _ = v8::JSON::stringify(scope, arg, None)
                        .unwrap_or_else(|| v8::String::new(scope, "<unprintable>").unwrap());
                    json_str.to_rust_string_lossy(scope)
                };
                output.push_str(&arg_str);
            }
            println!("[DEBUG] {}", output);
            retval.set_undefined();
        });
        // Set all console methods
        console.set(scope, "log", console_log.into()).map_err(|e| anyhow!("Failed to set console.log: {}", e))?;
        console.set(scope, "error", console_error.into()).map_err(|e| anyhow!("Failed to set console.error: {}", e))?;
        console.set(scope, "warn", console_warn.into()).map_err(|e| anyhow!("Failed to set console.warn: {}", e))?;
        console.set(scope, "info", console_info.into()).map_err(|e| anyhow!("Failed to set console.info: {}", e))?;
        console.set(scope, "debug", console_debug.into()).map_err(|e| anyhow!("Failed to set console.debug: {}", e))?;
        // Get the global object and set console
        let global: _ = context.global(scope);
        global.set(scope, "console", console).map_err(|e| anyhow!("Failed to set global console: {}", e))?;
        Ok(())
    }
    /// Execute JavaScript/TypeScript code
    pub fn execute_code(&self, code: &str) -> Result<String> {
        let base_dir: _ = PathBuf::from(".");
        self.execute_code_with_context(code, base_dir)
    }
    /// Get execution count
    pub fn execution_count(&self) -> usize {
        self.execution_count.load(Ordering::SeqCst)
    }
    /// Check if runtime is initialized
    pub fn is_initialized(&self) -> bool {
        true
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if self.verbose {
            let count: _ = self.execution_count.load(Ordering::SeqCst);
            println!("Runtime shutting down. Total executions: {}", count);
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_runtime_creation() {
        let runtime: _ = Runtime::new(67108864, 1073741824, false, false);
        assert!(runtime.is_ok());
        assert!(runtime.unwrap().is_initialized());
    }
    #[test]
    fn test_simple_code_execution() {
        let runtime: _ = Runtime::new(67108864, 1073741824, false, false);
        let result: _ = runtime.execute_code("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "2");
    }
    #[test]
    fn test_file_execution() {
        let runtime: _ = Runtime::new(67108864, 1073741824, false, false);
        // Create a temporary file with JavaScript code
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "const x = 42; x * 2;").unwrap();
        let result: _ = runtime.execute_file(&file.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "84");
    }
    #[test]
    fn test_execution_count() {
        let runtime: _ = Runtime::new(67108864, 1073741824, false, false);
        assert_eq!(runtime.execution_count(), 0);
        runtime.execute_code("1").unwrap();
        assert_eq!(runtime.execution_count(), 1);
        runtime.execute_code("2").unwrap();
        assert_eq!(runtime.execution_count(), 2);
    }
}