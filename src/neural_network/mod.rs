// Amber 神经网络模块
//
// 高性能神经网络推理引擎：
// - 张量 (Tensor) 运算
// - 神经网络层 (Dense, Conv, Activation)
// - 模型构建与推理
// - 计算图优化
// - 硬件感知优化
mod tensor;
mod layers;
mod model;
mod optimizer;
mod hardware;

use hardware::{HardwareBackend, MemoryInfo};
use layers::{ActivationLayer, ActivationType, ConvLayer, DenseLayer, Layer};
use model::{Model, ModelConfig};
use optimizer::{GraphOptimizer, OptimizationLevel};
use std::collections::{BTreeMap, HashMap};
use tensor::{DType, Tensor, TensorShape};
