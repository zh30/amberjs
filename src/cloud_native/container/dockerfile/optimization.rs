// Dockerfile optimization strategies
// Provides various optimization techniques for container builds
/// Dockerfile optimizer
pub struct Optimizer {
    /// Optimization strategies
    strategies: Vec<Box<dyn OptimizationStrategy>>,
}
impl Optimizer {
    /// Create a new optimizer
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }
    /// Add optimization strategy
    pub fn add_strategy(mut self, strategy: Box<dyn OptimizationStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }
    /// Optimize a Dockerfile
    pub fn optimize(&self, dockerfile: &str) -> Result<String, Error> {
        let mut optimized = dockerfile.to_string();
        for strategy in &self.strategies {
            optimized = strategy.apply(&optimized)?;
        }
        Ok(optimized)
    }
    /// Analyze Dockerfile and suggest optimizations
    pub fn analyze(&self, dockerfile: &str) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        // Check for common optimization opportunities
        if self.has_multiple_copy_commands(dockerfile) {
            suggestions.push(OptimizationSuggestion {
                name: "combine-copy-commands".to_string(),
                description: "Combine multiple COPY commands to reduce layers".to_string(),
                impact: "medium".to_string(),
                example: Some("COPY . .\n".to_string()),
            });
        }
        if self.has_unoptimized_base_image(dockerfile) {
            suggestions.push(OptimizationSuggestion {
                name: "use-slim-image".to_string(),
                description: "Use a smaller base image to reduce image size".to_string(),
                impact: "high".to_string(),
                example: Some("FROM debian:bookworm-slim".to_string()),
            });
        }
        if self.has_missing_cache_optimization(dockerfile) {
            suggestions.push(OptimizationSuggestion {
                name: "optimize-layer-caching".to_string(),
                description: "Optimize COPY commands for better layer caching".to_string(),
                impact: "high".to_string(),
                example: Some(
                    "COPY Cargo.toml Cargo.lock ./\nRUN cargo fetch\nCOPY src ./src".to_string(),
                ),
            });
        }
        if self.has_unnecessary_packages(dockerfile) {
            suggestions.push(OptimizationSuggestion {
                name: "remove-unnecessary-packages".to_string(),
                description: "Remove unnecessary packages to reduce image size".to_string(),
                impact: "medium".to_string(),
                example: None,
            });
        }
        if self.has_missing_security_hardening(dockerfile) {
            suggestions.push(OptimizationSuggestion {
                name: "add-security-hardening".to_string(),
                description: "Add security hardening measures".to_string(),
                impact: "high".to_string(),
                example: Some(
                    "RUN addgroup -g 1000 amberjs && adduser -D -s /bin/sh -G amberjs amberjs"
                        .to_string(),
                ),
            });
        }
        suggestions
    }
    /// Check if Dockerfile has multiple COPY commands
    fn has_multiple_copy_commands(&self, dockerfile: &str) -> bool {
        let copy_count: _ = dockerfile.matches("COPY").count();
        copy_count > 3
    }
    /// Check if using non-slim base image
    fn has_unoptimized_base_image(&self, dockerfile: &str) -> bool {
        dockerfile.contains("FROM ubuntu") || dockerfile.contains("FROM debian:")
    }
    /// Check for missing cache optimization
    fn has_missing_cache_optimization(&self, dockerfile: &str) -> bool {
        !dockerfile.contains("cargo fetch") && dockerfile.contains("cargo build")
    }
    /// Check for unnecessary packages
    fn has_unnecessary_packages(&self, dockerfile: &str) -> bool {
        dockerfile.contains("apt-get install") && dockerfile.contains("curl wget vim")
    }
    /// Check for missing security hardening
    fn has_missing_security_hardening(&self, dockerfile: &str) -> bool {
        !dockerfile.contains("adduser") && !dockerfile.contains("USER")
    }
}
/// Optimization strategy trait
pub trait OptimizationStrategy {
    /// Get strategy name
    fn name(&self) -> &str;
    /// Apply optimization
    fn apply(&self, dockerfile: &str) -> Result<String, Error>;
}
/// Layer minimization strategy
pub struct LayerMinimizationStrategy;
impl OptimizationStrategy for LayerMinimizationStrategy {
    fn name(&self) -> &str {
        "layer-minimization"
    }
    fn apply(&self, dockerfile: &str) -> Result<String, Error> {
        // Combine RUN commands to reduce layers
        let lines: Vec<&str> = dockerfile.lines().collect();
        let mut optimized_lines = Vec::new();
        let mut current_run = String::new();
        let mut in_run = false;
        for line in lines {
            if line.trim_start().starts_with("RUN") {
                if in_run {
                    // Combine with previous RUN
                    current_run.push_str(" && ");
                    current_run.push_str(
                        line.trim_start()
                            .trim_start_matches("RUN ")
                            .trim_start_matches("apt-get update && apt-get install -y "),
                    );
                } else {
                    // Start new RUN
                    in_run = true;
                    current_run = line.to_string();
                }
            } else {
                // Flush current RUN if exists
                if in_run {
                    optimized_lines.push(current_run.clone());
                    current_run.clear();
                    in_run = false;
                }
                optimized_lines.push(line.to_string());
            }
        }
        // Flush final RUN if exists
        if in_run {
            optimized_lines.push(current_run.clone());
        }
        Ok(optimized_lines.join("\n"))
    }
}
/// Base image optimization strategy
pub struct BaseImageOptimizationStrategy {
    /// Target base image
    pub target_image: String,
    /// Use distroless
    pub use_distroless: bool,
}
impl OptimizationStrategy for BaseImageOptimizationStrategy {
    fn name(&self) -> &str {
        "base-image-optimization"
    }
    fn apply(&self, dockerfile: &str) -> Result<String, Error> {
        // Replace base image
        let result: _ = if self.use_distroless {
            dockerfile
                .replace(
                    "FROM debian:bookworm-slim",
                    "FROM gcr.io/distroless/base-debian12",
                )
                .replace("FROM ubuntu:", "FROM gcr.io/distroless/base")
        } else {
            dockerfile.replace("FROM ubuntu:", &format!("FROM {}:", self.target_image))
        };
        Ok(result)
    }
}
/// Cache optimization strategy
pub struct CacheOptimizationStrategy;
impl OptimizationStrategy for CacheOptimizationStrategy {
    fn name(&self) -> &str {
        "cache-optimization"
    }
    fn apply(&self, dockerfile: &str) -> Result<String, Error> {
        // Optimize COPY commands for better layer caching
        if dockerfile.contains("COPY . .") && !dockerfile.contains("COPY Cargo.toml") {
            // Insert dependency copy before source copy
            let lines: Vec<&str> = dockerfile.lines().collect();
            let mut optimized_lines = Vec::new();
            for line in &lines {
                optimized_lines.push(*line);
                if line.contains("WORKDIR /app")
                    && !lines.iter().any(|l| l.contains("COPY Cargo.toml"))
                {
                    optimized_lines.push("COPY Cargo.toml Cargo.lock ./\nRUN cargo fetch");
                }
            }
            let result: _ = optimized_lines.join("\n");
            Ok(result)
        } else {
            Ok(dockerfile.to_string())
        }
    }
}
/// Security hardening strategy
pub struct SecurityHardeningStrategy {
    /// Add non-root user
    pub add_non_root_user: bool,
    /// Read-only root filesystem
    pub read_only_root: bool,
    /// Drop capabilities
    pub drop_capabilities: bool,
}
impl OptimizationStrategy for SecurityHardeningStrategy {
    fn name(&self) -> &str {
        "security-hardening"
    }
    fn apply(&self, dockerfile: &str) -> Result<String, Error> {
        let mut additions = Vec::new();
        if self.add_non_root_user {
            additions.push(
                "RUN addgroup -g 1000 amberjs && adduser -D -s /bin/sh -G amberjs amberjs".to_string(),
            );
            additions.push("USER amberjs".to_string());
        }
        if self.read_only_root {
            additions.push("RUN chmod -R u-w,go-w /usr/local/bin/amber".to_string());
        }
        if self.drop_capabilities {
            additions.push(
                "RUN setcap cap_setpcap,cap_setuid,cap_setgid+ep /usr/local/bin/amber".to_string(),
            );
        }
        let mut result = dockerfile.to_string();
        if !additions.is_empty() {
            result.push_str("\n# Security Hardening\n");
            for addition in additions {
                result.push_str(&addition);
                result.push_str("\n");
            }
        }
        Ok(result)
    }
}
/// Size optimization strategy
pub struct SizeOptimizationStrategy {
    /// Strip binaries
    pub strip_binaries: bool,
    /// Remove unnecessary files
    pub remove_unnecessary_files: bool,
}
impl OptimizationStrategy for SizeOptimizationStrategy {
    fn name(&self) -> &str {
        "size-optimization"
    }
    fn apply(&self, dockerfile: &str) -> Result<String, Error> {
        let mut result = dockerfile.to_string();
        if self.strip_binaries && dockerfile.contains("cargo build --release") {
            // Add strip command to build
            result = result.replace(
                "cargo build --release",
                "cargo build --release && strip target/release/amber",
            );
        }
        if self.remove_unnecessary_files {
            // Add cleanup commands
            let cleanup: _ = r#"
RUN rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /root/.cargo/registry
"#;
            result.push_str(cleanup);
        }
        Ok(result)
    }
}
/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    /// Suggestion name
    pub name: String,
    /// Description
    pub description: String,
    /// Impact level (low, medium, high)
    pub impact: String,
    /// Example fix
    pub example: Option<String>,
}
impl OptimizationSuggestion {
    /// Get impact level
    pub fn impact_level(&self) -> ImpactLevel {
        match self.impact.as_str() {
            "high" => ImpactLevel::High,
            "medium" => ImpactLevel::Medium,
            "low" => ImpactLevel::Low,
            _ => ImpactLevel::Low,
        }
    }
}
/// Impact level
#[derive(Debug, Clone, PartialEq)]
pub enum ImpactLevel {
    High,
    Medium,
    Low,
}
/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
    #[error("Analysis error: {0}")]
    AnalysisFailed(String),
    #[error("Other error: {0}")]
    Other(String),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_layer_minimization_strategy() {
        let strategy: _ = LayerMinimizationStrategy;
        let dockerfile: _ = r#"RUN apt-get update
RUN apt-get install -y curl
RUN apt-get install -y wget
COPY . ."#;
        let optimized: _ = strategy.apply(dockerfile).unwrap();
        // Should combine RUN commands
        assert!(optimized
            .contains("apt-get update && apt-get install -y curl && apt-get install -y wget"));
    }
    #[test]
    fn test_base_image_optimization_strategy() {
        let strategy: _ = BaseImageOptimizationStrategy {
            target_image: "debian:bookworm-slim".to_string(),
            use_distroless: true,
        };
        let dockerfile: _ = "FROM ubuntu:latest\n";
        let optimized: _ = strategy.apply(dockerfile).unwrap();
        assert!(optimized.contains("distroless"));
    }
    #[test]
    fn test_cache_optimization_strategy() {
        let strategy: _ = CacheOptimizationStrategy;
        let dockerfile: _ = r#"WORKDIR /app
COPY . .
RUN cargo build --release"#;
        let optimized: _ = strategy.apply(dockerfile).unwrap();
        // Should add cargo fetch
        assert!(optimized.contains("cargo fetch"));
    }
    #[test]
    fn test_security_hardening_strategy() {
        let strategy: _ = SecurityHardeningStrategy {
            add_non_root_user: true,
            read_only_root: false,
            drop_capabilities: true,
        };
        let dockerfile: _ = "FROM debian:bookworm-slim\n";
        let optimized: _ = strategy.apply(dockerfile).unwrap();
        assert!(optimized.contains("adduser"));
        assert!(optimized.contains("USER"));
        assert!(optimized.contains("setcap"));
    }
    #[test]
    fn test_size_optimization_strategy() {
        let strategy: _ = SizeOptimizationStrategy {
            strip_binaries: true,
            remove_unnecessary_files: true,
        };
        let dockerfile: _ = "RUN cargo build --release\n";
        let optimized: _ = strategy.apply(dockerfile).unwrap();
        assert!(optimized.contains("strip"));
        assert!(optimized.contains("rm -rf"));
    }
    #[test]
    fn test_analyzer_suggestions() {
        let optimizer: _ = Optimizer::new();
        let dockerfile: _ = r#"FROM ubuntu:latest
RUN apt-get update && apt-get install -y curl wget vim
COPY . .
RUN cargo build --release"#;
        let suggestions: _ = optimizer.analyze(dockerfile);
        // Should suggest multiple optimizations
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.name == "use-slim-image"));
        assert!(suggestions
            .iter()
            .any(|s| s.name == "combine-copy-commands"));
    }
    #[test]
    fn test_impact_level() {
        let suggestion: _ = OptimizationSuggestion {
            name: "test".to_string(),
            description: "Test".to_string(),
            impact: "high".to_string(),
            example: None,
        };
        assert_eq!(suggestion.impact_level(), ImpactLevel::High);
    }
}
use std::collections::{BTreeMap, HashMap};
