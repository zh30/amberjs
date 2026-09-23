// Amber Plugin Engine - Stage 86
// 高性能、安全的插件引擎核心实现

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================================
// 核心类型定义
// ============================================================================
/// 插件 ID
pub type PluginId = String;
/// 插件状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    /// 已加载但未激活
    Loaded,
    /// 激活中
    Active,
    /// 已停用
    Inactive,
    /// 发生错误
    Error,
    /// 已崩溃
    Crashed,
}
/// 插件错误
#[derive(Debug, Clone)]
pub enum PluginError {
    /// 权限被拒绝
    PermissionDenied(String),
    /// 加载失败
    LoadFailed(String),
    /// 执行失败
    ExecutionFailed(String),
    /// 超时
    Timeout,
    /// 资源限制
    ResourceLimitExceeded(String),
    /// 未找到
    NotFound(String),
    /// 无效配置
    InvalidConfig(String),
}
impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            PluginError::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
            PluginError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            PluginError::Timeout => write!(f, "Plugin execution timed out"),
            PluginError::ResourceLimitExceeded(msg) => write!(f, "Resource limit exceeded: {}", msg),
            PluginError::NotFound(msg) => write!(f, "Plugin not found: {}", msg),
            PluginError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}
impl std::error::Error for PluginError {}
/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件唯一 ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 入口点文件
    pub entry_point: String,
    /// 所需权限
    pub permissions: Vec<String>,
    /// 依赖项
    pub dependencies: HashMap<String, String>,
}
impl PluginMetadata {
    /// 创建简单的插件元数据
    pub fn simple(id: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            version: version.to_string(),
            author: "Unknown".to_string(),
            description: format!("Plugin {}", id),
            entry_point: "index.js".to_string(),
            permissions: vec![],
            dependencies: HashMap::new(),
        }
    }
}
/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 启用沙箱
    pub sandbox_enabled: bool,
    /// 超时时间 (毫秒)
    pub timeout_ms: u64,
    /// 最大内存 (MB)
    pub max_memory_mb: u64,
    /// 自定义选项
    pub options: HashMap<String, Value>,
}
impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            timeout_ms: 5000,
            max_memory_mb: 100,
            options: HashMap::new(),
        }
    }
}
/// 插件句柄
#[derive(Debug, Clone)]
pub struct PluginHandle {
    id: PluginId,
    status: Arc<std::sync::RwLock<PluginStatus>>,
}
impl PluginHandle {
    /// 获取插件 ID
    pub fn plugin_id(&self) -> &str {
        &self.id
    }
    /// 获取插件状态
    pub fn status(&self) -> PluginStatus {
        self.status.read().unwrap().clone()
    }
    /// 设置状态
    fn set_status(&self, status: PluginStatus) {
        *self.status.write().unwrap() = status;
    }
}
/// 插件执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    /// 是否成功
    pub success: bool,
    /// 返回数据
    pub data: Option<Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间 (毫秒)
    pub execution_time_ms: u64,
}
impl PluginResult {
    /// 成功结果
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms: 0,
        }
    }
    /// 失败结果
    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
            execution_time_ms: 0,
        }
    }
}
// ============================================================================
// 权限系统
// ============================================================================
/// 权限集合
#[derive(Debug, Clone)]
pub struct PermissionSet {
    granted: Vec<String>,
}
impl PermissionSet {
    /// 创建最小权限集
    pub fn minimal() -> Self {
        Self { granted: vec![] }
    }
    /// 创建完全权限集
    pub fn full() -> Self {
        Self {
            granted: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "net.fetch".to_string(),
                "net.listen".to_string(),
                "env.read".to_string(),
                "runtime.execute".to_string(),
            ],
        }
    }
    /// 授予权限
    pub fn grant(&mut self, permission: &str) {
        if !self.granted.contains(&permission.to_string()) {
            self.granted.push(permission.to_string());
        }
    }
    /// 撤销权限
    pub fn revoke(&mut self, permission: &str) {
        self.granted.retain(|p| p != permission);
    }
    /// 检查权限
    pub fn has(&self, permission: &str) -> bool {
        self.granted.iter().any(|p| {
            p == permission || permission.starts_with(&format!("{}.", p))
        })
    }
}
/// 资源限制
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// 最大内存 (MB)
    pub max_memory_mb: u64,
    /// 最大 CPU 时间 (毫秒)
    pub max_cpu_time_ms: u64,
    /// 最大文件句柄数
    pub max_file_handles: u32,
    /// 最大网络连接数
    pub max_network_connections: u32,
}
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 100,
            max_cpu_time_ms: 5000,
            max_file_handles: 100,
            max_network_connections: 10,
        }
    }
}
// ============================================================================
// 插件沙箱
// ============================================================================
/// 沙箱执行上下文
pub struct SandboxContext {
    permissions: PermissionSet,
}
impl SandboxContext {
    /// 访问文件系统
    pub fn access_fs(&self, _path: &str) -> Result<(), PluginError> {
        if self.permissions.has("fs.read") || self.permissions.has("fs.write") {
            Ok(())
        } else {
            Err(PluginError::PermissionDenied("fs access not allowed".to_string())
        }
    }
}
/// 插件沙箱
pub struct PluginSandbox {
    permissions: Arc<PermissionSet>,
    limits: ResourceLimits,
}
impl PluginSandbox {
    /// 创建新沙箱
    pub fn new(permissions: PermissionSet) -> Self {
        Self {
            permissions: Arc::new(Mutex::new(permissions)))
            limits: ResourceLimits::default(),
        }
    }
    /// 创建带资源限制的沙箱
    pub fn with_limits(permissions: PermissionSet, limits: ResourceLimits) -> Self {
        Self {
            permissions: Arc::new(Mutex::new(permissions)))
            limits,
        }
    }
    /// 获取内存限制
    pub fn memory_limit(&self) -> u64 {
        self.limits.max_memory_mb
    }
    /// 获取 CPU 时间限制
    pub fn cpu_time_limit(&self) -> u64 {
        self.limits.max_cpu_time_ms
    }
    /// 检查权限
    pub fn check_permission(&self, permission: &str) -> Result<(), PluginError> {
        if self.permissions.has(permission) {
            Ok(())
        } else {
            Err(PluginError::PermissionDenied(format!(
                "Permission '{}' not granted",
                permission
            ))
        }
    }
    /// 在沙箱中执行
    pub async fn execute_in_sandbox<F, T>(&self, f: F) -> Result<T, PluginError>
    where
        F: FnOnce(&SandboxContext) -> Result<T, PluginError>,
    {
        let ctx: _ = SandboxContext {
            permissions: (*self.permissions).clone(),
        };
        f(&ctx)
    }
    /// 带超时执行
    pub async fn execute_with_timeout<F, T>(&self, f: F) -> Result<T, PluginError>
    where
        F: Future<Output = Result<T, PluginError>> + Send,
    {
        let timeout: _ = tokio::time::Duration::from_millis(self.limits.max_cpu_time_ms);
        match tokio::time::timeout(timeout, f).await {
            Ok(result) => result,
            Err(_) => Err(PluginError::Timeout),
        }
    }
}
// ============================================================================
// 插件注册表
// ============================================================================
/// 插件注册表
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<PluginId, PluginMetadata>>>,
}
impl PluginRegistry {
    /// 创建新注册表
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new()))
        }
    }
    /// 注册插件
    pub async fn register(&self, metadata: &PluginMetadata) -> Result<PluginId, PluginError> {
        let id: _ = metadata.id.clone();
        let mut plugins = self.plugins.write().await;
        plugins.insert(id.clone(), metadata.clone());
        Ok(id)
    }
    /// 注销插件
    pub async fn unregister(&self, id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        plugins.remove(id);
        Ok(())
    }
    /// 检查是否已注册 (异步版本)
    pub async fn is_registered_async(&self, id: &PluginId) -> bool {
        let plugins: _ = self.plugins.read().await;
        plugins.contains_key(id)
    }
    /// 检查是否已注册 (同步版本 - 用于测试)
    pub fn is_registered(&self, id: &PluginId) -> bool {
        // 使用 try_read 避免死锁，失败时返回 false
        if let Ok(plugins) = self.plugins.try_read() {
            plugins.contains_key(id)
        } else {
            false
        }
    }
    /// 获取插件元数据
    pub async fn get(&self, id: &str) -> Option<PluginMetadata> {
        let plugins: _ = self.plugins.read().await;
        plugins.get(id).cloned()
    }
    /// 发现所有插件
    pub async fn discover_all(&self) -> Vec<PluginMetadata> {
        let plugins: _ = self.plugins.read().await;
        plugins.values().cloned().collect()
    }
    /// 搜索插件
    pub async fn search(&self, query: &str) -> Vec<PluginMetadata> {
        let plugins: _ = self.plugins.read().await;
        plugins
            .values()
            .filter(|p| {
                p.name.contains(query)
                    || p.id.contains(query)
                    || p.description.contains(query)
            })
            .cloned()
            .collect()
    }
}
// ============================================================================
// 插件加载器
// ============================================================================
/// 插件加载器
pub struct PluginLoader {
    /// 支持的语言
    supported_languages: Vec<String>,
}
impl PluginLoader {
    /// 创建新加载器
    pub fn new() -> Self {
        Self {
            supported_languages: vec![
                "javascript".to_string(),
                "typescript".to_string(),
                "wasm".to_string(),
            ],
        }
    }
    /// 从源码加载
    pub async fn load_from_source(
        &self,
        code: &str,
        language: &str,
    ) -> Result<LoadedPlugin, PluginError> {
        if !self.supported_languages.contains(&language.to_lowercase()) {
            return Err(PluginError::LoadFailed(format!(
                "Unsupported language: {}",
                language
            ));
        }
        Ok(LoadedPlugin {
            code: code.to_string(),
            language: language.to_string(),
            compiled: false,
        })
    }
    /// 从 WASM 加载
    pub async fn load_from_wasm(&self, _bytes: &[u8]) -> Result<LoadedPlugin, PluginError> {
        Ok(LoadedPlugin {
            code: String::new(),
            language: "wasm".to_string(),
            compiled: true,
        })
    }
    /// 解析依赖
    pub async fn resolve_dependencies(
        &self,
        deps: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, PluginError> {
        // 模拟依赖解析
        let mut resolved = HashMap::new();
        for (name, version) in deps {
            resolved.insert(name.clone(), version.clone());
        }
        Ok(resolved)
    }
}
/// 已加载的插件
pub struct LoadedPlugin {
    pub code: String,
    pub language: String,
    pub compiled: bool,
}
// ============================================================================
// 插件 API
// ============================================================================
/// 插件 API
pub struct PluginAPI {
    /// API 版本
    versions: Vec<String>,
}
impl PluginAPI {
    /// 创建新 API
    pub fn new() -> Self {
        Self {
            versions: vec!["v1".to_string(), "v2".to_string()],
        }
    }
    /// 调用 API
    pub async fn call(&self, method: &str, params: &Value) -> Result<Value, PluginError> {
        match method {
            "log" => {
                let level: _ = params.get("level").and_then(|v| v.as_str()).unwrap_or("info");
                let message: _ = params.get("message").and_then(|v| v.as_str()).unwrap_or("");
                println!("[Plugin {}] {}", level.to_uppercase(), message);
                Ok(serde_json::json!({"success": true}))
            }
            "echo" => Ok(params.clone()),
            _ => Err(PluginError::NotFound(format!("API method '{}' not found", method)),
        }
    }
    /// 调用带版本的 API
    pub async fn call_versioned(
        &self,
        version: &str,
        method: &str,
        params: &Value,
    ) -> Result<Value, PluginError> {
        if !self.versions.contains(&version.to_string()) {
            return Err(PluginError::NotFound(format!("API version '{}' not found", version));
        }
        self.call(method, params).await
    }
}
// ============================================================================
// 插件接口 trait
// ============================================================================
/// 插件接口
#[async_trait::async_trait]
pub trait PluginInterface: Send + Sync {
    /// 初始化
    async fn initialize(&self, config: PluginConfig) -> Result<(), PluginError>;
    /// 执行
    async fn execute(&self, input: &Value) -> Result<Value, PluginError>;
    /// 关闭
    async fn shutdown(&self) -> Result<(), PluginError>;
}
// ============================================================================
// 插件引擎
// ============================================================================
/// 插件实例
struct PluginInstance {
    metadata: PluginMetadata,
    status: Arc<std::sync::RwLock<PluginStatus>>,
    sandbox: PluginSandbox,
    connections: Vec<String>,
}
/// 插件引擎
pub struct PluginEngine {
    /// 是否已初始化
    initialized: AtomicBool,
    /// 注册表
    registry: Arc<PluginRegistry>,
    /// 加载器
    loader: Arc<PluginLoader>,
    /// API
    api: Arc<PluginAPI>,
    /// 插件实例
    instances: Arc<RwLock<HashMap<PluginId, PluginInstance>>>,
}
impl PluginEngine {
    /// 创建新引擎
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            registry: Arc::new(Mutex::new(PluginRegistry::new()))
            loader: Arc::new(Mutex::new(PluginLoader::new()))
            api: Arc::new(Mutex::new(PluginAPI::new()))
            instances: Arc::new(Mutex::new(HashMap::new()))
        }
    }
    /// 初始化引擎
    pub async fn initialize(&self) -> Result<(), PluginError> {
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }
    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }
    /// 加载插件
    pub async fn load_plugin(&self, metadata: &PluginMetadata) -> Result<PluginHandle, PluginError> {
        // 注册插件
        self.registry.register(metadata).await?;
        // 创建沙箱
        let mut permissions = PermissionSet::minimal();
        for perm in &metadata.permissions {
            permissions.grant(perm);
        }
        let sandbox: _ = PluginSandbox::new(permissions);
        // 创建实例
        let status: _ = Arc::new(Mutex::new(std::sync::RwLock::new(PluginStatus::Loaded)),;
        let instance: _ = PluginInstance {
            metadata: metadata.clone(),
            status: status.clone(),
            sandbox,
            connections: vec![],
        };
        // 存储实例
        let mut instances = self.instances.write().await;
        instances.insert(metadata.id.clone(), instance);
        Ok(PluginHandle {
            id: metadata.id.clone(),
            status,
        })
    }
    /// 卸载插件
    pub async fn unload_plugin(&self, handle: &PluginHandle) -> Result<(), PluginError> {
        let mut instances = self.instances.write().await;
        instances.remove(&handle.id);
        self.registry.unregister(&handle.id).await?;
        Ok(())
    }
    /// 检查插件是否存在 (异步版本)
    pub async fn has_plugin_async(&self, id: &str) -> bool {
        let instances: _ = self.instances.read().await;
        instances.contains_key(id)
    }
    /// 检查插件是否存在 (同步版本 - 用于测试)
    pub fn has_plugin(&self, id: &str) -> bool {
        if let Ok(instances) = self.instances.try_read() {
            instances.contains_key(id)
        } else {
            false
        }
    }
    /// 获取插件状态
    pub fn get_plugin_status(&self, handle: &PluginHandle) -> PluginStatus {
        handle.status()
    }
    /// 激活插件
    pub async fn activate_plugin(&self, handle: &PluginHandle) -> Result<(), PluginError> {
        let instances: _ = self.instances.read().await;
        if instances.contains_key(&handle.id) {
            handle.set_status(PluginStatus::Active);
            Ok(())
        } else {
            Err(PluginError::NotFound(handle.id.clone())
        }
    }
    /// 停用插件
    pub async fn deactivate_plugin(&self, handle: &PluginHandle) -> Result<(), PluginError> {
        let instances: _ = self.instances.read().await;
        if instances.contains_key(&handle.id) {
            handle.set_status(PluginStatus::Inactive);
            Ok(())
        } else {
            Err(PluginError::NotFound(handle.id.clone())
        }
    }
    /// 执行插件
    pub async fn execute_plugin(
        &self,
        handle: &PluginHandle,
        input: &Value,
    ) -> Result<PluginResult, PluginError> {
        let start: _ = std::time::Instant::now();
        let instances: _ = self.instances.read().await;
        let _instance: _ = instances
            .get(&handle.id)
            .ok_or_else(|| PluginError::NotFound(handle.id.clone())?;
        // 模拟插件执行
        let action: _ = input.get("action").and_then(|v| v.as_str()).unwrap_or("default");
        let result: _ = match action {
            "throw_error" => PluginResult::failure("Simulated error"),
            _ => PluginResult::success(serde_json::json!({
                "input": input,
                "processed": true
            })),
        };
        let mut result = result;
        result.execution_time_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }
    /// 列出所有插件
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.registry.discover_all().await
    }
    /// 连接插件
    pub async fn connect_plugins(
        &self,
        from: &PluginHandle,
        _to: &PluginHandle,
        channel: &str,
    ) -> Result<(), PluginError> {
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.get_mut(&from.id) {
            instance.connections.push(channel.to_string());
            Ok(())
        } else {
            Err(PluginError::NotFound(from.id.clone())
        }
    }
    /// 获取插件连接
    pub async fn get_plugin_connections(&self, handle: &PluginHandle) -> Vec<String> {
        let instances: _ = self.instances.read().await;
        instances
            .get(&handle.id)
            .map(|i| i.connections.clone())
            .unwrap_or_default()
    }
}
impl Default for PluginEngine {
    fn default() -> Self {
        Self::new()
    }
}
// ============================================================================
// 模块测试
// ============================================================================
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_plugin_engine_new() {
        let engine: _ = PluginEngine::new();
        assert!(!engine.is_initialized());
    }
    #[tokio::test]
    async fn test_plugin_metadata_simple() {
        let meta: _ = PluginMetadata::simple("test", "1.0.0");
        assert_eq!(meta.id, "test");
        assert_eq!(meta.version, "1.0.0");
    }
    #[tokio::test]
    async fn test_permission_set() {
        let mut perms = PermissionSet::minimal();
        assert!(!perms.has("fs.read"));
        perms.grant("fs.read");
        assert!(perms.has("fs.read"));
        perms.revoke("fs.read");
        assert!(!perms.has("fs.read"));
    }
    #[tokio::test]
    async fn test_plugin_result() {
        let success: _ = PluginResult::success(serde_json::json!({"test": true}));
        assert!(success.success);
        let failure: _ = PluginResult::failure("error");
        assert!(!failure.success);
    }
}