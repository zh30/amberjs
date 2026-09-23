// Platform Support Module
// Provides cross-platform runtime support for mobile and WebAssembly
pub mod mobile_runtime;
pub mod wasm_runtime;

use anyhow::Result;
use mobile_runtime::MobileRuntime;
use std::sync::Arc;
use wasm_runtime::{BeeWasmAPI, JS2WASMCompiler, WASMRuntime};

/// Unified cross-platform runtime
#[derive(Debug)]
pub struct CrossPlatformRuntime {
    mobile: Option<MobileRuntime>,
    wasm: Option<WASMRuntime>,
}
impl CrossPlatformRuntime {
    /// Create a new cross-platform runtime
    pub fn new() -> Self {
        CrossPlatformRuntime {
            mobile: None,
            wasm: None,
        }
    }
    /// Initialize mobile runtime
    pub fn init_mobile(&mut self) -> Result<()> {
        self.mobile = Some(MobileRuntime::new());
        Ok(())
    }
    /// Initialize WASM runtime
    pub fn init_wasm(&mut self, amber_api: Arc<dyn BeeWasmAPI>) -> Result<()> {
        self.wasm = Some(WASMRuntime::new(amber_api)?);
        Ok(())
    }
    /// Execute code on specified platform
    pub async fn execute(&self, platform: &str, code: &str) -> Result<String> {
        match platform.to_lowercase().as_str() {
            "ios" | "android" => {
                if let Some(mobile) = &self.mobile {
                    mobile.execute_mobile(platform, code).await
                } else {
                    Err(anyhow::anyhow!("Mobile runtime not initialized"))
                }
            }
            "wasm" | "webassembly" => {
                if let Some(wasm) = &self.wasm {
                    // For WASM, we need to compile first
                    let compiler: _ = JS2WASMCompiler::new()?;
                    let compilation: _ = compiler.compile_to_wasm(code).await?;
                    if !compilation.success {
                        return Err(anyhow::anyhow!("WASM compilation failed"));
                    }
                    let module_name: _ = "dynamic_module";
                    wasm.load_module(module_name.to_string(), compilation.module.binary)
                        .await?;
                    wasm.execute_wasm(module_name, "main", &[]).await
                } else {
                    Err(anyhow::anyhow!("WASM runtime not initialized"))
                }
            }
            _ => Err(anyhow::anyhow!("Unsupported platform: {}", platform)),
        }
    }
    /// Get supported platforms
    pub fn supported_platforms(&self) -> Vec<String> {
        let mut platforms = Vec::new();
        if self.mobile.is_some() {
            platforms.push("ios".to_string());
            platforms.push("android".to_string());
        }
        if self.wasm.is_some() {
            platforms.push("wasm".to_string());
            platforms.push("webassembly".to_string());
        }
        platforms
    }
    /// Check if platform is supported
    pub fn is_platform_supported(&self, platform: &str) -> bool {
        self.supported_platforms()
            .iter()
            .any(|p| p.to_lowercase() == platform.to_lowercase())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    struct MockBeeWasmAPI;
    impl BeeWasmAPI for MockBeeWasmAPI {
        fn console_log(&self, _message: &str) -> Result<(), anyhow::Error> {
            Ok(())
        }
        fn execute_js(&self, code: &str) -> Result<String, anyhow::Error> {
            Ok(format!("Executed: {}", code))
        }
        fn get_variable(&self, name: &str) -> Result<String, anyhow::Error> {
            Ok(format!("value_of_{}", name))
        }
        fn set_variable(&self, _name: &str, _value: &str) -> Result<(), anyhow::Error> {
            Ok(())
        }
    }
    #[tokio::test]
    async fn test_cross_platform_runtime() {
        let mut runtime = CrossPlatformRuntime::new();
        // Initialize mobile runtime
        runtime.init_mobile().unwrap();
        // Test iOS execution
        let result: _ = runtime.execute("ios", "console.log('Hello iOS')").await;
        assert!(result.is_ok());
        // Test Android execution
        let result: _ = runtime
            .execute("android", "console.log('Hello Android')")
            .await;
        assert!(result.is_ok());
        // Initialize WASM runtime
        let amber_api: _ = Arc::new(MockBeeWasmAPI);
        runtime.init_wasm(amber_api).unwrap();
        // Test WASM execution
        let result: _ = runtime
            .execute("wasm", "function main() { return 'Hello WASM'; }")
            .await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_supported_platforms() {
        let mut runtime = CrossPlatformRuntime::new();
        let platforms: _ = runtime.supported_platforms();
        assert!(platforms.is_empty());
        runtime.init_mobile().unwrap();
        let platforms: _ = runtime.supported_platforms();
        assert!(platforms.contains(&"ios".to_string()));
        assert!(platforms.contains(&"android".to_string()));
        let amber_api: _ = Arc::new(MockBeeWasmAPI);
        runtime.init_wasm(amber_api).unwrap();
        let platforms: _ = runtime.supported_platforms();
        assert!(platforms.contains(&"wasm".to_string()));
    }
    #[tokio::test]
    async fn test_platform_support_check() {
        let mut runtime = CrossPlatformRuntime::new();
        assert!(!runtime.is_platform_supported("ios"));
        runtime.init_mobile().unwrap();
        assert!(runtime.is_platform_supported("ios"));
        assert!(runtime.is_platform_supported("android"));
        assert!(!runtime.is_platform_supported("wasm"));
    }
}
