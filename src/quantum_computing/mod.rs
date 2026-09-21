// Amber 量子计算模块
//
// 提供量子计算模拟和混合计算能力：
// - 量子比特 (Qubit) 模拟
// - 量子门 (Quantum Gates) 操作
// - 量子电路 (Quantum Circuit) 构建与执行
// - 量子优化器 (Quantum Optimizer)
// - 经典-量子混合计算
mod qubit;
mod gates;
mod circuit;
mod simulator;
mod optimizer;
mod hybrid;

use circuit::{CircuitResult, QuantumCircuit};
use hybrid::{HybridComputing, QaoaResult, VariationalResult};
use qubit::{Qubit, QubitState};
use std::collections::{BTreeMap, HashMap};

/// 复数类型别名
pub type Complex64 = num_complex::Complex64;