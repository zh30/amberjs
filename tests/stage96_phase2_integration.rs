// Stage 96 Phase 2 Integration Tests
// Tests the integration of Kubernetes Operator, Multi-tenancy, and Monitoring

#![cfg(feature = "enterprise")]

use amberjs::enterprise::{
    self, ClusterMetrics, MonitoringConfig, MonitoringManager, Operator, OperatorConfig,
    OperatorEvent, ResourceQuota, TenancyManager, TenantId, TenantStatus,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};

#[tokio::test]
async fn test_enterprise_integration() {
    // Setup enterprise components
    let operator_config = OperatorConfig {
        namespace: "default".to_string(),
        reconcile_interval: std::time::Duration::from_secs(30),
        max_retries: 3,
    };

    let monitoring_config = MonitoringConfig {
        metrics_retention_hours: 24,
        metrics_collection_interval_seconds: 30,
        alerts_enabled: true,
        prometheus_endpoint: None,
        grafana_endpoint: None,
    };

    let (operator, _operator_receiver) = Operator::new(operator_config);
    let tenancy_manager = TenancyManager::new();
    let monitoring_manager = MonitoringManager::new(monitoring_config);

    // Test 1: Create a tenant
    let resource_quota = ResourceQuota {
        max_clusters: 5,
        max_replicas_per_cluster: 3,
        max_memory_mb: 4096,
        max_cpu_cores: 2.0,
        max_storage_gb: 50,
        max_concurrent_executions: 10,
    };

    let tenant_id = tenancy_manager
        .create_tenant(
            "enterprise-tenant".to_string(),
            "enterprise@example.com".to_string(),
            resource_quota,
        )
        .await
        .expect("Failed to create tenant");

    println!("Created tenant: {} ({})", tenant_id.0, "enterprise-tenant");

    // Verify tenant was created
    let tenant = tenancy_manager.get_tenant(&tenant_id).await.unwrap();
    assert_eq!(tenant.name, "enterprise-tenant");
    assert_eq!(tenant.status, TenantStatus::Active);

    // Test 2: Create execution context for the tenant
    let execution_context = tenancy_manager
        .create_execution_context(&tenant_id, "test-cluster".to_string())
        .await
        .expect("Failed to create execution context");

    println!(
        "Created execution context: {} for cluster: {}",
        execution_context.execution_id, execution_context.cluster_name
    );

    // Test 3: Start monitoring manager
    monitoring_manager
        .start()
        .await
        .expect("Failed to start monitoring manager");

    // Test 4: Record cluster metrics
    let cluster_metrics = ClusterMetrics {
        cluster_name: "test-cluster".to_string(),
        namespace: "default".to_string(),
        tenant_id: Some(tenant_id.0.clone()),
        cpu_usage: 50.0,
        memory_usage_mb: 1024,
        memory_limit_mb: 2048,
        cpu_limit_cores: 2.0,
        replicas: 3,
        ready_replicas: 3,
        restart_count: 0,
        uptime_seconds: 3600,
        request_count: 1000,
        error_count: 5,
        average_response_time_ms: 100.0,
        timestamp: chrono::Utc::now(),
    };

    monitoring_manager
        .record_cluster_metrics(cluster_metrics.clone())
        .await;

    println!(
        "Recorded cluster metrics for: {}/{}",
        cluster_metrics.namespace, cluster_metrics.cluster_name
    );

    // Test 5: Create a BeejsCluster through the operator
    let cluster = enterprise::BeejsCluster {
        api_version: "v1".to_string(),
        kind: "BeejsCluster".to_string(),
        metadata: enterprise::ObjectMeta {
            name: "enterprise-cluster".to_string(),
            namespace: "default".to_string(),
            labels: Some({
                let mut labels = std::collections::HashMap::new();
                labels.insert("tenant-id".to_string(), tenant_id.0.clone());
                labels.insert("managed-by".to_string(), "beejs-operator".to_string());
                labels
            }),
        },
        spec: enterprise::BeejsClusterSpec {
            replicas: 3,
            version: "v0.1.0".to_string(),
            image: Some("beejs:v0.1.0".to_string()),
            resources: enterprise::ResourceRequirements {
                cpu: Some("500m".to_string()),
                memory: Some("1Gi".to_string()),
                storage: Some("10Gi".to_string()),
            },
            networking: enterprise::NetworkingConfig {
                service_type: enterprise::ServiceType::ClusterIP,
                port: 8080,
                ingress: Some(enterprise::IngressConfig {
                    enabled: true,
                    host: Some("beejs.example.com".to_string()),
                    tls_enabled: true,
                }),
            },
        },
        status: None,
    };

    operator
        .create_cluster(cluster)
        .await
        .expect("Failed to create cluster");

    println!("Created BeejsCluster: enterprise-cluster");

    // Test 6: Verify integration
    let clusters = operator.list_clusters().await.unwrap();
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].metadata.name, "enterprise-cluster");

    let tenant_metrics = monitoring_manager
        .get_tenant_metrics(&tenant_id.0, Some(1))
        .await
        .unwrap();
    assert!(!tenant_metrics.is_empty());

    let cluster_metrics_list = monitoring_manager
        .get_cluster_metrics("test-cluster", "default", Some(1))
        .await
        .unwrap();
    assert_eq!(cluster_metrics_list.len(), 1);

    // Test 7: Verify resource usage tracking
    let resource_usage = tenancy_manager
        .get_resource_usage(&tenant_id)
        .await
        .unwrap();
    assert_eq!(resource_usage.concurrent_executions, 1);

    // Test 8: Export Prometheus metrics
    let prometheus_metrics = monitoring_manager
        .export_prometheus_metrics()
        .await
        .unwrap();

    assert!(prometheus_metrics.contains("beejs_requests_total"));
    println!(
        "Exported Prometheus metrics ({} bytes)",
        prometheus_metrics.len()
    );

    // Test 9: Cleanup
    tenancy_manager
        .cleanup_execution(&execution_context.execution_id)
        .await
        .expect("Failed to cleanup execution");

    operator
        .delete_cluster("enterprise-cluster".to_string(), "default".to_string())
        .await
        .expect("Failed to delete cluster");

    println!("Cleaned up all resources");

    // Final verification
    let clusters = operator.list_clusters().await.unwrap();
    assert_eq!(clusters.len(), 0);

    let resource_usage = tenancy_manager
        .get_resource_usage(&tenant_id)
        .await
        .unwrap();
    assert_eq!(resource_usage.concurrent_executions, 0);

    println!("All integration tests passed!");
}

