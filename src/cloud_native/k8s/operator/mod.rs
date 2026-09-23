// Kubernetes Operator Controller Module
// Provides reconciliation logic for AmberCluster and AmberWorkload resources
pub mod controller;
pub mod lifecycle;
pub mod reconciler;
// Re-export types for convenience

use controller::{ClusterController, Error as ControllerError};
use lifecycle::{ClusterLifecycle, LifecycleError, WorkloadLifecycle};
use reconciler::{ClusterDiff, ClusterState, ReconcileResult, WorkloadDiff, WorkloadState};
use std::collections::{BTreeMap, HashMap};

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_structure() {
        // Verify that the module structure is correct
        let _controller: Option<ClusterController> = None;
        let _lifecycle: Option<ClusterLifecycle> = None;
        let _reconciler: Option<Reconciler> = None;
    }
}
