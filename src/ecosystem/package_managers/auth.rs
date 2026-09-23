// 认证管理器
// Stage 91 Phase 3.1 - 包管理器认证
//
/// 处理 npm 注册表认证、私有仓库访问等
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::{BTreeMap};
/// 认证类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthType {
    /// Bearer token
    Bearer(String),
    /// Basic auth (username/password)
    Basic(String, String),
    /// Auth token (npm 格式)
    NpmAuth(String),
    /// 自定义头部
    Custom(HashMap<String, String>),
}
/// 认证信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    pub registry_url: String,
    pub auth_type: AuthType,
    pub email: Option<String>,
    pub always_auth: bool,
    pub _auth: Option<String>, // npm 兼容字段
}
/// 认证管理器
#[derive(Debug)]
pub struct AuthManager {
    auth_configs: HashMap<String, AuthInfo>,
    default_registry: String,
}
impl AuthManager {
    /// 创建新的认证管理器
    pub fn new() -> Self {
        let mut manager = Self {
            auth_configs: HashMap::new(),
            default_registry: "https://registry.npmjs.org/".to_string(),
        };
        // 从环境变量加载认证信息
        manager.load_from_env();
        manager.load_from_npmrc();
        manager
    }
    /// 从环境变量加载认证
    fn load_from_env(&mut self) {
        // 从环境变量加载认证 token
        if let Ok(token) = std::env::var("NPM_TOKEN") {
            let auth_info: _ = AuthInfo {
                registry_url: "https://registry.npmjs.org/".to_string(),
                auth_type: AuthType::NpmAuth(token),
                email: std::env::var("NPM_EMAIL").ok(),
                always_auth: false,
                _auth: Some(token),
            };
            self.auth_configs.insert("https://registry.npmjs.org/".to_string(), auth_info);
        }
        // 从环境变量加载私有仓库认证
        if let Ok(url) = std::env::var("AMBER_PRIVATE_REGISTRY") {
            if let Ok(token) = std::env::var("AMBER_PRIVATE_REGISTRY_TOKEN") {
                let auth_info: _ = AuthInfo {
                    registry_url: url.clone(),
                    auth_type: AuthType::Bearer(token),
                    email: std::env::var("NPM_EMAIL").ok(),
                    always_auth: true,
                    _auth: None,
                };
                self.auth_configs.insert(url, auth_info);
            }
        }
    }
    /// 从 .npmrc 文件加载认证
    fn load_from_npmrc(&mut self) {
        if let Ok(home_dir) = std::env::var("HOME") {
            let npmrc_path: _ = format!("{}/.npmrc", home_dir));
            if let Ok(content) = std::fs::read_to_string(&npmrc_path) {
                self.parse_npmrc(&content);
            }
        }
        // 也检查项目目录的 .npmrc
        if let Ok(content) = std::fs::read_to_string(".npmrc") {
            self.parse_npmrc(&content);
        }
    }
    /// 解析 .npmrc 文件内容
    fn parse_npmrc(&mut self, content: &str) {
        for line in content.lines() {
            let line: _ = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key: _ = key.trim();
                let value: _ = value.trim();
                match key {
                    "//registry.npmjs.org/:_auth" => {
                        let auth_info: _ = AuthInfo {
                            registry_url: "https://registry.npmjs.org/".to_string(),
                            auth_type: AuthType::NpmAuth(value.to_string()),
                            email: None,
                            always_auth: false,
                            _auth: Some(value.to_string()),
                        };
                        self.auth_configs.insert("https://registry.npmjs.org/".to_string(), auth_info);
                    }
                    "registry" => {
                        self.default_registry = value.to_string();
                    }
                    _ if key.starts_with("//") && key.ends_with("/:_auth") => {
                        let registry_url: _ = key[..key.len() - 6].to_string();
                        let auth_info: _ = AuthInfo {
                            registry_url: registry_url.clone(),
                            auth_type: AuthType::NpmAuth(value.to_string()),
                            email: None,
                            always_auth: true,
                            _auth: Some(value.to_string()),
                        };
                        self.auth_configs.insert(registry_url, auth_info);
                    }
                    _ if key.starts_with("//") && key.ends_with("/:auth") => {
                        let registry_url: _ = key[..key.len() - 6].to_string();
                        let auth_info: _ = AuthInfo {
                            registry_url: registry_url.clone(),
                            auth_type: AuthType::Bearer(value.to_string()),
                            email: None,
                            always_auth: true,
                            _auth: None,
                        };
                        self.auth_configs.insert(registry_url, auth_info);
                    }
                    _ => {}
                }
            }
        }
    }
    /// 添加认证配置
    pub fn add_auth(&mut self, registry_url: String, auth_info: AuthInfo) {
        self.auth_configs.insert(registry_url, auth_info);
    }
    /// 获取注册表的认证信息
    pub fn get_auth(&self, registry_url: &str) -> Option<&AuthInfo> {
        // 精确匹配
        if let Some(auth) = self.auth_configs.get(registry_url) {
            return Some(auth);
        }
        // 尝试查找匹配的注册表 URL
        for (registered_url, auth) in &self.auth_configs {
            if registry_url.starts_with(registered_url.trim_end_matches('/')) {
                return Some(auth);
            }
        }
        None
    }
    /// 为请求添加认证头
    pub fn add_auth_headers(&self, request_builder: reqwest::RequestBuilder, registry_url: &str) -> reqwest::RequestBuilder {
        if let Some(auth_info) = self.get_auth(registry_url) {
            match &auth_info.auth_type {
                AuthType::Bearer(token) => {
                    request_builder.bearer_auth(token)
                }
                AuthType::NpmAuth(token) => {
                    let base64_token: _ = base64::encode(token);
                    request_builder.header("Authorization", format!("Bearer {}", base64_token))
                }
                AuthType::Basic(username, password) => {
                    let credentials: _ = base64::encode(&format!("{}:{}", username, password));
                    request_builder.header("Authorization", format!("Basic {}", credentials))
                }
                AuthType::Custom(headers) => {
                    let mut builder = request_builder;
                    for (key, value) in headers {
                        builder = builder.header(key, value);
                    }
                    builder
                }
            }
        } else {
            request_builder
        }
    }
    /// 检查是否需要认证
    pub fn requires_auth(&self, registry_url: &str) -> bool {
        if let Some(auth_info) = self.get_auth(registry_url) {
            auth_info.always_auth
        } else {
            false
        }
    }
    /// 登录
    pub async fn login(
        &mut self,
        registry_url: &str,
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 实际实现需要调用注册表 API 进行认证
        // 这里只是模拟
        let auth_info: _ = AuthInfo {
            registry_url: registry_url.to_string(),
            auth_type: AuthType::Basic(username.to_string(), password.to_string()),
            email: email.map(|s| s.to_string()),
            always_auth: true,
            _auth: None,
        };
        self.add_auth(registry_url.to_string(), auth_info);
        // 保存到 .npmrc
        self.save_to_npmrc().await?;
        Ok(())
    }
    /// 登出
    pub async fn logout(&mut self, registry_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.auth_configs.remove(registry_url);
        // 从 .npmrc 移除
        self.save_to_npmrc().await?;
        Ok(())
    }
    /// 登录（使用 token）
    pub async fn login_with_token(&mut self, registry_url: &str, token: &str) -> Result<(), Box<dyn std::error::Error>> {
        let auth_info: _ = AuthInfo {
            registry_url: registry_url.to_string(),
            auth_type: AuthType::NpmAuth(token.to_string()),
            email: None,
            always_auth: true,
            _auth: Some(token.to_string()),
        };
        self.add_auth(registry_url.to_string(), auth_info);
        // 保存到 .npmrc
        self.save_to_npmrc().await?;
        Ok(())
    }
    /// 保存认证信息到 .npmrc
    async fn save_to_npmrc(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(home_dir) = std::env::var("HOME") {
            let npmrc_path: _ = format!("{}/.npmrc", home_dir));
            let mut content = String::new();
            // 读取现有内容
            if let Ok(existing) = std::fs::read_to_string(&npmrc_path) {
                content = existing;
            }
            // 添加认证信息
            for (registry_url, auth_info) in &self.auth_configs {
                match &auth_info.auth_type {
                    AuthType::NpmAuth(token) => {
                        let auth_line: _ = format!("{}:_auth={}\n", registry_url.trim_end_matches('/'), token);
                        if !content.contains(&format!("{}:_auth", registry_url.trim_end_matches('/')) {
                            content.push_str(&auth_line);
                        }
                    }
                    AuthType::Bearer(token) => {
                        let auth_line: _ = format!("{}:auth={}\n", registry_url.trim_end_matches('/'), token);
                        if !content.contains(&format!("{}:auth", registry_url.trim_end_matches('/')) {
                            content.push_str(&auth_line);
                        }
                    }
                    _ => {}
                }
            }
            // 添加默认注册表
            if !content.contains("registry=") {
                content.push_str(&format!("registry={}\n", self.default_registry));
            }
            std::fs::write(&npmrc_path, content)?;
        }
        Ok(())
    }
    /// 列出所有认证的注册表
    pub fn list_auth_configs(&self) -> Vec<(String, &AuthInfo)> {
        self.auth_configs
            .iter()
            .map(|(url, info)| (url.clone(), info))
            .collect()
    }
    /// 清除所有认证
    pub fn clear_all(&mut self) {
        self.auth_configs.clear();
    }
}
impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}