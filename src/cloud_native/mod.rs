// Cloud Native Integration Module
// Provides Kubernetes, containerization, Service Mesh, and CI/CD features
pub mod k8s;
/// Re-export Kubernetes module
pub use k8s::{
    AmberCluster, AmberClusterSpec, AmberWorkload, AmberWorkloadSpec, ClusterPhase, Condition,
    ConditionStatus, ConditionType, DistributedConfig, HPAConfig, MonitoringConfig,
    NetworkPolicyConfig, PodAffinity, PodAntiAffinity, PreferredSchedulingTerm,
    ResourceRequirements, RetryConfig, SecurityConfig, SecurityContext, ServiceDiscoveryConfig,
    ServiceMonitorConfig, Toleration, WorkloadPhase,
};
/// Container module for Docker builds and security
pub mod container;
/// Re-export container types
pub use container::{
    BaseImageOptimizationStrategy, BuilderStage, CacheOptimizationStrategy, ComplianceIssue,
    ComplianceSeverity, ContainerImage, DockerfileError, ImageLayer, ImpactLevel,
    LayerMinimizationStrategy, MultiStageBuilder, Optimization, OptimizationStrategy,
    OptimizationSuggestion, Optimizer, RuntimeStage, ScanConfig, ScanReport, Secret, SecurityError,
    SecurityHardeningStrategy, SecurityScanner, SizeOptimizationStrategy, Vulnerability,
    VulnerabilitySeverity,
};
/// Service Mesh module for Istio/Linkerd
pub mod service_mesh;
/// Re-export service mesh types
pub use service_mesh::{
    ConnectionPoolConfig, DistributedTracer, ErrorMetrics, FaultType, IstioConfig,
    IstioConfigManager, IstioError, IstioService, LatencyMetrics, LoadBalancerAlgorithm,
    MetricsCollector, MetricsReport, OutlierDetectionConfig, PerformanceAnalysis, RequestMetrics,
    SpanEvent, SpanRecord, SpanStatus, TraceContext, TrafficManager, TrafficPolicyConfig,
    TrafficSplit,
};
/// CI/CD module for GitOps and pipeline integration
pub mod cicd;
/// Re-export CI/CD types
pub use cicd::{
    ArgoCDApplication, BlueGreenDeployment, CanaryDeployment, DeploymentConfig, DeploymentStatus,
    DeploymentStrategy, Error as CICDError, FluxHelmRelease, GitHubActionsWorkflow,
    GitLabCIPipeline, GitOpsConfig, GitOpsManager, JenkinsPipeline, PipelineConfig, PipelineEvent,
    PipelineManager, PipelineStage, PipelineStatus, RollingDeployment,
};
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    #[test]
    fn test_module_structure() {
        // Verify that the module structure is correct
        let _cluster: Option<AmberCluster> = None;
        let _workload: Option<AmberWorkload> = None;
    }
}
