// Beejs 运行时核心模块
// 包含 MinimalRuntime 的完整实现和扩展功能

use std::sync::{Arc, Mutex};
use std::collections::{HashMap};
use thiserror::Error;
use rusty_v8 as v8;
use std::time::Instant;
use std::task::Context;

/// 运行时错误类型
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("V8 initialization failed: {0}")]
    V8InitError(String),

    #[error("Script compilation failed: {0}")]
    CompilationError(String),

    #[error("Script execution failed: {0}")]
    ExecutionError(String),

    #[error("Module loading failed: {0}")]
    ModuleLoadError(String),

    #[error("Context creation failed")]
    ContextError,

    #[error("Invalid code: {0}")]
    InvalidCode(String),
}

/// 运行时统计信息
#[derive(Debug, Clone, Default)]
pub struct RuntimeStats {
    pub execution_count: u64,
    pub compilation_count: u64,
    pub error_count: u64,
    pub total_execution_time_ms: u64,
}

/// 核心运行时结构体
pub struct CoreRuntime {
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    module_cache: Arc<Mutex<HashMap<String, v8::Global<v8::Module>>>>,
    stats: Arc<Mutex<RuntimeStats>>,
}

impl CoreRuntime {
    /// 创建新的运行时实例
    pub fn new() -> Result<Self, RuntimeError> {
        // 初始化 V8 平台（全局一次）
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            v8::V8::initialize_platform(v8::new_default_platform().unwrap());
            v8::V8::initialize();
        });

        let isolate = v8::Isolate::new(v8::CreateParams::default());
        let scope = &mut v8::HandleScope::new(isolate);

        // 创建上下文
        let context = v8::Context::new(scope, Default::default());
        let context_global = v8::Global::new(scope, context);

        // 设置全局对象（console.log 等）
        Self::setup_globals(scope, context);

        Ok(Self {
            isolate: scope.into_owned(),
            context: context_global,
            module_cache: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(RuntimeStats::default())),
        })
    }

    /// 设置全局对象（console, setTimeout 等）
    fn setup_globals(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) {
        let global = context.global(scope);

        // 设置 console 对象
        let console_object = v8::Object::new(scope);
        let console_log = v8::FunctionTemplate::new(scope, |args| {
            let handle_scope = args.isolate();
            for i in 0..args.length() {
                let arg = args.get(i);
                println!("{}", arg.to_string(handle_scope).unwrap_or_else(|| "undefined".into()));
            }
            v8::undefined(handle_scope).into()
        });
        console_object.set(
            scope,
            v8::String::new(scope, "log").unwrap().as_ref(),
            console_log.get_function(scope).unwrap().into(),
        );
        global.set(
            scope,
            v8::String::new(scope, "console").unwrap().as_ref(),
            console_object.into(),
        );

        // 设置 setTimeout
        let set_timeout = v8::FunctionTemplate::new(scope, |args| {
            println!("setTimeout called (not implemented)");
            v8::undefined(args.isolate()).into()
        });
        global.set(
            scope,
            v8::String::new(scope, "setTimeout").unwrap().as_ref(),
            set_timeout.get_function(scope).unwrap().into(),
        );
    }

    /// 执行 JavaScript 代码
    pub fn execute(&self, code: &str) -> Result<String, RuntimeError> {
        let start_time = std::time::Instant::now();

        let isolate = &self.isolate;
        let mut scope = v8::HandleScope::new(isolate);

        let context = v8::Local::new(&mut scope, &self.context);
        let mut ctx_scope = v8::ContextScope::new(&mut scope, context);

        // 更新统计
        {
            let mut stats = self.stats.lock().unwrap();
            stats.compilation_count += 1;
        }

        // 编译脚本
        let source = v8::String::new(&mut ctx_scope, code)
            .ok_or_else(|| RuntimeError::InvalidCode("Failed to create source string".to_string()))?;

        let script = v8::Script::compile(&mut ctx_scope, source, None)
            .ok_or_else(|| RuntimeError::CompilationError("Script compilation failed".to_string()))?;

        // 执行脚本
        let result = script.run(&mut ctx_scope)
            .ok_or_else(|| RuntimeError::ExecutionError("Script execution failed".to_string()))?;

        let result_str = result.to_string(&mut ctx_scope);

        // 更新统计
        let elapsed = start_time.elapsed();
        {
            let mut stats = self.stats.lock().unwrap();
            stats.execution_count += 1;
            stats.total_execution_time_ms += elapsed.as_millis() as u64;
        }

        Ok(result_str.to_string())
    }

    /// 加载并执行模块
    pub fn load_module(&self, module_name: &str, code: &str) -> Result<String, RuntimeError> {
        let isolate = &self.isolate;
        let mut scope = v8::HandleScope::new(isolate);

        let context = v8::Local::new(&mut scope, &self.context);
        let mut ctx_scope = v8::ContextScope::new(&mut scope, context);

        // 创建模块
        let source = v8::String::new(&mut ctx_scope, code)
            .ok_or_else(|| RuntimeError::InvalidCode("Failed to create module source".to_string()))?;

        let script_origin = v8::ScriptOrigin::new(
            &mut ctx_scope,
            module_name.into(),
            0,
            0,
            false,
            0,
            v8::Local::new(&mut ctx_scope, &v8::undefined(&mut ctx_scope)),
            false,
            false,
        );

        let script = v8::Script::compile(&mut ctx_scope, source, Some(&script_origin))
            .ok_or_else(|| RuntimeError::ModuleLoadError("Module compilation failed".to_string()))?;

        // 缓存模块
        {
            let mut cache = self.module_cache.lock().unwrap();
            let module_global = v8::Global::new(&mut ctx_scope, script);
            cache.insert(module_name.to_string(), module_global);
        }

        // 执行模块
        let result = script.run(&mut ctx_scope)
            .ok_or_else(|| RuntimeError::ModuleLoadError("Module execution failed".to_string()))?;

        Ok(result.to_string(&mut ctx_scope).to_string())
    }

    /// 获取运行时统计信息
    pub fn get_stats(&self) -> RuntimeStats {
        self.stats.lock().unwrap().clone()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = RuntimeStats::default();
    }

    /// 获取模块缓存大小
    pub fn get_cached_modules_count(&self) -> usize {
        self.module_cache.lock().unwrap().len()
    }

    /// 清空模块缓存
    pub fn clear_module_cache(&self) {
        self.module_cache.lock().unwrap().clear();
    }
}

