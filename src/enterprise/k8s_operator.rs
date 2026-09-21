// Kubernetes Operator for Amber Cluster Management
// 实现 Amber 集群的 Kubernetes Operator，提供自动化集群管理能力

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anyhow::{Result, Context};
use kube::{
    api::{Api, ListParams, PatchParams, Patch},
    client::Client,
    runtime::{
        controller::Action,
        events::{Event, EventRecorder, Recorder},
        finalizer::{finalizer, Event as Finalizer},
        waiter::Condition,
        Controller, WatchStreamExt,
    },
    Resource, ResourceExt,
};
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, StatefulSet},
        core::v1::{ConfigMap, Service, Endpoints, Node, Pod},
    },
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
    apimachinery::pkg::{
        api::resource::Quantity,
        apis::meta::v1::{LabelSelector, Time},
        runtime::util::structured::StructuredList,
    },
    chrono::Utc,
};
use serde::{Deserialize, Serialize};
use schemars::{JsonSchema};
use std::::{
    collections::BTreeMap,
    sync::Arc,
    time::Duration,
};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
/// Custom Resource Definition for AmberCluster
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "amberjs.io",
    version = "v1",
    kind = "AmberCluster",
    plural = "amberjsclusters",
    shortname = "bc",
    namespaced
)]
pub struct AmberClusterSpec {
    /// Amber version to deploy
    pub version: String,
    /// Number of replica pods
    pub nodes: usize,
    /// Cluster configuration
    pub config: ClusterConfig,
    /// Resource requirements
    pub resources: ResourceRequirements,
}
/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClusterConfig {
    /// Namespace for the cluster
    pub namespace: Option<String>,
    /// Image repository
    pub image: Option<String>,
    /// Service type
    pub service_type: Option<String>,
    /// Enable monitoring
    pub monitoring: Option<bool>,
    /// Enable auto-scaling
    pub auto_scaling: Option<bool>,
    /// Node selector labels
    pub node_selector: Option<BTreeMap<String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String>>,
    /// Tolerations
    pub tolerations: Option<Vec<Toleration>>,
}
/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceRequirements {
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub storage: Option<String>,
}
/// Toleration configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Toleration {
    pub key: String,
    pub operator: String,
    pub value: Option<String>,
    pub effect: String,
}
/// AmberCluster status
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AmberClusterStatus {
    /// Current phase of the cluster
    pub phase: ClusterPhase,
    /// Number of ready nodes
    pub ready_nodes: usize,
    /// Total number of nodes
    pub total_nodes: usize,
    /// Last update time
    pub last_update: Option<Time>,
    /// Conditions
    pub conditions: Vec<ClusterCondition>,
    /// Current version
    pub current_version: Option<String>,
    /// Target version for upgrade
    pub target_version: Option<String>,
    /// Upgrade progress
    pub upgrade_progress: Option<UpgradeProgress>,
    /// Health status
    pub health_status: HealthStatus,
    /// Node statuses
    pub node_statuses: Vec<NodeStatus>,
}
/// Upgrade progress information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpgradeProgress {
    /// Current step
    pub current_step: String,
    /// Total steps
    pub total_steps: usize,
    /// Completion percentage
    pub percentage: u8,
    /// Start time
    pub started_at: Option<Time>,
    /// Estimated completion time
    pub estimated_completion: Option<Time>,
}
/// Health status
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct HealthStatus {
    /// Overall health status
    pub status: HealthState,
    /// Last health check time
    pub last_check: Option<Time>,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Health check details
    pub checks: Vec<HealthCheck>,
}
/// Health states
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum HealthState {
    /// All nodes are healthy
    Healthy,
    /// Some nodes are unhealthy
    Degraded,
    /// Cluster is unhealthy
    Unhealthy,
    /// Health check failed
    Unknown,
}
/// Individual node status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NodeStatus {
    /// Node name
    pub name: String,
    /// Node IP
    pub ip: String,
    /// Node phase
    pub phase: NodePhase,
    /// Last health check
    pub last_health_check: Option<Time>,
    /// Restart count
    pub restart_count: u32,
}
/// Node phases
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum NodePhase {
    /// Node is pending
    Pending,
    /// Node is running
    Running,
    /// Node is upgrading
    Upgrading,
    /// Node is failing
    Failing,
    /// Node has failed
    Failed,
}
/// Health check details
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: bool,
    /// Check message
    pub message: String,
    /// Last check time
    pub last_check: Option<Time>,
}
/// Cluster phases
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ClusterPhase {
    /// Cluster is creating
    Creating,
    /// Cluster is running
    Running,
    /// Cluster is updating
    Updating,
    /// Cluster is scaling
    Scaling,
    /// Cluster is failing
    Failing,
    /// Cluster has failed
    Failed,
    /// Cluster is terminating
    Terminating,
}
impl Default for ClusterPhase {
    fn default() -> Self {
        ClusterPhase::Creating
    }
}
/// Cluster conditions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClusterCondition {
    pub last_transition_time: Time,
    pub message: String,
    pub reason: String,
    pub status: String,
    pub type_: String,
}
/// Operator configuration
#[derive(Debug, Clone)]
pub struct OperatorConfig {
    /// Reconciliation interval
    pub reconcile_interval: Duration,
    /// Maximum concurrent reconciliations
    pub max_concurrent: usize,
    /// Leader election enabled
    pub leader_election: bool,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Upgrade timeout
    pub upgrade_timeout: Duration,
    /// Enable monitoring integration
    pub monitoring_enabled: bool,
    /// Enable auto-healing
    pub auto_healing: bool,
}
/// AmberOperator main struct
#[derive(Debug)]
pub struct AmberOperator {
    /// Kubernetes client
    client: Client,
    /// Operator configuration
    config: OperatorConfig,
    /// Cluster API
    clusters: Api<AmberCluster>,
    /// Deployment API
    deployments: Api<Deployment>,
    /// StatefulSet API
    statefulsets: Api<StatefulSet>,
    /// Service API
    services: Api<Service>,
    /// ConfigMap API
    configmaps: Api<ConfigMap>,
    /// Event recorder
    recorder: Recorder,
}
impl AmberOperator {
    /// Create a new AmberOperator
    pub fn new(client: Client, config: OperatorConfig) -> Self {
        let namespaces: _ = client.clone();
        let recorder: _ = EventRecorder::new(client.clone(), "amberjs-operator".to_string());
        Self {
            client: client.clone(),
            config,
            clusters: Api::all(client.clone()),
            deployments: Api::all(client.clone()),
            statefulsets: Api::all(client.clone()),
            services: Api::all(client.clone()),
            configmaps: Api::all(client.clone()),
            recorder,
        }
    }
    /// Get default operator configuration
    pub fn default_config() -> OperatorConfig {
        OperatorConfig {
            reconcile_interval: Duration::from_secs(30),
            max_concurrent: 10,
            leader_election: true,
            health_check_interval: Duration::from_secs(60),
            upgrade_timeout: Duration::from_secs(600),
            monitoring_enabled: true,
            auto_healing: true,
        }
    }
    /// Run the operator
    pub async fn run(&self) -> Result<()> {
        info!("Starting Amber Kubernetes Operator");
        let controller: _ = Controller::new(self.clusters.clone(), ListParams::default())
            .run(
                self.reconcile(),
                self.error_policy(),
                Arc::new(Mutex::new(self.clone()))
            )
            .await
            .context("Failed to start controller")?;
        info!("Amber Operator started successfully");
        Ok(()))
    }
    /// Reconciliation logic
    fn reconcile(&self) -> Arc<dyn Fn(AmberCluster) -> Action + Send + Sync + 'static> {
        Arc::new(Mutex::new(move |amberjs_cluster: AmberCluster| {)),
            let client: _ = self.client.clone()));
            let clusters: _ = self.clusters.clone();
            let deployments: _ = self.deployments.clone();
            let statefulsets: _ = self.statefulsets.clone();
            let services: _ = self.services.clone();
            let configmaps: _ = self.configmaps.clone();
            let recorder: _ = self.recorder.clone();
            async move {
                let name: _ = amberjs_cluster.name_any();
                let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
                info!("Reconciling AmberCluster: {}/{}", namespace, name);
                // Apply finalizer
                let finalizer_action: _ = finalizer(
                    &clusters,
                    "amberjs.io/finalizer",
                    amberjs_cluster.clone(),
                    |event| async {
                        match event {
                            Finalizer::Apply(amberjs_cluster) => {
                                Self::reconcile_apply(
                                    client.clone(),
                                    amberjs_cluster.clone(),
                                    deployments.clone(),
                                    statefulsets.clone(),
                                    services.clone(),
                                    configmaps.clone(),
                                    recorder.clone(),
                                )
                                .await
                            }
                            Finalizer::Cleanup(amberjs_cluster) => {
                                Self::reconcile_cleanup(
                                    client.clone(),
                                    amberjs_cluster.clone(),
                                    deployments.clone(),
                                    statefulsets.clone(),
                                    services.clone(),
                                    configmaps.clone(),
                                    recorder.clone(),
                                )
                                .await
                            }
                        }
                    },
                )
                .await;
                match finalizer_action {
                    Ok(action) => action,
                    Err(e) => {
                        error!("Reconciliation failed: {}", e);
                        Action::requeue(Duration::from_secs(30))
                    }
                }
            }
        })
    }
    /// Error policy
    fn error_policy(&self) -> Arc<dyn Fn(&kube::Error, &AmberCluster) -> Action + Send + Sync + 'static> {
        Arc::new(Mutex::new(move |_error, _amberjs_cluster| {)),
            warn!("Error occurred during reconciliation")));
            Action::requeue(Duration::from_secs(60))
        })
    }
    /// Apply reconciliation
    async fn reconcile_apply(
        client: Client,
        amberjs_cluster: AmberCluster,
        deployments: Api<Deployment>,
        statefulsets: Api<StatefulSet>,
        services: Api<Service>,
        configmaps: Api<ConfigMap>,
        recorder: Recorder,
    ) -> Result<Action> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        // Check if upgrade is needed
        let status: _ = amberjs_cluster.status.clone().unwrap_or_default();
        let needs_upgrade: _ = Self::check_upgrade_needed(&amberjs_cluster, &status);
        if needs_upgrade {
            info!("Initiating upgrade for AmberCluster: {}/{}", namespace, name);
            return Self::perform_upgrade(
                client.clone(),
                amberjs_cluster.clone(),
                deployments.clone(),
                statefulsets.clone(),
                services.clone(),
                configmaps.clone(),
                recorder.clone(),
            )
            .await;
        }
        // Create or update ConfigMap
        let configmap: _ = Self::create_configmap(&amberjs_cluster)?;
        configmaps
            .patch(
                &format!("{}-config", name),
                &PatchParams::apply("amberjs-operator"),
                &Patch::Apply(&configmap),
            )
            .await
            .context("Failed to patch ConfigMap")?;
        // Create or update StatefulSet for the cluster
        let statefulset: _ = Self::create_statefulset(&amberjs_cluster)?;
        statefulsets
            .patch(
                &name,
                &PatchParams::apply("amberjs-operator"),
                &Patch::Apply(&statefulset),
            )
            .await
            .context("Failed to patch StatefulSet")?;
        // Create or update Service
        let service: _ = Self::create_service(&amberjs_cluster)?;
        services
            .patch(
                &format!("{}-svc", name),
                &PatchParams::apply("amberjs-operator"),
                &Patch::Apply(&service),
            )
            .await
            .context("Failed to patch Service")?;
        // Perform health check
        let health_status: _ = Self::perform_health_check(
            client.clone(),
            &amberjs_cluster,
            statefulsets.clone(),
        )
        .await;
        // Update status
        let mut new_status = amberjs_cluster.status.clone().unwrap_or_default();
        new_status.phase = ClusterPhase::Running;
        new_status.ready_nodes = amberjs_cluster.spec.nodes;
        new_status.total_nodes = amberjs_cluster.spec.nodes;
        new_status.current_version = Some(amberjs_cluster.spec.version.clone());
        new_status.health_status = health_status;
        new_status.last_update = Some(Time(Utc::now());
        let _: _ = recorder
            .publish(Event::normal(
                &amberjs_cluster,
                &format!("AmberCluster {} reconciled successfully", name),
            ))
            .await;
        Ok(Action::requeue(Duration::from_secs(30))
    }
    /// Cleanup reconciliation
    async fn reconcile_cleanup(
        client: Client,
        amberjs_cluster: AmberCluster,
        deployments: Api<Deployment>,
        statefulsets: Api<StatefulSet>,
        services: Api<Service>,
        configmaps: Api<ConfigMap>,
        recorder: Recorder,
    ) -> Result<Action> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        info!("Cleaning up AmberCluster: {}/{}", namespace, name);
        // Delete StatefulSet
        if let Err(e) = statefulsets.delete(&name, &Default::default()).await {
            warn!("Failed to delete StatefulSet: {}", e);
        }
        // Delete Service
        if let Err(e) = services
            .delete(&format!("{}-svc", name), &Default::default())
            .await
        {
            warn!("Failed to delete Service: {}", e);
        }
        // Delete ConfigMap
        if let Err(e) = configmaps
            .delete(&format!("{}-config", name), &Default::default())
            .await
        {
            warn!("Failed to delete ConfigMap: {}", e);
        }
        let _: _ = recorder
            .publish(Event::normal(
                &amberjs_cluster,
                &format!("AmberCluster {} deleted", name),
            ))
            .await;
        Ok(Action::await())
    }
    /// Create ConfigMap for the cluster
    fn create_configmap(amberjs_cluster: &AmberCluster) -> Result<ConfigMap> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        let mut data = BTreeMap::new();
        data.insert("version".to_string(), amberjs_cluster.spec.version.clone());
        data.insert(
            "nodes".to_string(),
            amberjs_cluster.spec.nodes.to_string(),
        );
        Ok(ConfigMap {
            metadata: kube::api::ObjectMeta {
                name: Some(format!("{}-config", name)),
                namespace: Some(namespace),
                labels: Some(Self::labels_for_cluster(name)),
                ..Default::default()
            },
            data: Some(data),
            ..Default::default()
        })
    }
    /// Create StatefulSet for the cluster
    fn create_statefulset(amberjs_cluster: &AmberCluster) -> Result<StatefulSet> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        let spec: _ = &amberjs_cluster.spec;
        let mut labels = Self::labels_for_cluster(name);
        labels.insert("app".to_string(), "amberjs".to_string());
        Ok(StatefulSet {
            metadata: kube::api::ObjectMeta {
                name: Some(name.clone()),
                namespace: Some(namespace),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::apps::v1::StatefulSetSpec {
                service_name: format!("{}-svc", name),
                replicas: Some(spec.nodes as i32),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..Default::default()
                },
                template: k8s_openapi::api::core::v1::PodTemplateSpec {
                    metadata: Some(kube::api::ObjectMeta {
                        labels: Some(labels),
                        ..Default::default()
                    }),
                    spec: Some(k8s_openapi::api::core::v1::PodSpec {
                        containers: vec![k8s_openapi::api::core::v1::Container {
                            name: "amberjs".to_string(),
                            image: Some(spec.config.image.clone().unwrap_or_default()),
                            ports: Some(vec![k8s_openapi::api::core::v1::ContainerPort {
                                container_port: 3000,
                                ..Default::default()
                            }]),
                            resources: Some(k8s_openapi::api::core::v1::ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".to_string(), Quantity(spec.resources.cpu.clone().unwrap_or_default()),
                                    ("memory".to_string(), Quantity(spec.resources.memory.clone().unwrap_or_default()),
                                ])),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        })
    }
    /// Create Service for the cluster
    fn create_service(amberjs_cluster: &AmberCluster) -> Result<Service> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        let labels: _ = Self::labels_for_cluster(name);
        Ok(Service {
            metadata: kube::api::ObjectMeta {
                name: Some(format!("{}-svc", name)),
                namespace: Some(namespace),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ServiceSpec {
                type_: Some(
                    amberjs_cluster
                        .spec
                        .config
                        .service_type
                        .clone()
                        .unwrap_or_else(|| "ClusterIP".to_string()),
                ),
                selector: Some(labels),
                ports: Some(vec![k8s_openapi::api::core::v1::ServicePort {
                    port: 3000,
                    target_port: Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(3000)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
    /// Generate labels for cluster resources
    fn labels_for_cluster(name: &str) -> BTreeMap<String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("amberjs.io/cluster".to_string(), name.to_string());
        labels.insert("amberjs.io/component".to_string(), "cluster".to_string());
        labels
    }
    /// Check if cluster needs upgrade
    fn check_upgrade_needed(
        amberjs_cluster: &AmberCluster,
        status: &AmberClusterStatus,
    ) -> bool {
        if let Some(current_version) = &status.current_version {
            if current_version != &amberjs_cluster.spec.version {
                return true;
            }
        }
        false
    }
    /// Perform rolling upgrade
    async fn perform_upgrade(
        client: Client,
        amberjs_cluster: AmberCluster,
        deployments: Api<Deployment>,
        statefulsets: Api<StatefulSet>,
        services: Api<Service>,
        configmaps: Api<ConfigMap>,
        recorder: Recorder,
    ) -> Result<Action> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        info!("Starting rolling upgrade for AmberCluster: {}/{}", namespace, name);
        // Get current StatefulSet
        let current_ss: _ = statefulsets.get(&name).await
            .context("Failed to get StatefulSet for upgrade")?;
        // Create new StatefulSet with updated image
        let mut new_ss = current_ss.clone();
        if let Some(spec) = &mut new_ss.spec {
            if let Some(template) = &mut spec.template.spec {
                for container in &mut template.containers {
                    if container.name == "amberjs" {
                        container.image = Some(amberjs_cluster.spec.version.clone());
                    }
                }
            }
        }
        // Apply rolling update strategy
        if let Some(spec) = &mut new_ss.spec {
            spec.update_strategy = Some(k8s_openapi::api::apps::v1::StatefulSetUpdateStrategy {
                type_: Some("RollingUpdate".to_string()),
                rolling_update: Some(
                    k8s_openapi::api::apps::v1::RollingUpdateStatefulSetStrategy {
                        partition: Some(0),
                        max_unavailable: Some(
                            k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(1),
                        ),
                    }
                ),
            });
        }
        // Patch the StatefulSet
        statefulsets
            .patch(
                &name,
                &PatchParams::apply("amberjs-operator"),
                &Patch::Apply(&new_ss),
            )
            .await
            .context("Failed to patch StatefulSet for upgrade")?;
        // Update status to upgrading
        let mut status = amberjs_cluster.status.clone().unwrap_or_default();
        status.phase = ClusterPhase::Upgrading;
        status.target_version = Some(amberjs_cluster.spec.version.clone());
        status.upgrade_progress = Some(UpgradeProgress {
            current_step: "Starting upgrade".to_string(),
            total_steps: amberjs_cluster.spec.nodes,
            percentage: 0,
            started_at: Some(Time(Utc::now()),
            estimated_completion: None,
        });
        let _: _ = recorder
            .publish(Event::normal(
                &amberjs_cluster,
                &format!("Started upgrade to version {}", amberjs_cluster.spec.version),
            ))
            .await;
        Ok(Action::requeue(Duration::from_secs(10))
    }
    /// Perform health check on cluster
    async fn perform_health_check(
        client: Client,
        amberjs_cluster: &AmberCluster,
        statefulsets: Api<StatefulSet>,
    ) -> HealthStatus {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        let mut checks = Vec::new();
        let mut healthy_nodes = 0;
        // Check StatefulSet status
        match statefulsets.get(&name).await {
            Ok(ss) => {
                if let Some(status) = &ss.status {
                    let ready_replicas: _ = status.ready_replicas.unwrap_or(0);
                    let replicas: _ = status.replicas.unwrap_or(0);
                    checks.push(HealthCheck {
                        name: "StatefulSet Ready".to_string(),
                        status: ready_replicas == replicas,
                        message: format!(
                            "Ready: {}/{}, Available: {}",
                            ready_replicas,
                            replicas,
                            status.available_replicas.unwrap_or(0)
                        ),
                        last_check: Some(Time(Utc::now()),
                    });
                    healthy_nodes = ready_replicas as usize;
                }
            }
            Err(e) => {
                checks.push(HealthCheck {
                    name: "StatefulSet Status".to_string(),
                    status: false,
                    message: format!("Failed to get StatefulSet: {}", e),
                    last_check: Some(Time(Utc::now()),
                });
            }
        }
        // Determine overall health
        let overall_status: _ = if healthy_nodes == amberjs_cluster.spec.nodes {
            HealthState::Healthy
        } else if healthy_nodes > 0 {
            HealthState::Degraded
        } else {
            HealthState::Unhealthy
        };
        HealthStatus {
            status: overall_status,
            last_check: Some(Time(Utc::now()),
            healthy_nodes,
            checks,
        }
    }
    /// Perform auto-healing if enabled
    async fn perform_auto_healing(
        client: Client,
        amberjs_cluster: &AmberCluster,
        statefulsets: Api<StatefulSet>,
        recorder: Recorder,
    ) -> Result<Action> {
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        // Get current StatefulSet
        let ss: _ = statefulsets.get(&name).await
            .context("Failed to get StatefulSet for healing")?;
        let mut needs_healing = false;
        let mut healing_actions = Vec::new();
        if let Some(status) = &ss.status {
            let ready: _ = status.ready_replicas.unwrap_or(0) as usize;
            let total: _ = status.replicas.unwrap_or(0) as usize;
            if ready < total {
                info!("Auto-healing triggered for {}/{}: {}/{} pods ready",
                    namespace, name, ready, total);
                needs_healing = true;
                // Force restart of failed pods by patching the StatefulSet
                let mut patched_ss = ss.clone();
                if let Some(spec) = &mut patched_ss.spec {
                    if let Some(template) = &mut spec.template.spec {
                        // Add annotation to force pod recreation
                        for container in &mut template.containers {
                            if container.name == "amberjs" {
                                container.env = Some(vec![
                                    k8s_openapi::api::core::v1::EnvVar {
                                        name: "AMBER_AUTO_HEAL".to_string(),
                                        value: Some("true".to_string()),
                                        value_from: None,
                                    }
                                ]);
                            }
                        }
                    }
                }
                statefulsets
                    .patch(
                        &name,
                        &PatchParams::apply("amberjs-operator"),
                        &Patch::Apply(&patched_ss),
                    )
                    .await
                    .context("Failed to patch StatefulSet for healing")?;
                healing_actions.push("Restarted failed pods".to_string());
                let _: _ = recorder
                    .publish(Event::normal(
                        amberjs_cluster,
                        &format!("Auto-healed {} pods", total - ready),
                    ))
                    .await;
            }
        }
        if needs_healing {
            Ok(Action::requeue(Duration::from_secs(30))
        } else {
            Ok(Action::requeue(Duration::from_secs(60))
        }
    }
    /// Get cluster metrics
    pub async fn get_cluster_metrics(
        &self,
        amberjs_cluster: &AmberCluster,
    ) -> Result<BTreeMap<String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String>> {
        let mut metrics = BTreeMap::new();
        let name: _ = amberjs_cluster.name_any();
        let namespace: _ = amberjs_cluster.namespace().unwrap_or_default();
        // Get StatefulSet metrics
        if let Ok(ss) = self.statefulsets.get(&name).await {
            if let Some(status) = ss.status {
                metrics.insert("replicas".to_string(),
                    status.replicas.unwrap_or(0).to_string());
                metrics.insert("ready_replicas".to_string(),
                    status.ready_replicas.unwrap_or(0).to_string());
                metrics.insert("updated_replicas".to_string(),
                    status.updated_replicas.unwrap_or(0).to_string());
                metrics.insert("available_replicas".to_string(),
                    status.available_replicas.unwrap_or(0).to_string());
            }
        }
        Ok(metrics)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
use std::collections::{HashMap, BTreeMap};
use std::time::Duration;
    #[tokio::test]
    async fn test_create_operator() {
        let client: _ = Client::try_default().await.unwrap();
        let config: _ = OperatorConfig {
            reconcile_interval: Duration::from_secs(30),
            max_concurrent: 10,
            leader_election: true,
        };
        let operator: _ = AmberOperator::new(client, config);
        assert_eq!(operator.config.reconcile_interval, Duration::from_secs(30));
    }
    #[test]
    fn test_create_configmap() {
        let cluster: _ = AmberCluster::new(
            "test-cluster",
            AmberClusterSpec {
                version: "v1.0.0".to_string(),
                nodes: 3,
                config: ClusterConfig {
                    namespace: Some("default".to_string()),
                    image: Some("amberjs:latest".to_string()),
                    service_type: Some("ClusterIP".to_string()),
                    monitoring: Some(true),
                    auto_scaling: Some(true),
                    node_selector: None,
                    tolerations: None,
                },
                resources: ResourceRequirements {
                    cpu: Some("500m".to_string()),
                    memory: Some("1Gi".to_string()),
                    storage: Some("10Gi".to_string()),
                },
            },
        );
        let configmap: _ = AmberOperator::create_configmap(&cluster).unwrap();
        assert_eq!(configmap.metadata.name, Some("test-cluster-config".to_string());
    }
    #[test]
    fn test_labels_for_cluster() {
        let labels: _ = AmberOperator::labels_for_cluster("test-cluster");
        assert_eq!(
            labels.get("amberjs.io/cluster"),
            Some(&"test-cluster".to_string());
        assert_eq!(
            labels.get("amberjs.io/component"),
            Some(&"cluster".to_string());
    }
    #[test]
    fn test_check_upgrade_needed() {
        let cluster: _ = AmberCluster::new(
            "test-cluster",
            AmberClusterSpec {
                version: "v2.0.0".to_string(),
                nodes: 3,
                config: ClusterConfig {
                    namespace: Some("default".to_string()),
                    image: Some("amberjs:v2.0.0".to_string()),
                    service_type: Some("ClusterIP".to_string()),
                    monitoring: Some(true),
                    auto_scaling: Some(true),
                    node_selector: None,
                    tolerations: None,
                },
                resources: ResourceRequirements {
                    cpu: Some("500m".to_string()),
                    memory: Some("1Gi".to_string()),
                    storage: Some("10Gi".to_string()),
                },
            },
        );
        let old_status: _ = AmberClusterStatus {
            phase: ClusterPhase::Running,
            ready_nodes: 3,
            total_nodes: 3,
            last_update: Some(Time(Utc::now()),
            conditions: vec![],
            current_version: Some("v1.0.0".to_string()),
            target_version: None,
            upgrade_progress: None,
            health_status: HealthStatus::default(),
            node_statuses: vec![],
        };
        assert!(AmberOperator::check_upgrade_needed(&cluster, &old_status));
        let current_status: _ = AmberClusterStatus {
            current_version: Some("v2.0.0".to_string()),
            ..old_status
        };
        assert!(!AmberOperator::check_upgrade_needed(&cluster, &current_status));
    }
    #[test]
    fn test_default_config() {
        let config: _ = AmberOperator::default_config();
        assert_eq!(config.reconcile_interval, Duration::from_secs(30));
        assert_eq!(config.max_concurrent, 10);
        assert!(config.leader_election);
        assert!(config.monitoring_enabled);
        assert!(config.auto_healing);
    }
    #[test]
    fn test_health_status_determination() {
        let mut checks = Vec::new();
        checks.push(HealthCheck {
            name: "Ready".to_string(),
            status: true,
            message: "All pods ready".to_string(),
            last_check: Some(Time(Utc::now()),
        });
        let health_status: _ = HealthStatus {
            status: HealthState::Healthy,
            last_check: Some(Time(Utc::now()),
            healthy_nodes: 3,
            checks,
        };
        assert_eq!(health_status.status, HealthState::Healthy);
        assert_eq!(health_status.healthy_nodes, 3);
    }
}
}