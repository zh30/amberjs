// AmberCluster Custom Resource Definition
/// Defines the cluster-level configuration for Amber runtime
use kube::CustomResource;
use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::Path;
/// AmberCluster is the schema for the amberjscluster API
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "cloudnative.amberjs.io",
    version = "v1",
    kind = "AmberCluster",
    plural = "amberjsclusters",
    shortname = "bjc",
    namespaced
)]
#[cfg_attr(test, derive(Default))]
pub struct AmberClusterSpec {
    /// Version of Amber runtime to deploy
    pub version: String,
    /// Number of nodes in the cluster
    pub nodes: u32,
    /// Container image for Amber runtime
    pub image: String,
    /// Resource requirements for each node
    pub resources: ResourceRequirements,
    /// Security configuration
    pub security: SecurityConfig,
    /// Distributed runtime configuration
    pub distributed: DistributedConfig,
    /// Node selector for scheduling
    pub node_selector: Option<HashMap<String, String>>,
    /// Tolerations for node taints
    pub tolerations: Option<Vec<Toleration>>,
    /// Affinity rules for pod scheduling
    pub affinity: Option<Affinity>,
    /// Monitoring and observability settings
    pub monitoring: Option<MonitoringConfig>,
}
/// Resource requirements for cluster nodes
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ResourceRequirements {
    /// CPU resource request (e.g., "2", "2000m")
    pub cpu: String,
    /// Memory resource request (e.g., "4Gi", "4096Mi")
    pub memory: String,
    /// Disk storage request (e.g., "20Gi")
    pub disk: String,
    /// GPU resources (optional)
    pub gpu: Option<String>,
}
/// Security configuration for the cluster
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct SecurityConfig {
    /// Enable sandbox isolation
    pub sandbox_enabled: bool,
    /// Enable RBAC
    pub rbac_enabled: bool,
    /// Enable encryption
    pub encryption_enabled: bool,
    /// Security context configuration
    pub security_context: Option<SecurityContext>,
}
/// Security context for pods
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct SecurityContext {
    /// Run as non-root user
    pub run_as_non_root: bool,
    /// Run as user ID
    pub run_as_user: Option<u32>,
    /// Run as group ID
    pub run_as_group: Option<u32>,
    /// Filesystem read-only
    pub read_only_root_filesystem: bool,
    /// Allow privilege escalation
    pub allow_privilege_escalation: bool,
}
/// Distributed runtime configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct DistributedConfig {
    /// Cluster name for service discovery
    pub cluster_name: String,
    /// Enable service discovery
    pub service_discovery: bool,
    /// Enable load balancer
    pub load_balancer: bool,
    /// Enable auto-scaling
    pub auto_scaling: bool,
    /// Configuration for service discovery
    pub discovery_config: Option<ServiceDiscoveryConfig>,
}
/// Service discovery configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ServiceDiscoveryConfig {
    /// Gossip protocol interval
    pub gossip_interval_ms: Option<u64>,
    /// Node timeout
    pub node_timeout_sec: Option<u64>,
    /// Discovery backend (gossip, etcd, consul)
    pub backend: Option<String>,
}
/// Node toleration for taints
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct Toleration {
    /// Taint key
    pub key: String,
    /// Taint operator
    pub operator: Option<String>,
    /// Taint value
    pub value: Option<String>,
    /// Effect (NoSchedule, PreferNoSchedule, NoExecute)
    pub effect: Option<String>,
    /// Toleration seconds
    pub toleration_seconds: Option<i64>,
}
/// Affinity rules for pod scheduling
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct Affinity {
    /// Pod affinity rules
    pub pod_affinity: Option<PodAffinity>,
    /// Pod anti-affinity rules
    pub pod_anti_affinity: Option<PodAntiAffinity>,
    /// Node affinity rules
    pub node_affinity: Option<NodeAffinity>,
}
/// Pod affinity configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct PodAffinity {
    /// Required during scheduling (must be met)
    pub required_during_scheduling: Option<Vec<PodAffinityTerm>>,
    /// Preferred during scheduling (weight matters)
    pub preferred_during_scheduling: Option<Vec<WeightedPodAffinityTerm>>,
}
/// Pod affinity term
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct PodAffinityTerm {
    /// Label selector
    pub label_selector: Option<LabelSelector>,
    /// Namespace selector
    pub namespace_selector: Option<LabelSelector>,
    /// Topology key
    pub topology_key: String,
}
/// Weighted pod affinity term
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct WeightedPodAffinityTerm {
    /// Weight (1-100)
    pub weight: i32,
    /// Pod affinity term
    pub pod_affinity_term: PodAffinityTerm,
}
/// Label selector
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct LabelSelector {
    /// Match labels
    pub match_labels: Option<HashMap<String, String>>,
    /// Match expressions
    pub match_expressions: Option<Vec<LabelSelectorRequirement>>,
}
/// Label selector requirement
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct LabelSelectorRequirement {
    /// Key
    pub key: String,
    /// Operator (In, NotIn, Exists, DoesNotExist, Gt, Lt)
    pub operator: String,
    /// Values
    pub values: Option<Vec<String>>,
}
/// Pod anti-affinity (same as PodAffinity)
pub type PodAntiAffinity = PodAffinity;
/// Node affinity configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NodeAffinity {
    /// Required during scheduling
    pub required_during_scheduling: Option<NodeSelector>,
    /// Preferred during scheduling
    pub preferred_during_scheduling: Option<Vec<PreferredSchedulingTerm>>,
}
/// Node selector
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NodeSelector {
    /// Node selector terms
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}
/// Node selector term
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NodeSelectorTerm {
    /// Match expressions
    pub match_expressions: Option<Vec<NodeSelectorRequirement>>,
}
/// Node selector requirement
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct NodeSelectorRequirement {
    /// Key
    pub key: String,
    /// Operator (In, NotIn, Exists, DoesNotExist, Gt, Lt)
    pub operator: String,
    /// Values
    pub values: Option<Vec<String>>,
}
/// Preferred scheduling term
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct PreferredSchedulingTerm {
    /// Weight (1-100)
    pub weight: i32,
    /// Preference
    pub preference: NodeSelectorTerm,
}
/// Monitoring and observability configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct MonitoringConfig {
    /// Enable Prometheus monitoring
    pub prometheus_enabled: bool,
    /// Enable Grafana dashboards
    pub grafana_enabled: bool,
    /// Enable distributed tracing
    pub tracing_enabled: bool,
    /// ServiceMonitor configuration
    pub service_monitor: Option<ServiceMonitorConfig>,
}
/// ServiceMonitor configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[cfg_attr(test, derive(Default))]
pub struct ServiceMonitorConfig {
    /// Interval for scraping metrics
    pub interval: Option<String>,
    /// Path to metrics endpoint
    pub path: Option<String>,
    /// Port name
    pub port: Option<String>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_amberjs_cluster_crd_creation() {
        let cluster: _ = AmberCluster::new(
            "test-cluster",
            AmberClusterSpec {
                version: "v1.0.0".to_string(),
                nodes: 3,
                image: "amberjs:latest".to_string(),
                resources: ResourceRequirements {
                    cpu: "2".to_string(),
                    memory: "4Gi".to_string(),
                    disk: "20Gi".to_string(),
                    gpu: None,
                },
                security: SecurityConfig {
                    sandbox_enabled: true,
                    rbac_enabled: true,
                    encryption_enabled: true,
                    security_context: None,
                },
                distributed: DistributedConfig {
                    cluster_name: "test-cluster".to_string(),
                    service_discovery: true,
                    load_balancer: true,
                    auto_scaling: true,
                    discovery_config: None,
                },
                node_selector: None,
                tolerations: None,
                affinity: None,
                monitoring: None,
            },
        );
        assert_eq!(cluster.spec.version, "v1.0.0");
        assert_eq!(cluster.spec.nodes, 3);
        assert!(cluster.spec.security.sandbox_enabled);
        assert!(cluster.spec.distributed.service_discovery);
    }
    #[test]
    fn test_resource_requirements() {
        let resources: _ = ResourceRequirements {
            cpu: "4".to_string(),
            memory: "8Gi".to_string(),
            disk: "100Gi".to_string(),
            gpu: Some("nvidia-tesla-v100".to_string()),
        };
        assert_eq!(resources.cpu, "4");
        assert_eq!(resources.memory, "8Gi");
        assert_eq!(resources.disk, "100Gi");
        assert_eq!(resources.gpu, Some("nvidia-tesla-v100".to_string()));
    }
    #[test]
    fn test_security_context() {
        let security_context: _ = SecurityContext {
            run_as_non_root: true,
            run_as_user: Some(1000),
            run_as_group: Some(1000),
            read_only_root_filesystem: true,
            allow_privilege_escalation: false,
        };
        assert!(security_context.run_as_non_root);
        assert_eq!(security_context.run_as_user, Some(1000));
        assert!(security_context.read_only_root_filesystem);
        assert!(!security_context.allow_privilege_escalation);
    }
}
use std::collections::{BTreeMap, HashMap};
