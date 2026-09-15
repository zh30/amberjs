//! Beejs AI Engine - Native embeddings, tensor ops, and local inference

pub mod candle_embeddings;
pub mod candle_engine;
pub mod embedding;
pub mod generator;

pub use candle_embeddings::{candle_cosine_similarity, candle_embed, candle_top_k_similar};
pub use candle_engine::{
    candle_dot, candle_matmul, candle_norm, candle_softmax, get_device, get_model, CandleModel,
};
pub use embedding::{
    cosine_similarity, embed_batch, embed_text, EmbedOptions, DEFAULT_EMBEDDING_DIM,
};
pub use generator::{EdgeGenerator, GenerateOptions, GenerateResult};
