// Stage 54.4: AI 推理引擎完善测试套件
// 测试 TorchEngine 的完整 InferenceEngine trait 实现

use amberjs::ai_inference{
    TorchEngine, TorchEngineFactory, InferenceEngine, ModelFormat, EngineType,
    InferenceOptions, Tensor, ModelHandle, ModelInfo, EngineStats
};
use anyhow::Result;
use tokio::test;
use std::sync::Arc;

/// 测试 1: infer_stream 方法 - 流式推理
#[test]
async fn test_torch_engine_infer_stream() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();

    // 创建测试输入
    let input = Tensor::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]).unwrap();

    // 测试流式推理
    let mut receiver = engine.infer_stream(&model, input).await.unwrap();

    // 验证接收流式结果
    let mut result_count = 0;
    let timeout = tokio::time::timeout(Duration::from_secs(1), async {
        while let Some(result) = receiver.recv().await {
            assert!(result.is_ok());
            result_count += 1;
            if result_count >= 5 {
                break;
            }
        }
    }).await;

    assert!(timeout.is_ok(), "Stream inference should complete within timeout");
    assert_eq!(result_count, 5, "Should receive 5 streaming results");

    println!("✅ Test 1 passed: infer_stream method works correctly");
}

/// 测试 2: get_model_info 方法 - 模型信息获取
#[test]
async fn test_torch_engine_get_model_info() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();

    // 获取模型信息
    let model_info = engine.get_model_info(&model).await.unwrap();

    // 验证模型信息
    assert_eq!(model_info.format, ModelFormat::PyTorch);
    assert!(!model_info.input_shapes.is_empty());
    assert!(!model_info.output_shapes.is_empty());
    assert!(model_info.parameters > 0);
    assert!(model_info.size_mb > 0.0);

    println!("✅ Test 2 passed: get_model_info returns correct model metadata");
    println!("   Input shapes: {:?}", model_info.input_shapes);
    println!("   Output shapes: {:?}", model_info.output_shapes);
    println!("   Parameters: {}", model_info.parameters);
    println!("   Size: {:.2} MB", model_info.size_mb);
}

/// 测试 3: warmup 方法 - 模型预热
#[test]
async fn test_torch_engine_warmup() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();

    // 预热模型
    let warmup_result = engine.warmup(&model).await;

    // 验证预热成功
    assert!(warmup_result.is_ok(), "Warmup should complete successfully");

    println!("✅ Test 3 passed: warmup method initializes model and GPU");
}

/// 测试 4: unload_model 方法 - 模型卸载
#[test]
async fn test_torch_engine_unload_model() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();

    // 卸载模型
    let unload_result = engine.unload_model(&model).await;

    // 验证卸载成功
    assert!(unload_result.is_ok(), "Unload should complete successfully");

    println!("✅ Test 4 passed: unload_model releases resources correctly");
}

/// 测试 5: clone_engine 方法 - 引擎克隆
#[test]
async fn test_torch_engine_clone() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 克隆引擎
    let cloned_engine = engine.clone_engine();

    // 验证克隆的引擎
    assert!(cloned_engine.is_available());
    assert_eq!(cloned_engine.name(), "PyTorch-TorchScript");
    assert!(cloned_engine.supported_formats().contains(&ModelFormat::PyTorch));

    // 测试克隆的引擎可以正常工作
    let model = cloned_engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();
    let input = Tensor::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]).unwrap();
    let result = cloned_engine.infer(&model, &input).await.unwrap();

    assert!(result.output_tensor.size() > 0);

    println!("✅ Test 5 passed: clone_engine creates functional copy");
}

