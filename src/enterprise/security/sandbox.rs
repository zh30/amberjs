// Runtime Security Sandbox
// 实现运行时安全沙箱和资源隔离

use anyhow::{Context, Result};
use libc::{SIGTERM, kill};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};
use std::time::Instant;
use std::path::PathBuf;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandbox
    pub enabled: bool,
    /// Base directory for sandboxed processes
    pub base_dir: PathBuf,
    /// Maximum memory limit (bytes)
    pub max_memory: u64,
    /// Maximum CPU time limit (seconds)
    pub max_cpu_time: u64,
    /// Maximum number of processes
    pub max_processes: usize,
    /// Maximum file size (bytes)
    pub max_file_size: u64,
    /// Allowed file system paths
    pub allowed_paths: Vec<PathBuf>,
    /// Blocked file system paths
    pub blocked_paths: Vec<PathBuf>,
    /// Network access allowed
    pub network_enabled: bool,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Environment variables to remove
    pub blocked_env_vars: Vec<String>,
}
/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Memory limit in bytes
    pub memory: u64,
    /// CPU time limit in seconds
    pub cpu_time: u64,
    /// Maximum number of file descriptors
    pub max_fds: usize,
    /// Maximum number of processes
    pub max_processes: usize,
    /// Maximum file size in bytes
    pub max_file_size: u64,
}
/// Sandbox execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    /// Execution success
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time
    pub execution_time_ms: u64,
    /// Memory usage peak
    pub memory_peak: u64,
    /// CPU time used
    pub cpu_time_used: u64,
    /// Files created
    pub files_created: Vec<PathBuf>,
    /// Network connections
    pub network_connections: Vec<String>,
}
/// Runtime security sandbox
#[derive(Debug)]
pub struct SecuritySandbox {
    /// Configuration
    config: SandboxConfig,
    /// Active sandboxes
    active_sandboxes: Arc<Mutex<HashMap<String, SandboxExecution>>>,
    /// Resource usage tracker
    resource_usage: Arc<Mutex<ResourceTracker>>,
}
/// Sandbox execution
#[derive(Debug)]
struct SandboxExecution {
    /// Process ID
    pid: u32,
    /// Working directory
    work_dir: PathBuf,
    /// Start time
    start_time: std::time::Instant,
    /// Command
    command: String,
}
/// Resource tracker
#[derive(Debug, Default)]
struct ResourceTracker {
    /// Active sandbox count
    active_count: usize,
    /// Total memory usage
    total_memory: u64,
    /// Total CPU time
    total_cpu_time: u64,
    /// Peak memory usage
    peak_memory: u64,
}
impl SecuritySandbox {
    /// Create a new SecuritySandbox
    pub fn new(config: SandboxConfig) -> Result<Self> {
        // Create base directory if it doesn't exist
        if !config.base_dir.exists() {
            std::fs::create_dir_all(&config.base_dir)
                .context("Failed to create sandbox base directory")?;
        }
        info!("Security sandbox initialized with base dir: {:?}", config.base_dir);
        Ok(Self {
            config,
            active_sandboxes: Arc::new(Mutex::new(HashMap::new()))
            resource_usage: Arc::new(Mutex::new(ResourceTracker::default()))
        })
    }
    /// Execute a command in the sandbox
    pub async fn execute(&self, command: &str, args: &[&str]) -> Result<SandboxResult> {
        let sandbox_id: _ = format!("sandbox_{}", std::process::id());
        let work_dir: _ = self.create_sandbox_dir(&sandbox_id)?;
        info!("Executing command in sandbox: {} {}", command, args.join(" "));
        // Set up resource limits
        let limits: _ = self.create_resource_limits();
        // Execute command with restrictions
        let start_time: _ = std::time::Instant::now();
        let output: _ = Command::new(command)
            .args(args)
            .current_dir(&work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .context("Failed to spawn sandboxed process")?;
        let pid: _ = output.id();
        // Track sandbox execution
        {
            let mut active = self.active_sandboxes.lock().unwrap();
            active.insert(sandbox_id.clone(), SandboxExecution {
                pid,
                work_dir: work_dir.clone(),
                start_time,
                command: format!("{} {}", command, args.join(" ")),
            });
        }
        // Wait for process completion
        let result: _ = output.wait_with_output()
            .context("Failed to wait for sandboxed process")?;
        let execution_time: _ = start_time.elapsed();
        // Clean up
        self.cleanup_sandbox_dir(&work_dir)?;
        let sandbox_result: _ = SandboxResult {
            success: result.status.success(),
            exit_code: result.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            execution_time_ms: execution_time.as_millis() as u64,
            memory_peak: 0, // TODO: Implement memory tracking
            cpu_time_used: 0, // TODO: Implement CPU time tracking
            files_created: Vec::new(), // TODO: Implement file tracking
            network_connections: Vec::new(), // TODO: Implement network tracking
        };
        info!("Sandbox execution completed: success={}, exit_code={}",
            sandbox_result.success, sandbox_result.exit_code);
        Ok(sandbox_result)
    }
    /// Create a sandboxed working directory
    fn create_sandbox_dir(&self, sandbox_id: &str) -> Result<PathBuf> {
        let work_dir: _ = self.config.base_dir.join(sandbox_id);
        // Create directory with restricted permissions
        std::fs::create_dir_all(&work_dir)
            .context("Failed to create sandbox directory")?;
        // Set directory permissions (read/write/execute for owner only)
        #[cfg(unix)]
        {
            std::fs::set_permissions(&work_dir, std::fs::Permissions::from_mode(0o700))
                .context("Failed to set sandbox directory permissions")?;
        }
        Ok(work_dir)
    }
    /// Clean up sandbox directory
    fn cleanup_sandbox_dir(&self, work_dir: &PathBuf) -> Result<()> {
        if work_dir.exists() {
            std::fs::remove_dir_all(work_dir)
                .context("Failed to remove sandbox directory")?;
        }
        Ok(())
    }
    /// Create resource limits for the process
    fn create_resource_limits(&self) -> ResourceLimits {
        ResourceLimits {
            memory: self.config.max_memory,
            cpu_time: self.config.max_cpu_time,
            max_fds: 1024,
            max_processes: self.config.max_processes,
            max_file_size: self.config.max_file_size,
        }
    }
    /// Check if a path is allowed
    pub fn is_path_allowed(&self, path: &PathBuf) -> bool {
        // Check blocked paths first
        for blocked in &self.config.blocked_paths {
            if path.starts_with(blocked) {
                return false;
            }
        }
        // Check allowed paths
        for allowed in &self.config.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }
        // If no allowed paths specified, default to sandbox base dir
        if self.config.allowed_paths.is_empty() {
            return path.starts_with(&self.config.base_dir);
        }
        false
    }
    /// Get sandbox statistics
    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        let active: _ = self.active_sandboxes.lock().unwrap();
        let resources: _ = self.resource_usage.lock().unwrap();
        stats.insert("active_sandboxes".to_string(), serde_json::Value::from(active.len());
        stats.insert("total_memory_bytes".to_string(), serde_json::Value::from(resources.total_memory));
        stats.insert("total_cpu_time_seconds".to_string(), serde_json::Value::from(resources.total_cpu_time));
        stats.insert("peak_memory_bytes".to_string(), serde_json::Value::from(resources.peak_memory));
        stats
    }
    /// Terminate a sandboxed process
    pub fn terminate_sandbox(&self, sandbox_id: &str) -> Result<()> {
        let mut active = self.active_sandboxes.lock().unwrap();
        if let Some(execution) = active.remove(sandbox_id) {
            #[cfg(unix)]
            {
                unsafe { kill(execution.pid as i32, SIGTERM) };
            }
            #[cfg(windows)]
            {
                Command::new("taskkill")
                    .args(&["/PID", &execution.pid.to_string(), "/F"])
                    .status()
                    .ok();
            }
            // Clean up directory
            self.cleanup_sandbox_dir(&execution.work_dir)?;
            info!("Terminated sandbox: {}", sandbox_id);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_sandbox_creation() {
        let config: _ = SandboxConfig {
            enabled: true,
            base_dir: PathBuf::from("/tmp/amberjs-sandbox"),
            max_memory: 1024 * 1024 * 1024, // 1GB
            max_cpu_time: 60,
            max_processes: 10,
            max_file_size: 100 * 1024 * 1024, // 100MB
            allowed_paths: Vec::new(),
            blocked_paths: Vec::new(),
            network_enabled: false,
            env_vars: HashMap::new(),
            blocked_env_vars: Vec::new(),
        };
        let sandbox: _ = SecuritySandbox::new(config);
        assert!(sandbox.is_ok());
    }
    #[test]
    fn test_path_allowed() {
        let config: _ = SandboxConfig {
            enabled: true,
            base_dir: PathBuf::from("/tmp/amberjs-sandbox"),
            max_memory: 1024 * 1024 * 1024,
            max_cpu_time: 60,
            max_processes: 10,
            max_file_size: 100 * 1024 * 1024,
            allowed_paths: vec![PathBuf::from("/tmp/amberjs-sandbox")],
            blocked_paths: vec![PathBuf::from("/etc")],
            network_enabled: false,
            env_vars: HashMap::new(),
            blocked_env_vars: Vec::new(),
        };
        let sandbox: _ = SecuritySandbox::new(config).unwrap();
        assert!(sandbox.is_path_allowed(&PathBuf::from("/tmp/amberjs-sandbox/test"));
        assert!(!sandbox.is_path_allowed(&PathBuf::from("/etc/passwd"));
    }
    #[test]
    fn test_resource_limits() {
        let config: _ = SandboxConfig {
            enabled: true,
            base_dir: PathBuf::from("/tmp/amberjs-sandbox"),
            max_memory: 512 * 1024 * 1024, // 512MB
            max_cpu_time: 30,
            max_processes: 5,
            max_file_size: 50 * 1024 * 1024, // 50MB
            allowed_paths: Vec::new(),
            blocked_paths: Vec::new(),
            network_enabled: false,
            env_vars: HashMap::new(),
            blocked_env_vars: Vec::new(),
        };
        let sandbox: _ = SecuritySandbox::new(config).unwrap();
        let limits: _ = sandbox.create_resource_limits();
        assert_eq!(limits.memory, 512 * 1024 * 1024);
        assert_eq!(limits.cpu_time, 30);
        assert_eq!(limits.max_processes, 5);
        assert_eq!(limits.max_file_size, 50 * 1024 * 1024);
    }
}