#[tokio::test]
async fn test_multi_tenant_isolation() {
    let tenancy_manager = TenancyManager::new();

    // Create multiple tenants
    let resource_quota = ResourceQuota {
        max_clusters: 2,
        max_replicas_per_cluster: 2,
        max_memory_mb: 2048,
        max_cpu_cores: 1.0,
        max_storage_gb: 20,
        max_concurrent_executions: 5,
    };

    let tenant1_id = tenancy_manager
        .create_tenant(
            "tenant-1".to_string(),
            "tenant1@example.com".to_string(),
            resource_quota.clone(),
        )
        .await
        .unwrap();

    let tenant2_id = tenancy_manager
        .create_tenant(
            "tenant-2".to_string(),
            "tenant2@example.com".to_string(),
            resource_quota.clone(),
        )
        .await
        .unwrap();

    // Verify tenants are isolated
    let tenant1 = tenancy_manager.get_tenant(&tenant1_id).await.unwrap();
    let tenant2 = tenancy_manager.get_tenant(&tenant2_id).await.unwrap();

    assert_ne!(tenant1.id.0, tenant2.id.0);
    assert_eq!(tenant1.name, "tenant-1");
    assert_eq!(tenant2.name, "tenant-2");

    // Create execution contexts for both tenants
    let context1 = tenancy_manager
        .create_execution_context(&tenant1_id, "cluster-1".to_string())
        .await
        .unwrap();

    let context2 = tenancy_manager
        .create_execution_context(&tenant2_id, "cluster-2".to_string())
        .await
        .unwrap();

    // Verify isolation - different execution contexts
    assert_ne!(context1.execution_id, context2.execution_id);
    assert_ne!(context1.tenant_id.0, context2.tenant_id.0);

    // Verify resource usage is tracked separately
    let usage1 = tenancy_manager
        .get_resource_usage(&tenant1_id)
        .await
        .unwrap();
    let usage2 = tenancy_manager
        .get_resource_usage(&tenant2_id)
        .await
        .unwrap();

    assert_eq!(usage1.concurrent_executions, 1);
    assert_eq!(usage2.concurrent_executions, 1);

    // Cleanup
    tenancy_manager
        .cleanup_execution(&context1.execution_id)
        .await
        .unwrap();
    tenancy_manager
        .cleanup_execution(&context2.execution_id)
        .await
        .unwrap();

    println!("Multi-tenant isolation test passed!");
}
