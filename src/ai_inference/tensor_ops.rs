// 张量操作模块
// 提供高性能的张量计算和操作

use anyhow::Result;
use std::collections::{BTreeMap, HashMap};

/// 张量结构体
#[derive(Debug, Clone)]
pub struct Tensor {
    data: Vec<f32>,
    shape: Vec<usize>,
}
impl Tensor {
    /// 创建新的张量
    pub fn new(data: Vec<f32>, shape: Vec<usize>) -> Result<Self> {
        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(anyhow::anyhow!(
                "Data size mismatch: expected {}, got {}",
                expected_size,
                data.len()
            ));
        }
        Ok(Tensor { data, shape })
    }
    /// 创建新的张量（别名方法）
    pub fn new_with_data(data: Vec<f32>, shape: Vec<usize>) -> Result<Self> {
        Self::new(data, shape)
    }
    /// 获取张量形状
    pub fn shape(&self) -> &Vec<usize> {
        &self.shape
    }
    /// 获取张量数据
    pub fn data(&self) -> &Vec<f32> {
        &self.data
    }
    /// 获取张量维度数
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }
    /// 获取张量元素总数
    pub fn size(&self) -> usize {
        self.data.len()
    }
    /// 获取指定索引的值
    pub fn get(&self, indices: &[usize]) -> Result<f32> {
        if indices.len() != self.ndim() {
            return Err(anyhow::anyhow!("Invalid number of indices"));
        }
        let mut index = 0;
        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return Err(anyhow::anyhow!("Index out of bounds"));
            }
            let stride: usize = self.shape[i + 1..].iter().product();
            index += idx * stride;
        }
        Ok(self.data[index])
    }
    /// 设置指定索引的值
    pub fn set(&mut self, indices: &[usize], value: f32) -> Result<()> {
        if indices.len() != self.ndim() {
            return Err(anyhow::anyhow!("Invalid number of indices"));
        }
        let mut index = 0;
        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return Err(anyhow::anyhow!("Index out of bounds"));
            }
            let stride: usize = self.shape[i + 1..].iter().product();
            index += idx * stride;
        }
        self.data[index] = value;
        Ok(())
    }
    /// 重塑张量
    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Tensor> {
        let new_size: usize = new_shape.iter().product();
        if new_size != self.size() {
            return Err(anyhow::anyhow!(
                "Cannot reshape: size mismatch ({} vs {})",
                new_size,
                self.size()
            ));
        }
        Ok(Tensor {
            data: self.data.clone(),
            shape: new_shape,
        })
    }
    /// 矩阵乘法
    pub fn matmul(&self, other: &Tensor) -> Result<Tensor> {
        if self.ndim() != 2 || other.ndim() != 2 {
            return Err(anyhow::anyhow!("matmul requires 2D tensors"));
        }
        let (m, k) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        if k != k2 {
            return Err(anyhow::anyhow!(
                "matmul dimension mismatch: {}x{} * {}x{}",
                m,
                k,
                k2,
                n
            ));
        }
        let mut result = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0;
                for x in 0..k {
                    sum += self.data[i * k + x] * other.data[x * n + j];
                }
                result[i * n + j] = sum;
            }
        }
        Ok(Tensor::new(result, vec![m, n])?)
    }
    /// 元素级加法
    pub fn add(&self, other: &Tensor) -> Result<Tensor> {
        if self.shape != other.shape {
            return Err(anyhow::anyhow!("Shape mismatch in add"));
        }
        let mut result = Vec::with_capacity(self.size());
        for (a, b) in self.data.iter().zip(other.data.iter()) {
            result.push(a + b);
        }
        Ok(Tensor::new(result, self.shape.clone())?)
    }
    /// 元素级乘法
    pub fn mul(&self, other: &Tensor) -> Result<Tensor> {
        if self.shape != other.shape {
            return Err(anyhow::anyhow!("Shape mismatch in mul"));
        }
        let mut result = Vec::with_capacity(self.size());
        for (a, b) in self.data.iter().zip(other.data.iter()) {
            result.push(a * b);
        }
        Ok(Tensor::new(result, self.shape.clone())?)
    }
    /// ReLU 激活函数
    pub fn relu(&self) -> Tensor {
        let data: Vec<f32> = self.data.iter().map(|&x| x.max(0.0)).collect();
        Tensor {
            data,
            shape: self.shape.clone(),
        }
    }
    /// Softmax 激活函数
    pub fn softmax(&self) -> Result<Tensor> {
        if self.ndim() != 1 {
            return Err(anyhow::anyhow!("softmax requires 1D tensor"));
        }
        let max_val: _ = self.data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exp_sum: f32 = self.data.iter().map(|&x| (x - max_val).exp()).sum();
        if exp_sum == 0.0 {
            return Err(anyhow::anyhow!("Invalid softmax input"));
        }
        let data: Vec<f32> = self
            .data
            .iter()
            .map(|&x| (x - max_val).exp() / exp_sum)
            .collect();
        Ok(Tensor {
            data,
            shape: self.shape.clone(),
        })
    }
    /// 平均池化
    pub fn avg_pool(&self, kernel_size: usize, stride: usize) -> Result<Tensor> {
        if self.ndim() != 4 {
            return Err(anyhow::anyhow!("avg_pool requires 4D tensor (N, C, H, W)"));
        }
        let (batch, channels, height, width) =
            (self.shape[0], self.shape[1], self.shape[2], self.shape[3]);
        let out_height: _ = (height - kernel_size) / stride + 1;
        let out_width: _ = (width - kernel_size) / stride + 1;
        let mut result = Vec::new();
        result.resize(batch * channels * out_height * out_width, 0.0);
        for b in 0..batch {
            for c in 0..channels {
                for oh in 0..out_height {
                    for ow in 0..out_width {
                        let mut sum = 0.0;
                        for kh in 0..kernel_size {
                            for kw in 0..kernel_size {
                                let h: _ = oh * stride + kh;
                                let w: _ = ow * stride + kw;
                                let input_idx: _ = ((b * channels + c) * height + h) * width + w;
                                sum += self.data[input_idx];
                            }
                        }
                        let output_idx: _ = ((b * channels + c) * out_height + oh) * out_width + ow;
                        result[output_idx] = sum / (kernel_size * kernel_size) as f32;
                    }
                }
            }
        }
        Ok(Tensor::new(
            result,
            vec![batch, channels, out_height, out_width],
        )?)
    }
    /// 转换为字符串表示
    pub fn to_string(&self) -> String {
        format!("Tensor(shape={:?}, data={:?})", self.shape, self.data)
    }
    /// 转换为 PyTorch 张量
    #[cfg(feature = "tch")]
    pub fn to_tch_tensor(&self, device: &tch::Device) -> Result<tch::Tensor> {
        let tensor: _ = tch::Tensor::from(&self.data[..])
            .reshape(self.shape.as_slice())
            .to_device(device);
        Ok(tensor)
    }
    /// 从 PyTorch 张量创建
    #[cfg(feature = "tch")]
    pub fn from_tch_tensor(tch_tensor: tch::Tensor, _device: &tch::Device) -> Result<Self> {
        // 获取张量数据和形状
        let data_vec: Vec<f32> = tch_tensor
            .to_kind(tch::Kind::Float)
            .try_into()
            .context("Failed to convert PyTorch tensor to Vec<f32>")?;
        let shape: Vec<usize> = tch_tensor.dims().iter().map(|&d| d as usize).collect();
        Ok(Tensor {
            data: data_vec,
            shape,
        })
    }
    /// 转换为 PyTorch 张量（未启用功能时的占位符）
    #[cfg(not(feature = "tch"))]
    pub fn to_tch_tensor(&self, _device: &()) -> Result<Box<dyn std::fmt::Debug>> {
        Err(anyhow::anyhow!(
            "PyTorch support not enabled. Enable with --features tch"
        ))
    }
    /// 从 PyTorch 张量创建（未启用功能时的占位符）
    #[cfg(not(feature = "tch"))]
    pub fn from_tch_tensor(_tch_tensor: Box<dyn std::fmt::Debug>, _device: &()) -> Result<Self> {
        Err(anyhow::anyhow!(
            "PyTorch support not enabled. Enable with --features tch"
        ))
    }
}
impl fmt::Display for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
/// 张量操作工具函数
pub struct TensorOps;
impl TensorOps {
    /// 创建零张量
    pub fn zeros(shape: Vec<usize>) -> Result<Tensor> {
        let size: usize = shape.iter().product();
        let data: _ = vec![0.0; size];
        Tensor::new(data, shape)
    }
    /// 创建一张量
    pub fn ones(shape: Vec<usize>) -> Result<Tensor> {
        let size: usize = shape.iter().product();
        let data: _ = vec![1.0; size];
        Tensor::new(data, shape)
    }
    /// 创建随机张量
    pub fn random(shape: Vec<usize>) -> Result<Tensor> {
        let size: usize = shape.iter().product();
        let mut rng = rand::rng();
        let data: Vec<f32> = (0..size).map(|_| rng.random::<f32>()).collect();
        Tensor::new(data, shape)
    }
    /// 创建单位矩阵
    pub fn eye(n: usize) -> Result<Tensor> {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Tensor::new(data, vec![n, n])
    }
    /// 连接张量
    pub fn concat(tensors: &[Tensor], axis: usize) -> Result<Tensor> {
        if tensors.is_empty() {
            return Err(anyhow::anyhow!("Cannot concat empty tensors"));
        }
        let first_shape: _ = &tensors[0].shape;
        let ndim: _ = first_shape.len();
        if axis >= ndim {
            return Err(anyhow::anyhow!("Axis out of bounds"));
        }
        // 检查所有张量的形状（除了连接轴）
        for tensor in tensors.iter() {
            if tensor.ndim() != ndim {
                return Err(anyhow::anyhow!(
                    "All tensors must have same number of dimensions"
                ));
            }
            for (i, (&dim1, &dim2)) in first_shape.iter().zip(tensor.shape.iter()).enumerate() {
                if i != axis && dim1 != dim2 {
                    return Err(anyhow::anyhow!(
                        "Shape mismatch on axis {} (expected {}, got {})",
                        i,
                        dim1,
                        dim2
                    ));
                }
            }
        }
        // 计算输出形状
        let mut output_shape = first_shape.clone();
        output_shape[axis] = tensors.iter().map(|t| t.shape[axis]).sum();
        // 连接数据
        let mut output_data = Vec::new();
        for tensor in tensors {
            output_data.extend_from_slice(&tensor.data);
        }
        Tensor::new(output_data, output_shape)
    }
    /// 分割张量
    pub fn split(tensor: &Tensor, sections: Vec<usize>, axis: usize) -> Result<Vec<Tensor>> {
        let ndim: _ = tensor.ndim();
        if axis >= ndim {
            return Err(anyhow::anyhow!("Axis out of bounds"));
        }
        let total_size: _ = tensor.shape[axis];
        let sum_sections: usize = sections.iter().sum();
        if sum_sections != total_size {
            return Err(anyhow::anyhow!(
                "Sections sum {} does not equal dimension size {}",
                sum_sections,
                total_size
            ));
        }
        let result: _ = Vec::new();
        let _start: _ = 0;
        for &section_size in &sections {
            // 这里需要实现复杂的切片逻辑
            // 为了简化，这里只返回错误提示
            return Err(anyhow::anyhow!(
                "Split implementation requires complex slicing"
            ));
        }
        Ok(result)
    }
}
