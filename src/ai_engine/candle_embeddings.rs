//! Beejs Native Embeddings & Vector Operations powered by Candle
//!
//! Provides dense, normalized vector embeddings and SIMD/GPU accelerated similarity.

use candle_core::Tensor;

use crate::ai_engine::candle_engine::get_device;
use crate::ai_engine::embedding::{embed_text, EmbedOptions};

/// Computes embedding vector using native pipeline with optional dimension scaling
pub fn candle_embed(text: &str, options: &EmbedOptions) -> Vec<f32> {
    embed_text(text, options)
}

/// Computes cosine similarity between two embedding vectors using Candle
pub fn candle_cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dev = get_device(None);
    if let (Ok(t_a), Ok(t_b)) = (
        Tensor::from_slice(a, a.len(), &dev),
        Tensor::from_slice(b, b.len(), &dev),
    ) {
        if let (Ok(dot), Ok(norm_a_sq), Ok(norm_b_sq)) = (
            (&t_a * &t_b).and_then(|t| t.sum_all()?.to_scalar::<f32>()),
            t_a.sqr().and_then(|t| t.sum_all()?.to_scalar::<f32>()),
            t_b.sqr().and_then(|t| t.sum_all()?.to_scalar::<f32>()),
        ) {
            let denom = (norm_a_sq * norm_b_sq).sqrt();
            if denom > 0.0 {
                return dot / denom;
            }
        }
    }
    crate::ai_engine::embedding::cosine_similarity(a, b)
}

/// Computes top-K most similar vectors from a candidate pool
pub fn candle_top_k_similar(
    query: &[f32],
    candidates: &[Vec<f32>],
    top_k: usize,
) -> Vec<(usize, f32)> {
    let mut scores: Vec<(usize, f32)> = candidates
        .iter()
        .enumerate()
        .map(|(idx, cand)| (idx, candle_cosine_similarity(query, cand)))
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);
    scores
}