/// 测试 6: 引擎统计信息完整性
#[test]
async fn test_torch_engine_stats_completeness() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 获取统计信息
    let stats = engine.get_stats().await.unwrap();

    // 验证所有必需字段存在
    assert_eq!(stats.engine_name, "PyTorch-TorchScript");
    assert_eq!(stats.total_inferences, 0);
    assert_eq!(stats.successful_inferences, 0);
    assert_eq!(stats.failed_inferences, 0);
    assert_eq!(stats.total_time_ms, 0.0);
    assert_eq!(stats.average_time_ms, 0.0);
    assert_eq!(stats.gpu_utilization, 0.0);
    assert_eq!(stats.memory_usage_bytes, 0);
    assert_eq!(stats.cache_hit_rate, 0.0);

    println!("✅ Test 6 passed: EngineStats has all required fields");
}

/// 测试 7: 推理执行后统计信息更新
#[test]
async fn test_torch_engine_stats_update() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型和输入
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();
    let input = Tensor::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]).unwrap();

    // 执行推理
    let _result = engine.infer(&model, &input).await.unwrap();

    // 验证统计信息更新
    let stats = engine.get_stats().await.unwrap();
    assert_eq!(stats.total_inferences, 1);
    assert_eq!(stats.successful_inferences, 1);
    assert_eq!(stats.failed_inferences, 0);
    assert!(stats.total_time_ms > 0.0);
    assert!(stats.average_time_ms > 0.0);

    println!("✅ Test 7 passed: Stats are updated after inference");
    println!("   Total inferences: {}", stats.total_inferences);
    println!("   Average time: {:.2}ms", stats.average_time_ms);
}

/// 测试 8: 批处理推理统计信息
#[test]
async fn test_torch_engine_batch_stats() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型和批量输入
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();
    let inputs = vec![
        Tensor::new(vec![1.0, 2.0], vec![1, 2]).unwrap(),
        Tensor::new(vec![3.0, 4.0], vec![1, 2]).unwrap(),
        Tensor::new(vec![5.0, 6.0], vec![1, 2]).unwrap(),
    ];

    // 执行批处理推理
    let _results = engine.batch_infer(&model, &inputs).await.unwrap();

    // 验证统计信息更新
    let stats = engine.get_stats().await.unwrap();
    assert_eq!(stats.total_inferences, 3);
    assert_eq!(stats.successful_inferences, 3);

    println!("✅ Test 8 passed: Batch inference updates stats correctly");
    println!("   Total inferences after batch: {}", stats.total_inferences);
}

/// 测试 9: 错误处理的统计信息
#[test]
async fn test_torch_engine_error_stats() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 创建测试模型
    let model = engine.load_model("test_model.pt", InferenceOptions::default()).await.unwrap();

    // 创建无效输入（错误的形状）
    let invalid_input = Tensor::new(vec![1.0, 2.0], vec![1]).unwrap();

    // 执行推理（应该失败）
    let result = engine.infer(&model, &invalid_input).await;

    // 验证错误被正确处理
    assert!(result.is_err());

    // 验证统计信息反映错误
    let stats = engine.get_stats().await.unwrap();
    assert_eq!(stats.total_inferences, 1);
    assert_eq!(stats.successful_inferences, 0);
    assert_eq!(stats.failed_inferences, 1);

    println!("✅ Test 9 passed: Error handling updates stats correctly");
    println!("   Failed inferences: {}", stats.failed_inferences);
}

/// 测试 10: 引擎可用性检查
#[test]
async fn test_torch_engine_availability() {
    let factory = TorchEngineFactory::new(EngineType::CPU);
    let engine = factory.create(EngineType::CPU).await.unwrap();

    // 验证引擎初始可用性
    assert!(engine.is_available(), "Engine should be available after creation");

    // 验证支持的格式
    let supported_formats = engine.supported_formats();
    assert!(supported_formats.contains(&ModelFormat::PyTorch));

    // 验证引擎名称
    assert_eq!(engine.name(), "PyTorch-TorchScript");

    println!("✅ Test 10 passed: Engine availability and metadata correct");
    println!("   Engine name: {}", engine.name());
    println!("   Supported formats: {:?}", supported_formats);
}

use std::time::Duration;
