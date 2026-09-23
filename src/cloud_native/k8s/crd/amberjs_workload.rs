// AmberWorkload Custom Resource Definition
/// Defines workload-level configuration for running JavaScript/TypeScript scripts
use kube::CustomResource;
use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::Path;
/// AmberWorkload is the schema for the amberjsworkload API
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "cloudnative.amberjs.io",
    version = "v1",
    kind = "AmberWorkload",
    plural = "amberjsworkloads",
    shortname = "bjw",
    namespaced
)]
#[cfg_attr(test, derive(Default))]
pub struct AmberWorkloadSpec {
    /// Reference to the AmberCluster
    pub cluster_ref: String,
    /// Path to the JavaScript/TypeScript script
    pub script_path: String,
    /// Arguments to pass to the script
    pub script_args: Vec<String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Number of replicas
    pub replicas: u32,
    /// Resource requirements
    pub resources: super::amberjs_cluster::ResourceRequirements,
    /// HPA configuration for auto-scaling
    pub hpa_config: HPAConfig,
    /// Configuration for script execution
    pub execution: Option<ExecutionConfig>,
    /// Configuration for inputs and outputs
    pub io_config: Option<IOConfig>,
    /// Configuration for persistence
    pub persistence: Option<PersistenceConfig>,
    /// Configuration for networking
    pub networking: Option<NetworkingConfig>,
}
/// HPA (Horizontal Pod Autoscaler) configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct HPAConfig {
    /// Enable HPA
    pub enabled: bool,
    /// Minimum number of replicas
    pub min_replicas: u32,
    /// Maximum number of replicas
    pub max_replicas: u32,
    /// Target CPU utilization percentage
    pub target_cpu_percent: f64,
    /// Target memory utilization percentage
    pub target_memory_percent: f64,
    /// Custom metrics for scaling
    pub custom_metrics: Option<Vec<CustomMetric>>,
    /// Scale up/down stabilization window
    pub stabilization_window_seconds: Option<u64>,
    /// Scale policy
    pub scale_policy: Option<ScalePolicy>,
}
/// Custom metric for auto-scaling
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct CustomMetric {
    /// Metric name
    pub name: String,
    /// Metric type (Pod, Object, Resource)
    pub metric_type: String,
    /// Target value
    pub target_value: String,
    /// Metric selector
    pub metric_selector: Option<HashMap<String, String>>,
}
/// Scale policy
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ScalePolicy {
    /// Policy type (Percent, Pods)
    pub policy_type: String,
    /// Value (percentage or absolute number)
    pub value: u32,
    /// Period seconds
    pub period_seconds: u64,
}
/// Configuration for script execution
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ExecutionConfig {
    /// Execution mode (standalone, distributed, batch)
    pub mode: ExecutionMode,
    /// Script timeout
    pub timeout_seconds: Option<u64>,
    /// Retry configuration
    pub retry: Option<RetryConfig>,
    /// Concurrency limit
    pub concurrency_limit: Option<u32>,
    /// Memory limit per execution
    pub memory_limit_mb: Option<u32>,
}
/// Execution mode
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[cfg_attr(test, derive(Default))]
pub enum ExecutionMode {
    /// Standalone execution (single run)
    #[default]
    Standalone = 0,
    /// Distributed execution (across cluster)
    Distributed,
    /// Batch processing
    Batch,
    /// Streaming execution
    Streaming,
}
/// Retry configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff_seconds: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum backoff duration
    pub max_backoff_seconds: u64,
}
/// Configuration for inputs and outputs
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct IOConfig {
    /// Input sources
    pub inputs: Option<Vec<InputSource>>,
    /// Output destinations
    pub outputs: Option<Vec<OutputDestination>>,
    /// Buffer configuration
    pub buffer: Option<BufferConfig>,
}
/// Input source configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct InputSource {
    /// Source type (file, http, kafka, s3, etc.)
    pub source_type: String,
    /// Source configuration
    pub config: HashMap<String, String>,
    /// Format (json, csv, parquet, etc.)
    pub format: Option<String>,
    /// Schema (for structured data)
    pub schema: Option<String>,
}
/// Output destination configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct OutputDestination {
    /// Destination type (file, http, kafka, s3, etc.)
    pub dest_type: String,
    /// Destination configuration
    pub config: HashMap<String, String>,
    /// Format (json, csv, parquet, etc.)
    pub format: Option<String>,
    /// Compression (gzip, bzip2, etc.)
    pub compression: Option<String>,
}
/// Buffer configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct BufferConfig {
    /// Buffer size in bytes
    pub size_bytes: Option<u64>,
    /// Flush interval in seconds
    pub flush_interval_seconds: Option<u64>,
    /// Maximum items in buffer
    pub max_items: Option<u32>,
}
/// Persistence configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct PersistenceConfig {
    /// Enable persistence
    pub enabled: bool,
    /// Storage class
    pub storage_class: Option<String>,
    /// Access mode (ReadWriteOnce, ReadOnlyMany, ReadWriteMany)
    pub access_mode: Option<String>,
    /// Size of the volume
    pub size: Option<String>,
    /// Volume mount path
    pub mount_path: Option<String>,
}
/// Networking configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NetworkingConfig {
    /// Service configuration
    pub service: Option<ServiceConfig>,
    /// Ingress configuration
    pub ingress: Option<IngressConfig>,
    /// Network policies
    pub network_policies: Option<Vec<NetworkPolicyConfig>>,
}
/// Service configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ServiceConfig {
    /// Enable service
    pub enabled: bool,
    /// Service type (ClusterIP, NodePort, LoadBalancer)
    pub service_type: String,
    /// Port configuration
    pub ports: Option<Vec<ServicePort>>,
    /// Session affinity
    pub session_affinity: Option<String>,
}
/// Service port configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ServicePort {
    /// Port name
    pub name: Option<String>,
    /// Port number
    pub port: u32,
    /// Target port
    pub target_port: Option<u32>,
    /// Protocol (TCP, UDP)
    pub protocol: Option<String>,
}
/// Ingress configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct IngressConfig {
    /// Enable ingress
    pub enabled: bool,
    /// Ingress class
    pub ingress_class: Option<String>,
    /// Annotations
    pub annotations: Option<HashMap<String, String>>,
    /// Hosts
    pub hosts: Option<Vec<IngressHost>>,
    /// TLS configuration
    pub tls: Option<Vec<IngressTLS>>,
}
/// Ingress host configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct IngressHost {
    /// Hostname
    pub host: String,
    /// Paths
    pub paths: Vec<IngressPath>,
}
/// Ingress path configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct IngressPath {
    /// Path
    pub path: String,
    /// Path type (Exact, Prefix, ImplementationSpecific)
    pub path_type: String,
    /// Backend service
    pub backend: IngressBackend,
}
/// Ingress backend
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct IngressBackend {
    /// Service name
    pub service_name: String,
    /// Service port
    pub service_port: u32,
}
/// Ingress TLS configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct IngressTLS {
    /// Hosts
    pub hosts: Option<Vec<String>>,
    /// Secret name
    pub secret_name: Option<String>,
}
/// Network policy configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NetworkPolicyConfig {
    /// Policy name
    pub name: String,
    /// Policy types (Ingress, Egress)
    pub policy_types: Vec<String>,
    /// Ingress rules
    pub ingress: Option<Vec<NetworkPolicyRule>>,
    /// Egress rules
    pub egress: Option<Vec<NetworkPolicyRule>>,
}
/// Network policy rule
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NetworkPolicyRule {
    /// From (sources)
    pub from: Option<Vec<NetworkPolicyPeer>>,
    /// To (destinations)
    pub to: Option<Vec<NetworkPolicyPeer>>,
    /// Ports
    pub ports: Option<Vec<NetworkPolicyPort>>,
}
/// Network policy peer
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NetworkPolicyPeer {
    /// Namespace selector
    pub namespace_selector: Option<super::amberjs_cluster::LabelSelector>,
    /// Pod selector
    pub pod_selector: Option<super::amberjs_cluster::LabelSelector>,
    /// IP block
    pub ip_block: Option<NetworkPolicyIPBlock>,
}
/// Network policy IP block
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NetworkPolicyIPBlock {
    /// CIDR
    pub cidr: String,
    /// Except
    pub except: Option<Vec<String>>,
}
/// Network policy port
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NetworkPolicyPort {
    /// Port
    pub port: Option<u32>,
    /// Protocol
    pub protocol: Option<String>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_amberjs_workload_crd_creation() {
        let workload: _ = AmberWorkload::new(
            "my-workload",
            AmberWorkloadSpec {
                cluster_ref: "my-cluster".to_string(),
                script_path: "/app/main.js".to_string(),
                script_args: vec!["--mode=production".to_string()],
                environment: HashMap::from([
                    ("NODE_ENV".to_string(), "production".to_string()),
                    ("AMBER_MODE".to_string(), "distributed".to_string()),
                ]),
                replicas: 5,
                resources: super::super::amberjs_cluster::ResourceRequirements {
                    cpu: "1".to_string(),
                    memory: "2Gi".to_string(),
                    disk: "10Gi".to_string(),
                    gpu: None,
                },
                hpa_config: HPAConfig {
                    enabled: true,
                    min_replicas: 2,
                    max_replicas: 20,
                    target_cpu_percent: 70.0,
                    target_memory_percent: 80.0,
                    custom_metrics: None,
                    stabilization_window_seconds: None,
                    scale_policy: None,
                },
                execution: None,
                io_config: None,
                persistence: None,
                networking: None,
            },
        );
        assert_eq!(workload.spec.cluster_ref, "my-cluster");
        assert_eq!(workload.spec.script_path, "/app/main.js");
        assert_eq!(workload.spec.replicas, 5);
        assert!(workload.spec.hpa_config.enabled);
        assert_eq!(workload.spec.hpa_config.min_replicas, 2);
        assert_eq!(workload.spec.hpa_config.max_replicas, 20);
    }
    #[test]
    fn test_hpa_config() {
        let hpa: _ = HPAConfig {
            enabled: true,
            min_replicas: 3,
            max_replicas: 50,
            target_cpu_percent: 60.0,
            target_memory_percent: 75.0,
            custom_metrics: Some(vec![CustomMetric {
                name: "requests_per_second".to_string(),
                metric_type: "Pod".to_string(),
                target_value: "100".to_string(),
                metric_selector: None,
            }]),
            stabilization_window_seconds: Some(300),
            scale_policy: Some(ScalePolicy {
                policy_type: "Percent".to_string(),
                value: 10,
                period_seconds: 60,
            }),
        };
        assert!(hpa.enabled);
        assert_eq!(hpa.min_replicas, 3);
        assert_eq!(hpa.max_replicas, 50);
        assert_eq!(hpa.target_cpu_percent, 60.0);
        assert!(hpa.custom_metrics.is_some());
        assert_eq!(hpa.custom_metrics.as_ref().unwrap().len(), 1);
    }
    #[test]
    fn test_execution_config() {
        let execution: _ = ExecutionConfig {
            mode: ExecutionMode::Distributed,
            timeout_seconds: Some(300),
            retry: Some(RetryConfig {
                max_attempts: 3,
                initial_backoff_seconds: 5,
                backoff_multiplier: 2.0,
                max_backoff_seconds: 60,
            }),
            concurrency_limit: Some(100),
            memory_limit_mb: Some(512),
        };
        matches!(execution.mode, ExecutionMode::Distributed);
        assert_eq!(execution.timeout_seconds, Some(300));
        assert_eq!(execution.retry.as_ref().unwrap().max_attempts, 3);
        assert_eq!(execution.concurrency_limit, Some(100));
    }
    #[test]
    fn test_io_config() {
        let io_config: _ = IOConfig {
            inputs: Some(vec![InputSource {
                source_type: "kafka".to_string(),
                config: HashMap::from([
                    ("brokers".to_string(), "kafka:9092".to_string()),
                    ("topic".to_string(), "input-events".to_string()),
                ]),
                format: Some("json".to_string()),
                schema: None,
            }]),
            outputs: Some(vec![OutputDestination {
                dest_type: "s3".to_string(),
                config: HashMap::from([
                    ("bucket".to_string(), "output-bucket".to_string()),
                    ("region".to_string(), "us-west-2".to_string()),
                ]),
                format: Some("parquet".to_string()),
                compression: Some("gzip".to_string()),
            }]),
            buffer: Some(BufferConfig {
                size_bytes: Some(1024 * 1024), // 1MB
                flush_interval_seconds: Some(10),
                max_items: Some(1000),
            }),
        };
        assert_eq!(io_config.inputs.as_ref().unwrap().len(), 1);
        assert_eq!(io_config.outputs.as_ref().unwrap().len(), 1);
        assert_eq!(io_config.inputs.as_ref().unwrap()[0].source_type, "kafka");
        assert_eq!(io_config.outputs.as_ref().unwrap()[0].dest_type, "s3");
        assert_eq!(
            io_config.buffer.as_ref().unwrap().size_bytes,
            Some(1024 * 1024)
        );
    }
    #[test]
    fn test_networking_config() {
        let networking: _ = NetworkingConfig {
            service: Some(ServiceConfig {
                enabled: true,
                service_type: "ClusterIP".to_string(),
                ports: Some(vec![ServicePort {
                    name: Some("http".to_string()),
                    port: 8080,
                    target_port: Some(8080),
                    protocol: Some("TCP".to_string()),
                }]),
                session_affinity: Some("ClientIP".to_string()),
            }),
            ingress: Some(IngressConfig {
                enabled: true,
                ingress_class: Some("nginx".to_string()),
                annotations: Some(HashMap::from([(
                    "cert-manager.io/cluster-issuer".to_string(),
                    "letsencrypt".to_string(),
                )])),
                hosts: Some(vec![IngressHost {
                    host: "api.example.com".to_string(),
                    paths: vec![IngressPath {
                        path: "/".to_string(),
                        path_type: "Prefix".to_string(),
                        backend: IngressBackend {
                            service_name: "my-workload".to_string(),
                            service_port: 8080,
                        },
                    }],
                }]),
                tls: Some(vec![IngressTLS {
                    hosts: Some(vec!["api.example.com".to_string()]),
                    secret_name: Some("api-tls".to_string()),
                }]),
            }),
            network_policies: None,
        };
        assert!(networking.service.as_ref().unwrap().enabled);
        assert_eq!(
            networking.service.as_ref().unwrap().service_type,
            "ClusterIP"
        );
        assert!(networking.ingress.as_ref().unwrap().enabled);
        assert_eq!(
            networking
                .ingress
                .as_ref()
                .unwrap()
                .hosts
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            networking.ingress.as_ref().unwrap().hosts.as_ref().unwrap()[0].host,
            "api.example.com"
        );
    }
}
use std::collections::{BTreeMap, HashMap};
