// AI 驱动运维 (AIOps) 模块
//
// 这个模块提供了 Amber 的智能故障预测、自动根因分析、智能告警降噪和自动化修复功能。
pub mod allocation;
pub mod anomaly_detection;
pub mod core;
pub mod optimization;
pub mod prediction;
pub mod prediction_engine;
pub mod root_cause_analysis;

// Stage 95: AI Ops subsystem re-exports.
pub use allocation::{LoadBalancer, ResourceOptimizer, Scheduler};
pub use anomaly_detection::{
    Anomaly, AnomalyDetector, AnomalyType, Baseline, BaselineCalculator, MLAnomalyDetector,
    StatisticalAnomalyDetector,
};
pub use core::{AIOpsEngine, AIOpsError, DataCollector, ModelManager, Result};
pub use optimization::{AutoTuner as NewAutoTuner, Optimizer, PerformanceAnalyzer};
pub use prediction::{AnomalyDetector as NewAnomalyDetector, FailurePredictor, TrendAnalyzer};
pub use prediction_engine::{
    MetricType, Prediction, PredictionEngine, SystemMetric, TrendDirection, TrendReport,
};
pub use root_cause_analysis::{
    Change, ChangeImpactAnalysis, ChangeType, Incident, IncidentType, RootCauseAnalysis,
    RootCauseAnalyzer,
};
