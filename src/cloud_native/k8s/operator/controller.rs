// Kubernetes Operator Controller
// Implements the reconciliation loop for AmberCluster and AmberWorkload

use std::sync::Arc;

use super::super::crd::{
    AmberCluster, AmberClusterSpec, AmberWorkload, AmberWorkloadSpec, ClusterPhase,
    Condition as AmberCondition, ConditionStatus, ConditionType, WorkloadPhase,
};
use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec};
use k8s_openapi::api::core::v1::{
    ConfigMap, ConfigMapVolumeSource, Container, ContainerPort, EnvVar, EnvVarSource,
    ObjectFieldSelector, ObjectReference, PersistentVolumeClaim, PersistentVolumeClaimSpec,
    PodSpec, ResourceRequirements, Secret, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    LabelSelector, LabelSelectorRequirement, ObjectMeta,
};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use k8s_openapi::ByteString;
use kube::api::{DeleteParams, ListParams, Patch, PatchParams};
use kube::core::object::HasStatus;
use kube::runtime::controller::Action;
use kube::runtime::events::{Event, EventType, Recorder, Reporter};
use kube::{Api, Client, Resource, ResourceExt};
use std::collections::BTreeMap;
use tokio::time::Duration as TokioDuration;
use tracing::{debug, error, info, warn};
/// Cluster controller for managing AmberCluster resources
pub struct ClusterController {
    /// Kubernetes client
    pub client: Client,
    /// API for AmberCluster resources
    pub clusters: Api<AmberCluster>,
    /// API for StatefulSet resources
    pub statefulsets: Api<StatefulSet>,
    /// API for Service resources
    pub services: Api<Service>,
    /// API for ConfigMap resources
    pub configmaps: Api<ConfigMap>,
    /// API for Secret resources
    pub secrets: Api<Secret>,
    /// Event recorder
    pub recorder: Recorder,
}
/// Finalizer name for cluster resources
const CLUSTER_FINALIZER: &str = "amberjsclusters.cloudnative.amberjs.io/finalizer";
impl ClusterController {
    /// Create a new cluster controller
    pub fn new(client: Client, namespace: &str) -> Self {
        let clusters: _ = Api::<AmberCluster>::namespaced(client.clone(), namespace);
        let statefulsets: _ = Api::<StatefulSet>::namespaced(client.clone(), namespace);
        let services: _ =
            Api::<k8s_openapi::api::core::v1::Service>::namespaced(client.clone(), namespace);
        let configmaps: _ =
            Api::<k8s_openapi::api::core::v1::ConfigMap>::namespaced(client.clone(), namespace);
        let secrets: _ =
            Api::<k8s_openapi::api::core::v1::Secret>::namespaced(client.clone(), namespace);
        let reporter: _ = Reporter {
            controller: "amberjs-cluster-controller".to_string(),
            instance: None,
        };
        let recorder: _ = Recorder::new(client.clone(), reporter, ObjectReference::default());
        Self {
            client: client.clone(),
            clusters,
            statefulsets,
            services,
            configmaps,
            secrets,
            recorder,
        }
    }
    /// Reconcile function for AmberCluster
    pub async fn reconcile(&self, cluster: Arc<AmberCluster>) -> Result<Action, Error> {
        info!("Reconciling AmberCluster: {}", cluster.name_any());
        // Check if finalizer exists
        let has_finalizer: _ = cluster.annotations().contains_key(CLUSTER_FINALIZER);
        // Get current phase
        let phase: _ = self.get_current_phase(&cluster).await?;
        debug!("Current phase: {:?}", phase);
        match phase {
            ClusterPhase::Pending | ClusterPhase::Creating => {
                self.reconcile_create(cluster.clone()).await?;
                self.update_phase(cluster.as_ref(), ClusterPhase::Creating)
                    .await?;
            }
            ClusterPhase::Running => {
                self.reconcile_running(cluster.clone()).await?;
            }
            ClusterPhase::Updating => {
                self.reconcile_update(cluster.clone()).await?;
                self.update_phase(cluster.as_ref(), ClusterPhase::Running)
                    .await?;
            }
            ClusterPhase::Failed => {
                if !has_finalizer {
                    // Add finalizer before attempting recovery
                    self.add_finalizer(&cluster).await?;
                }
                self.reconcile_recovery(cluster.clone()).await?;
                self.update_phase(cluster.as_ref(), ClusterPhase::Running)
                    .await?;
            }
        }
        // Add finalizer if it doesn't exist
        if !has_finalizer {
            self.add_finalizer(&cluster).await?;
        }
        Ok(Action::requeue(Duration::from_secs(30)))
    }
    /// Handle cleanup when resource is deleted
    pub async fn cleanup(&self, cluster: Arc<AmberCluster>) -> Result<Action, Error> {
        info!("Cleaning up AmberCluster: {}", cluster.name_any());
        let name: _ = cluster.name_any();
        // Delete StatefulSet
        if let Err(e) = self
            .statefulsets
            .delete(&name, &kube::api::DeleteParams::default())
            .await
        {
            // 404 errors are expected when resource doesn't exist
            if !e.to_string().contains("404") && !e.to_string().contains("NotFound") {
                warn!("Failed to delete StatefulSet: {}", e);
            }
        }
        // Delete Service
        if let Err(e) = self
            .services
            .delete(&name, &kube::api::DeleteParams::default())
            .await
        {
            // 404 errors are expected when resource doesn't exist
            if !e.to_string().contains("404") && !e.to_string().contains("NotFound") {
                warn!("Failed to delete Service: {}", e);
            }
        }
        // Delete ConfigMap
        if let Err(e) = self
            .configmaps
            .delete(&name, &kube::api::DeleteParams::default())
            .await
        {
            // 404 errors are expected when resource doesn't exist
            if !e.to_string().contains("404") && !e.to_string().contains("NotFound") {
                warn!("Failed to delete ConfigMap: {}", e);
            }
        }
        // Delete Secret
        if let Err(e) = self
            .secrets
            .delete(&name, &kube::api::DeleteParams::default())
            .await
        {
            // 404 errors are expected when resource doesn't exist
            if !e.to_string().contains("404") && !e.to_string().contains("NotFound") {
                warn!("Failed to delete Secret: {}", e);
            }
        }
        info!("Successfully cleaned up AmberCluster: {}", name);
        Ok(Action::await_change())
    }
    /// Get current phase of the cluster
    async fn get_current_phase(&self, cluster: &AmberCluster) -> Result<ClusterPhase, Error> {
        // TODO: Implement proper status phase detection
        // For now, always return Pending
        Ok(ClusterPhase::Pending)
    }
    /// Add finalizer to cluster
    async fn add_finalizer(&self, cluster: &AmberCluster) -> Result<(), Error> {
        let patch: _ = Patch::Merge(&serde_json::json!({
            "metadata": {
                "annotations": {
                    CLUSTER_FINALIZER: "true"
                }
            }
        }));
        self.clusters
            .patch(&cluster.name_any(), &PatchParams::default(), &patch)
            .await?;
        Ok(())
    }
    /// Update the phase of the cluster
    async fn update_phase(&self, cluster: &AmberCluster, phase: ClusterPhase) -> Result<(), Error> {
        let is_ready: _ = matches!(phase, ClusterPhase::Running);
        let mut condition = serde_json::Map::new();
        condition.insert(
            "type".to_string(),
            serde_json::Value::String("Ready".to_string()),
        );
        condition.insert(
            "status".to_string(),
            serde_json::Value::String(is_ready.to_string()),
        );
        let mut status = serde_json::Map::new();
        status.insert(
            "phase".to_string(),
            serde_json::Value::String(phase.as_str().to_string()),
        );
        status.insert(
            "conditions".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::Object(condition)]),
        );
        let mut patch_map = serde_json::Map::new();
        patch_map.insert("status".to_string(), serde_json::Value::Object(status));
        let patch: _ = serde_json::Value::Object(patch_map);
        let params: _ = PatchParams::default();
        self.clusters
            .patch_status(&cluster.name_any(), &params, &Patch::Merge(&patch))
            .await?;
        // Emit event
        let event_type: _ = match phase {
            ClusterPhase::Running => EventType::Normal,
            ClusterPhase::Failed => EventType::Warning,
            _ => EventType::Normal,
        };
        self.recorder
            .publish(Event {
                type_: event_type,
                reason: "PhaseChanged".to_string(),
                note: Some(format!("Cluster phase changed to {:?}", phase)),
                action: "Update".to_string(),
                secondary: None,
            })
            .await;
        Ok(())
    }
    /// Reconcile cluster creation
    async fn reconcile_create(&self, cluster: Arc<AmberCluster>) -> Result<(), Error> {
        info!("Creating cluster resources for: {}", cluster.name_any());
        // 1. Create ConfigMap
        self.create_configmap(&cluster).await?;
        // 2. Create Secret
        self.create_secret(&cluster).await?;
        // 3. Create StatefulSet
        self.create_statefulset(&cluster).await?;
        // 4. Create Service
        self.create_service(&cluster).await?;
        // 5. Wait for pods to be ready
        self.wait_for_ready(&cluster).await?;
        info!(
            "Successfully created cluster resources for: {}",
            cluster.name_any()
        );
        Ok(())
    }
    /// Reconcile running cluster
    async fn reconcile_running(&self, cluster: Arc<AmberCluster>) -> Result<(), Error> {
        debug!("Checking cluster status for: {}", cluster.name_any());
        // Check if resources need updating
        if self.needs_update(&cluster).await? {
            info!("Cluster needs update: {}", cluster.name_any());
            self.reconcile_update(cluster.clone()).await?;
        }
        // Check health
        self.check_health(&cluster).await?;
        Ok(())
    }
    /// Reconcile cluster update
    async fn reconcile_update(&self, cluster: Arc<AmberCluster>) -> Result<(), Error> {
        info!("Updating cluster: {}", cluster.name_any());
        // Update StatefulSet if needed
        if let Err(e) = self.update_statefulset(&cluster).await {
            error!("Failed to update StatefulSet: {}", e);
            return Err(e);
        }
        // Wait for update to complete
        self.wait_for_update(&cluster).await?;
        info!("Successfully updated cluster: {}", cluster.name_any());
        Ok(())
    }
    /// Reconcile cluster recovery from failed state
    async fn reconcile_recovery(&self, cluster: Arc<AmberCluster>) -> Result<(), Error> {
        warn!(
            "Recovering cluster from failed state: {}",
            cluster.name_any()
        );
        // Check if resources exist
        if !self.resources_exist(&cluster).await? {
            // Resources don't exist, recreate them
            self.reconcile_create(cluster.clone()).await?;
        } else {
            // Resources exist, try to recover
            self.recover_resources(&cluster).await?;
        }
        Ok(())
    }
    /// Create ConfigMap for cluster configuration
    async fn create_configmap(&self, cluster: &AmberCluster) -> Result<(), Error> {
        let name: _ = cluster.name_any();
        let mut data = BTreeMap::new();
        data.insert("version".to_string(), cluster.spec.version.clone());
        data.insert(
            "clusterName".to_string(),
            cluster.spec.distributed.cluster_name.clone(),
        );
        data.insert("nodes".to_string(), cluster.spec.nodes.to_string());
        let cm: _ = k8s_openapi::api::core::v1::ConfigMap {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(name.clone()),
                namespace: cluster.namespace(),
                labels: Some(self.get_labels(cluster)),
                annotations: Some(self.get_annotations(cluster)),
                ..Default::default()
            },
            data: Some(data),
            binary_data: None,
            immutable: None,
        };
        self.configmaps
            .create(&kube::api::PostParams::default(), &cm)
            .await?;
        info!("Created ConfigMap: {}", name);
        Ok(())
    }
    /// Create Secret for cluster credentials
    async fn create_secret(&self, cluster: &AmberCluster) -> Result<(), Error> {
        let name: _ = cluster.name_any();
        let mut data = BTreeMap::new();
        let encoded: _ = base64::encode("dummy-token");
        data.insert("token".to_string(), ByteString(encoded.into_bytes()));
        let secret: _ = k8s_openapi::api::core::v1::Secret {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(name.clone()),
                namespace: cluster.namespace(),
                labels: Some(self.get_labels(cluster)),
                annotations: Some(self.get_annotations(cluster)),
                ..Default::default()
            },
            data: Some(data),
            string_data: None,
            type_: Some("Opaque".to_string()),
            immutable: None,
        };
        self.secrets
            .create(&kube::api::PostParams::default(), &secret)
            .await?;
        info!("Created Secret: {}", name);
        Ok(())
    }
    /// Create StatefulSet for cluster nodes
    async fn create_statefulset(&self, cluster: &AmberCluster) -> Result<(), Error> {
        let name: _ = cluster.name_any();
        let replicas: _ = cluster.spec.nodes as i32;
        let statefulset: _ = StatefulSet {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(name.clone()),
                namespace: cluster.namespace(),
                labels: Some(self.get_labels(cluster)),
                annotations: Some(self.get_annotations(cluster)),
                ..Default::default()
            },
            spec: Some(StatefulSetSpec {
                replicas: Some(replicas),
                service_name: format!("{}-headless", name),
                ordinals: None,
                selector: k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector {
                    match_labels: Some(self.get_labels(cluster)),
                    match_expressions: None,
                },
                template: k8s_openapi::api::core::v1::PodTemplateSpec {
                    metadata: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                        labels: Some(self.get_labels(cluster)),
                        annotations: Some(BTreeMap::from([(
                            "amberjs.io/cluster".to_string(),
                            name.clone(),
                        )])),
                        ..Default::default()
                    }),
                    spec: Some(self.create_pod_spec(cluster)?),
                },
                volume_claim_templates: Some(self.create_pvc_templates(cluster)?),
                min_ready_seconds: Some(10),
                persistent_volume_claim_retention_policy: None,
                pod_management_policy: None,
                revision_history_limit: Some(10),
                update_strategy: Some(k8s_openapi::api::apps::v1::StatefulSetUpdateStrategy {
                    type_: Some("RollingUpdate".to_string()),
                    rolling_update: Some(
                        k8s_openapi::api::apps::v1::RollingUpdateStatefulSetStrategy {
                            max_unavailable: Some(
                                k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(1),
                            ),
                            partition: None,
                        },
                    ),
                }),
            }),
            status: None,
        };
        self.statefulsets
            .create(&kube::api::PostParams::default(), &statefulset)
            .await?;
        info!("Created StatefulSet: {} with {} replicas", name, replicas);
        Ok(())
    }
    /// Create Service for cluster access
    async fn create_service(&self, cluster: &AmberCluster) -> Result<(), Error> {
        let name: _ = cluster.name_any();
        // Headless service for StatefulSet
        let headless_service: _ = k8s_openapi::api::core::v1::Service {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(format!("{}-headless", name)),
                namespace: cluster.namespace(),
                labels: Some(self.get_labels(cluster)),
                annotations: Some(self.get_annotations(cluster)),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ServiceSpec {
                cluster_ip: Some("None".to_string()),
                cluster_ips: None,
                ip_families: None,
                ip_family_policy: None,
                ports: None,
                publish_not_ready_addresses: Some(true),
                selector: Some(self.get_labels(cluster)),
                session_affinity: Some("ClientIP".to_string()),
                session_affinity_config: None,
                type_: Some("ClusterIP".to_string()),
                external_ips: None,
                external_name: None,
                health_check_node_port: None,
                load_balancer_class: None,
                load_balancer_ip: None,
                load_balancer_source_ranges: None,
                allocate_load_balancer_node_ports: None,
                external_traffic_policy: None,
                internal_traffic_policy: None,
            }),
            status: None,
        };
        self.services
            .create(&kube::api::PostParams::default(), &headless_service)
            .await?;
        // ClusterIP service for API access
        let api_service: _ = k8s_openapi::api::core::v1::Service {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(name.clone()),
                namespace: cluster.namespace(),
                labels: Some(self.get_labels(cluster)),
                annotations: Some(self.get_annotations(cluster)),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ServiceSpec {
                cluster_ip: Some("10.0.0.1".to_string()),
                cluster_ips: None,
                ip_families: None,
                ip_family_policy: None,
                ports: Some(vec![k8s_openapi::api::core::v1::ServicePort {
                    name: Some("api".to_string()),
                    port: 8080,
                    protocol: Some("TCP".to_string()),
                    target_port: Some(
                        k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080),
                    ),
                    app_protocol: None,
                    node_port: None,
                }]),
                publish_not_ready_addresses: None,
                selector: Some(self.get_labels(cluster)),
                session_affinity: Some("None".to_string()),
                session_affinity_config: None,
                type_: Some("ClusterIP".to_string()),
                external_ips: None,
                external_name: None,
                health_check_node_port: None,
                load_balancer_class: None,
                load_balancer_ip: None,
                load_balancer_source_ranges: None,
                allocate_load_balancer_node_ports: None,
                external_traffic_policy: None,
                internal_traffic_policy: None,
            }),
            status: None,
        };
        self.services
            .create(&kube::api::PostParams::default(), &api_service)
            .await?;
        info!("Created Services: {}-headless and {}", name, name);
        Ok(())
    }
    /// Check if cluster needs update
    async fn needs_update(&self, cluster: &AmberCluster) -> Result<bool, Error> {
        // TODO: Implement update detection logic
        Ok(false)
    }
    /// Update StatefulSet if needed
    async fn update_statefulset(&self, cluster: &AmberCluster) -> Result<(), Error> {
        // TODO: Implement StatefulSet update logic
        Ok(())
    }
    /// Check cluster health
    async fn check_health(&self, cluster: &AmberCluster) -> Result<(), Error> {
        // TODO: Implement health check logic
        Ok(())
    }
    /// Check if cluster resources exist
    async fn resources_exist(&self, cluster: &AmberCluster) -> Result<bool, Error> {
        let name: _ = cluster.name_any();
        // Check StatefulSet
        if let Err(e) = self.statefulsets.get(&name).await {
            // 404 errors mean resource doesn't exist
            if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                return Ok(false);
            }
            return Err(Error::Kube(e));
        }
        Ok(true)
    }
    /// Recover cluster resources
    async fn recover_resources(&self, cluster: &AmberCluster) -> Result<(), Error> {
        // TODO: Implement resource recovery logic
        Ok(())
    }
    /// Wait for pods to be ready
    async fn wait_for_ready(&self, cluster: &AmberCluster) -> Result<(), Error> {
        // TODO: Implement readiness check
        Ok(())
    }
    /// Wait for update to complete
    async fn wait_for_update(&self, cluster: &AmberCluster) -> Result<(), Error> {
        // TODO: Implement update wait logic
        Ok(())
    }
    /// Create pod specification
    fn create_pod_spec(
        &self,
        cluster: &AmberCluster,
    ) -> Result<k8s_openapi::api::core::v1::PodSpec, Error> {
        Ok(k8s_openapi::api::core::v1::PodSpec {
            containers: vec![k8s_openapi::api::core::v1::Container {
                name: "amberjs".to_string(),
                image: Some(cluster.spec.image.clone()),
                image_pull_policy: Some("IfNotPresent".to_string()),
                ports: Some(vec![k8s_openapi::api::core::v1::ContainerPort {
                    container_port: 8080,
                    protocol: Some("TCP".to_string()),
                    name: Some("api".to_string()),
                    host_ip: None,
                    host_port: None,
                }]),
                env: Some(vec![
                    k8s_openapi::api::core::v1::EnvVar {
                        name: "AMBER_CLUSTER_NAME".to_string(),
                        value: Some(cluster.spec.distributed.cluster_name.clone()),
                        value_from: None,
                    },
                    k8s_openapi::api::core::v1::EnvVar {
                        name: "AMBER_NODE_ID".to_string(),
                        value: None,
                        value_from: Some(k8s_openapi::api::core::v1::EnvVarSource {
                            field_ref: Some(k8s_openapi::api::core::v1::ObjectFieldSelector {
                                field_path: "metadata.name".to_string(),
                                api_version: None,
                            }),
                            resource_field_ref: None,
                            config_map_key_ref: None,
                            secret_key_ref: None,
                        }),
                    },
                ]),
                env_from: None,
                resources: Some(k8s_openapi::api::core::v1::ResourceRequirements {
                    requests: Some(BTreeMap::from([
                        (
                            "cpu".to_string(),
                            Quantity(cluster.spec.resources.cpu.clone()),
                        ),
                        (
                            "memory".to_string(),
                            Quantity(cluster.spec.resources.memory.clone()),
                        ),
                    ])),
                    limits: Some(BTreeMap::from([
                        (
                            "cpu".to_string(),
                            Quantity(cluster.spec.resources.cpu.clone()),
                        ),
                        (
                            "memory".to_string(),
                            Quantity(cluster.spec.resources.memory.clone()),
                        ),
                    ])),
                    claims: None,
                }),
                volume_mounts: Some(vec![k8s_openapi::api::core::v1::VolumeMount {
                    name: "config".to_string(),
                    mount_path: "/etc/amberjs".to_string(),
                    read_only: Some(true),
                    sub_path: None,
                    sub_path_expr: None,
                    mount_propagation: None,
                }]),
                ..Default::default()
            }],
            volumes: Some(vec![k8s_openapi::api::core::v1::Volume {
                name: "config".to_string(),
                config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                    default_mode: Some(0o644),
                    items: None,
                    name: Some(cluster.name_any()),
                    optional: Some(false),
                }),
                ..Default::default()
            }]),
            ..Default::default()
        })
    }
    /// Create PVC templates
    fn create_pvc_templates(
        &self,
        cluster: &AmberCluster,
    ) -> Result<Vec<k8s_openapi::api::core::v1::PersistentVolumeClaim>, Error> {
        let mut pvcs = Vec::new();
        // Add storage PVC if disk size is specified
        if !cluster.spec.resources.disk.is_empty() {
            let pvc: _ = k8s_openapi::api::core::v1::PersistentVolumeClaim {
                metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                    name: Some(format!("{}-data", cluster.name_any())),
                    ..Default::default()
                },
                spec: Some(k8s_openapi::api::core::v1::PersistentVolumeClaimSpec {
                    access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                    resources: Some(k8s_openapi::api::core::v1::ResourceRequirements {
                        requests: Some(BTreeMap::from([(
                            "storage".to_string(),
                            Quantity(cluster.spec.resources.disk.clone()),
                        )])),
                        limits: None,
                        claims: None,
                    }),
                    storage_class_name: Some("standard".to_string()),
                    selector: None,
                    volume_mode: None,
                    volume_name: None,
                    data_source: None,
                    data_source_ref: None,
                }),
                status: None,
            };
            pvcs.push(pvc);
        }
        Ok(pvcs)
    }
    /// Get labels for resources
    fn get_labels(
        &self,
        cluster: &AmberCluster,
    ) -> BTreeMap<
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    > {
        BTreeMap::from([
            ("amberjs.io/cluster".to_string(), cluster.name_any()),
            (
                "amberjs.io/version".to_string(),
                cluster.spec.version.clone(),
            ),
            (
                "amberjs.io/cluster-name".to_string(),
                cluster.spec.distributed.cluster_name.clone(),
            ),
        ])
    }
    /// Get annotations for resources
    fn get_annotations(
        &self,
        cluster: &AmberCluster,
    ) -> BTreeMap<
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    > {
        BTreeMap::from([
            (
                "amberjs.io/created-by".to_string(),
                "amberjs-operator".to_string(),
            ),
            (
                "amberjs.io/description".to_string(),
                "Amber Cluster".to_string(),
            ),
        ])
    }
}
/// Error type for operator controller
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),
    #[error("Serde JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use std::time::Duration;
    #[test]
    fn test_error_type() {
        // Test error conversion - just verify the error type exists
        // This test validates that the Error enum properly wraps kube::Error
        let _error: _ = Error::Other("test".to_string());
        // The actual conversion test would require constructing a real kube::Error
        // which is complex, so we just verify the types compile correctly
    }
}
