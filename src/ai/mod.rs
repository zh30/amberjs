//! AI workload helpers (feature `ai`).
//!
//! These modules are experimental. Built-in model inference is opt-in and must
//! not be treated as production-ready LLM serving. Prefer running real AI SDKs
//! (OpenAI/Anthropic/etc.) on Amber' Node-compatible I/O path.

pub mod acceleration_engine;
pub mod ai_async_queue;
pub mod ai_batch_processor;
pub mod ai_memory_pool;
pub mod ai_performance_engine;
pub mod auto_optimizer;
pub mod code_generator;
pub mod code_optimizer;
pub mod intelligent_scheduler;
pub mod llm_engine;
pub mod matrix_accelerator;
pub mod model_cache;
pub mod model_interface;
pub mod model_manager;
pub mod performance_predictor;
pub mod predictive_scaler;
pub mod smart_debugger;
pub mod tensor_optimizer;
