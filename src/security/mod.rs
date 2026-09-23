// Amber 企业级安全与合规模块
//
// 该模块提供完整的安全与合规体系，包括：
// - 零信任架构（身份验证、权限控制）
// - 数据加密（传输加密、存储加密）
// - 合规自动化（策略检查、风险评估）
// - 审计追踪（审计日志、事件响应）
pub mod audit;
pub mod authentication;
pub mod authorization;
pub mod compliance;
pub mod encryption;
pub mod incident_response;
pub mod risk_assessment;
pub mod tls;
// 重新导出主要类型
pub use audit::*;
pub use authentication::*;
pub use authorization::*;
pub use compliance::*;
pub use encryption::*;
use std::collections::{BTreeMap, HashMap};
