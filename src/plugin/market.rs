/// Plugin market module
use anyhow::Result;
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub downloads: u64,
    pub rating: f64,
    pub author: String,
}
pub struct PluginMarket {
    plugins: HashMap<String, PluginInfo>,
}
impl PluginMarket {
    pub fn new() -> Self {
        let mut plugins = HashMap::new();
        // Add some example plugins
        plugins.insert("typescript-transformer".to_string(), PluginInfo {
            name: "typescript-transformer".to_string(),
            version: "1.0.0".to_string(),
            description: "TypeScript code transformer".to_string(),
            downloads: 1000,
            rating: 4.5,
            author: "Amber Team".to_string(),
        });
        plugins.insert("css-minifier".to_string(), PluginInfo {
            name: "css-minifier".to_string(),
            version: "2.0.0".to_string(),
            description: "CSS code minifier".to_string(),
            downloads: 800,
            rating: 4.2,
            author: "Amber Team".to_string(),
        });
        Self { plugins }
    }
    /// Search plugins
    pub fn search(&self, query: &str) -> Vec<&PluginInfo> {
        self.plugins.values()
            .filter(|p| {
                p.name.contains(query) || 
                p.description.contains(query) || 
                p.author.contains(query)
            })
            .collect()
    }
    /// Get plugin info
    pub fn get_plugin(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }
    /// List all plugins
    pub fn list_all(&self) -> Vec<&PluginInfo> {
        self.plugins.values().collect()
    }
    /// Install plugin
    pub fn install(&self, name: &str) -> Result<String> {
        if let Some(plugin) = self.plugins.get(name) {
            println!("Installing plugin: {} v{}", plugin.name, plugin.version);
            Ok(format!("Installed {}", name))
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", name))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_plugin_market_creation() {
        let market: _ = PluginMarket::new();
        assert!(!market.list_all().is_empty());
    }
    #[test]
    fn test_search_plugins() {
        let market: _ = PluginMarket::new();
        let results: _ = market.search("typescript");
        assert!(!results.is_empty());
        assert!(results[0].name.contains("typescript"));
    }
    #[test]
    fn test_install_plugin() {
        let market: _ = PluginMarket::new();
        let result: _ = market.install("typescript-transformer").unwrap();
        assert!(result.contains("Installed"));
    }
}
use std::collections::{BTreeMap, HashMap};