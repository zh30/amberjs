// Custom Resource Definitions for Kubernetes
// Defines AmberCluster and AmberWorkload CRDs

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

mod amberjs_cluster;
mod amberjs_workload;
pub use amberjs_cluster::{
    Affinity, AmberCluster, AmberClusterSpec, DistributedConfig, MonitoringConfig, PodAffinity,
    PodAntiAffinity, PreferredSchedulingTerm, ResourceRequirements, SecurityConfig,
    SecurityContext, ServiceDiscoveryConfig, ServiceMonitorConfig, Toleration,
};
pub use amberjs_workload::{
    AmberWorkload, AmberWorkloadSpec, BufferConfig, CustomMetric, ExecutionConfig, ExecutionMode,
    HPAConfig, IOConfig, IngressBackend, IngressConfig, IngressHost, IngressPath, IngressTLS,
    InputSource, NetworkPolicyConfig, NetworkPolicyIPBlock, NetworkPolicyPeer, NetworkPolicyPort,
    NetworkPolicyRule, NetworkingConfig, OutputDestination, PersistenceConfig, RetryConfig,
    ScalePolicy, ServiceConfig, ServicePort,
};
/// Status phases for AmberCluster
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClusterPhase {
    /// Cluster is pending creation
    Pending,
    /// Cluster is being created
    Creating,
    /// Cluster is running normally
    Running,
    /// Cluster is being updated
    Updating,
    /// Cluster has failed
    Failed,
}
/// Status for AmberWorkload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkloadPhase {
    /// Workload is pending
    Pending,
    /// Workload is creating
    Creating,
    /// Workload is running
    Running,
    /// Workload is being updated
    Updating,
    /// Workload has failed
    Failed,
}
/// Condition types for cluster/workload status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConditionType {
    /// Available condition
    Available,
    /// Ready condition
    Ready,
    /// Failed condition
    Failed,
    /// Updating condition
    Updating,
}
/// Condition status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConditionStatus {
    /// Condition is true
    True,
    /// Condition is false
    False,
    /// Condition is unknown
    Unknown,
}
/// Common condition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Condition {
    /// Type of the condition
    pub condition_type: ConditionType,
    /// Status of the condition
    pub status: ConditionStatus,
    /// Last time we probed the condition
    pub last_probe_time: Option<String>,
    /// Last time the condition transitioned from one status to another
    pub last_transition_time: Option<String>,
    /// Unique, one-word, CamelCase reason for the condition's last transition
    pub reason: Option<String>,
    /// Human-readable message indicating details about the transition
    pub message: Option<String>,
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_cluster_phases() {
        assert_eq!(ClusterPhase::Pending.as_str(), "Pending");
        assert_eq!(ClusterPhase::Creating.as_str(), "Creating");
        assert_eq!(ClusterPhase::Running.as_str(), "Running");
        assert_eq!(ClusterPhase::Updating.as_str(), "Updating");
        assert_eq!(ClusterPhase::Failed.as_str(), "Failed");
    }
    #[test]
    fn test_workload_phases() {
        assert_eq!(WorkloadPhase::Pending.as_str(), "Pending");
        assert_eq!(WorkloadPhase::Creating.as_str(), "Creating");
        assert_eq!(WorkloadPhase::Running.as_str(), "Running");
        assert_eq!(WorkloadPhase::Updating.as_str(), "Updating");
        assert_eq!(WorkloadPhase::Failed.as_str(), "Failed");
    }
    #[test]
    fn test_condition_types() {
        assert_eq!(ConditionType::Available.as_str(), "Available");
        assert_eq!(ConditionType::Ready.as_str(), "Ready");
        assert_eq!(ConditionType::Failed.as_str(), "Failed");
        assert_eq!(ConditionType::Updating.as_str(), "Updating");
    }
    #[test]
    fn test_condition_status() {
        assert_eq!(ConditionStatus::True.as_str(), "True");
        assert_eq!(ConditionStatus::False.as_str(), "False");
        assert_eq!(ConditionStatus::Unknown.as_str(), "Unknown");
    }
    #[test]
    fn test_condition_structure() {
        let condition: _ = Condition {
            condition_type: ConditionType::Ready,
            status: ConditionStatus::True,
            last_probe_time: Some("2024-01-01T00:00:00Z".to_string()),
            last_transition_time: Some("2024-01-01T00:00:00Z".to_string()),
            reason: Some("KubeScheduler".to_string()),
            message: Some("Pod has been scheduled".to_string()),
        };
        assert_eq!(condition.condition_type, ConditionType::Ready);
        assert_eq!(condition.status, ConditionStatus::True);
        assert!(condition.reason.is_some());
        assert!(condition.message.is_some());
    }
}
// Implement helper methods for enums
impl ClusterPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClusterPhase::Pending => "Pending",
            ClusterPhase::Creating => "Creating",
            ClusterPhase::Running => "Running",
            ClusterPhase::Updating => "Updating",
            ClusterPhase::Failed => "Failed",
        }
    }
}
impl WorkloadPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkloadPhase::Pending => "Pending",
            WorkloadPhase::Creating => "Creating",
            WorkloadPhase::Running => "Running",
            WorkloadPhase::Updating => "Updating",
            WorkloadPhase::Failed => "Failed",
        }
    }
}
impl ConditionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionType::Available => "Available",
            ConditionType::Ready => "Ready",
            ConditionType::Failed => "Failed",
            ConditionType::Updating => "Updating",
        }
    }
}
impl ConditionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionStatus::True => "True",
            ConditionStatus::False => "False",
            ConditionStatus::Unknown => "Unknown",
        }
    }
}