impl Drop for CoreRuntime {
    fn drop(&mut self) {
        // 清理 V8 资源
        // 注意：在实际实现中可能需要更复杂的清理逻辑
    }
}

/// 简化版运行时（用于测试和快速原型）
pub struct MinimalRuntime {
    runtime: Option<CoreRuntime>,
}

impl MinimalRuntime {
    pub fn new() -> Self {
        Self { runtime: None }
    }

    /// 初始化运行时
    pub fn initialize(&mut self) -> Result<(), RuntimeError> {
        if self.runtime.is_none() {
            self.runtime = Some(CoreRuntime::new()?);
        }
        Ok(())
    }

    /// 执行代码
    pub fn execute(&self, code: &str) -> Result<String, RuntimeError> {
        match &self.runtime {
            Some(runtime) => runtime.execute(code),
            None => Err(RuntimeError::V8InitError("Runtime not initialized".to_string())),
        }
    }

    /// 加载模块
    pub fn load_module(&self, module_name: &str, code: &str) -> Result<String, RuntimeError> {
        match &self.runtime {
            Some(runtime) => runtime.load_module(module_name, code),
            None => Err(RuntimeError::V8InitError("Runtime not initialized".to_string())),
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> Option<RuntimeStats> {
        self.runtime.as_ref().map(|r| r.get_stats())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_minimal_runtime_creation() {
        let mut runtime = MinimalRuntime::new();
        let result = runtime.initialize();
        assert!(result.is_ok());
    }

    #[test]
    fn test_minimal_runtime_execution() {
        let mut runtime = MinimalRuntime::new();
        runtime.initialize().unwrap();

        let result = runtime.execute("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2");
    }

    #[test]
    fn test_minimal_runtime_uninitialized() {
        let runtime = MinimalRuntime::new();
        let result = runtime.execute("1 + 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_core_runtime_execution() {
        let runtime = CoreRuntime::new().unwrap();
        let result = runtime.execute("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2");
    }

    #[test]
    fn test_console_log() {
        let runtime = CoreRuntime::new().unwrap();
        let result = runtime.execute("console.log('test message')");
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_loading() {
        let runtime = CoreRuntime::new().unwrap();
        let code = "const module = { exports: {} }; module.exports;";
        let result = runtime.load_module("test_module", code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stats_tracking() {
        let runtime = CoreRuntime::new().unwrap();
        assert_eq!(runtime.get_stats().execution_count, 0);

        runtime.execute("1 + 1").unwrap();
        let stats = runtime.get_stats();
        assert_eq!(stats.execution_count, 1);
        assert_eq!(stats.compilation_count, 1);
    }
}
