// Amber 包管理器模块
// Stage 80 Phase 1 - 包管理器核心功能
pub mod dependency_resolver;
pub mod cache_manager;
pub mod version_selector;
pub use dependency_resolver::*;
pub use cache_manager::*;
pub use version_selector::*;
use std::collections::{HashMap, BTreeMap};