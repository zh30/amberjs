// Plugin system core
// Supports both Rust and JavaScript plugins with sandboxing

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub permissions: Vec<String>,
    pub dependencies: Vec<String>,
}
/// Plugin lifecycle events
#[derive(Debug, Clone)]
pub enum PluginEvent {
    Init,
    BeforeBuild,
    AfterBuild,
    BeforeBundle,
    AfterBundle,
    Shutdown,
}
/// Plugin lifecycle state
#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    Loaded,
    Initialized,
    Active,
    Inactive,
    Error,
}
/// Plugin interface
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn state(&self) -> &PluginState;
    fn on_event(&self, event: &PluginEvent) -> Result<()>;
}
/// JavaScript plugin wrapper
pub struct JsPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    code: String,
    exports: HashMap<String, String>,
}
impl JsPlugin {
    pub fn new(metadata: PluginMetadata, code: String) -> Self {
        Self {
            metadata,
            state: PluginState::Loaded,
            code,
            exports: HashMap::new(),
        }
    }
}
impl Plugin for JsPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    fn state(&self) -> &PluginState {
        &self.state
    }
    fn on_event(&self, event: &PluginEvent) -> Result<()> {
        println!("JS Plugin '{}' received event: {:?}", self.metadata.name, event);
        Ok(())
    }
}
/// Rust plugin wrapper
pub struct RustPlugin {
    metadata: PluginMetadata,
    state: PluginState,
}
impl RustPlugin {
    pub fn new(metadata: PluginMetadata) -> Self {
        Self {
            metadata,
            state: PluginState::Loaded,
        }
    }
}
impl Plugin for RustPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    fn state(&self) -> &PluginState {
        &self.state
    }
    fn on_event(&self, event: &PluginEvent) -> Result<()> {
        println!("Rust Plugin '{}' received event: {:?}", self.metadata.name, event);
        Ok(())
    }
}
/// Plugin manager
pub struct PluginManager {
    plugins: Arc<Mutex<HashMap<String, Arc<dyn Plugin>>>>,
    event_history: Arc<Mutex<Vec<PluginEvent>>>,
    sandbox_enabled: bool,
}
impl PluginManager {
    /// Create new plugin manager
    pub fn new(sandbox_enabled: bool) -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            event_history: Arc::new(Mutex::new(Vec::new())),
            sandbox_enabled,
        }
    }
    /// Register Rust plugin
    pub fn register_rust_plugin(&self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name: _ = plugin.metadata().name.clone();
        let arc_plugin: Arc<dyn Plugin> = plugin.into();
        {
            let mut plugins = self.plugins.lock().unwrap();
            plugins.insert(name.clone(), arc_plugin);
        }
        println!("Registered Rust plugin: {}", name);
        Ok(())
    }
    /// Register JavaScript plugin
    pub fn register_js_plugin(&self, plugin: JsPlugin) -> Result<()> {
        let name: _ = plugin.metadata().name.clone();
        let arc_plugin: Arc<dyn Plugin> = Arc::new(Mutex::new(plugin));
        {
            let mut plugins = self.plugins.lock().unwrap();
            plugins.insert(name.clone(), arc_plugin);
        }
        println!("Registered JS plugin: {}", name);
        Ok(())
    }
    /// Load plugin from file
    pub fn load_plugin_from_file(&self, path: &str) -> Result<()> {
        let code: _ = std::fs::read_to_string(path)?;
        // Extract plugin metadata from code comments
        let metadata: _ = self.extract_metadata_from_code(&code)?;
        let plugin: _ = JsPlugin::new(metadata, code);
        self.register_js_plugin(plugin)?;
        Ok(())
    }
    /// Extract metadata from code comments
    fn extract_metadata_from_code(&self, code: &str) -> Result<PluginMetadata> {
        let mut name = "unknown".to_string();
        let mut version = "1.0.0".to_string();
        let mut description = "No description".to_string();
        let mut author = "unknown".to_string();
        for line in code.lines() {
            let line: _ = line.trim();
            // Extract metadata from @amberjs-meta comments
            if line.starts_with("// @amberjs-meta") {
                if let Some(pos) = line.find(":") {
                    let key: _ = line[14..pos].trim();
                    let value: _ = line[pos + 1..].trim().trim_matches('"');
                    match key {
                        "name" => name = value.to_string(),
                        "version" => version = value.to_string(),
                        "description" => description = value.to_string(),
                        "author" => author = value.to_string(),
                        _ => {}
                    }
                }
            }
        }
        Ok(PluginMetadata {
            name,
            version,
            description,
            author,
            homepage: None,
            permissions: Vec::new(),
            dependencies: Vec::new(),
        })
    }
    /// Unregister plugin
    pub fn unregister_plugin(&self, name: &str) -> Result<()> {
        {
            let mut plugins = self.plugins.lock().unwrap();
            plugins.remove(name);
        }
        println!("Unregistered plugin: {}", name);
        Ok(())
    }
    /// Get plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let plugins: _ = self.plugins.lock().unwrap();
        plugins.get(name).cloned()
    }
    /// List all plugins
    pub fn list_plugins(&self) -> Vec<String> {
        let plugins: _ = self.plugins.lock().unwrap();
        plugins.keys().cloned().collect()
    }
    /// Emit event to all plugins
    pub fn emit_event(&self, event: PluginEvent) -> Result<()> {
        // Record event
        {
            let mut history = self.event_history.lock().unwrap();
            history.push(event.clone());
        }
        // Send to all plugins
        let plugins: _ = self.plugins.lock().unwrap();
        for (_name, plugin) in plugins.iter() {
            if let Err(e) = plugin.on_event(&event) {
                eprintln!("Plugin error: {:?}", e);
            }
        }
        Ok(())
    }
    /// Get event history
    pub fn get_event_history(&self) -> Vec<PluginEvent> {
        self.event_history.lock().unwrap().clone()
    }
    /// Check if plugin exists
    pub fn has_plugin(&self, name: &str) -> bool {
        let plugins: _ = self.plugins.lock().unwrap();
        plugins.contains_key(name)
    }
    /// Enable sandbox for all plugins
    pub fn enable_sandbox(&mut self) {
        self.sandbox_enabled = true;
        println!("Plugin sandbox enabled");
    }
    /// Disable sandbox for all plugins
    pub fn disable_sandbox(&mut self) {
        self.sandbox_enabled = false;
        println!("Plugin sandbox disabled");
    }
}
#[cfg(test)]
mod tests {
    struct TestRustPlugin {
        metadata: PluginMetadata,
    }
    impl TestRustPlugin {
        fn new() -> Self {
            Self {
                metadata: PluginMetadata {
                    name: "test-rust-plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Test Rust plugin".to_string(),
                    author: "Test Author".to_string(),
                    homepage: None,
                    permissions: vec!["read".to_string()],
                    dependencies: Vec::new(),
                }
            }
        }
    }
    impl Plugin for TestRustPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }
        fn state(&self) -> &PluginState {
            &PluginState::Active
        }
        fn on_event(&self, event: &PluginEvent) -> Result<()> {
            println!("TestRustPlugin received: {:?}", event);
            Ok(())
        }
    }
    #[test]
    fn test_plugin_manager_creation() {
        let manager: _ = PluginManager::new(true);
        assert!(manager.sandbox_enabled);
    }
    #[test]
    fn test_register_rust_plugin() {
        let manager: _ = PluginManager::new(false);
        let plugin: _ = Box::new(TestRustPlugin::new());
        assert!(manager.register_rust_plugin(plugin).is_ok());
    }
    #[test]
    fn test_register_js_plugin() {
        let manager: _ = PluginManager::new(false);
        let metadata: _ = PluginMetadata {
            name: "test-js-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test JS plugin".to_string(),
            author: "Test Author".to_string(),
            homepage: None,
            permissions: Vec::new(),
            dependencies: Vec::new(),
        };
        let plugin: _ = JsPlugin::new(metadata, "console.log('test');".to_string());
        assert!(manager.register_js_plugin(plugin).is_ok());
    }
    #[test]
    fn test_list_plugins() {
        let manager: _ = PluginManager::new(false);
        let plugin: _ = Box::new(TestRustPlugin::new());
        manager.register_rust_plugin(plugin).unwrap();
        let plugins: _ = manager.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert!(plugins.contains(&"test-rust-plugin".to_string()));
    }
    #[test]
    fn test_emit_event() {
        let manager: _ = PluginManager::new(false);
        let plugin: _ = Box::new(TestRustPlugin::new());
        manager.register_rust_plugin(plugin).unwrap();
        assert!(manager.emit_event(PluginEvent::Init).is_ok());
        let history: _ = manager.get_event_history();
        assert_eq!(history.len(), 1);
        assert!(matches!(history[0], PluginEvent::Init));
    }
    #[test]
    fn test_extract_metadata_from_code() {
        let manager: _ = PluginManager::new(false);
        let code: _ = r#"
            // @amberjs-meta: name: "test-plugin"
            // @amberjs-meta: version: "2.0.0"
            // @amberjs-meta: description: "A test plugin"
            // @amberjs-meta: author: "Test User"
            console.log('plugin code');
        "#;
        let metadata: _ = manager.extract_metadata_from_code(code).unwrap();
        assert_eq!(metadata.name, "test-plugin");
        assert_eq!(metadata.version, "2.0.0");
        assert_eq!(metadata.description, "A test plugin");
        assert_eq!(metadata.author, "Test User");
    }
}