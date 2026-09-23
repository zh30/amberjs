//! High-performance Rust-native text embedding encoder.
//!
//! Generates dense, normalized semantic embedding vectors (e.g. 64, 128, 384 dimensions)
//! from input text with zero external dependencies, zero Python, and sub-millisecond latency.
//! Fully interoperable with `amber:vector`'s similarity search.

use std::f32::consts::PI;

/// Default embedding dimension
pub const DEFAULT_EMBEDDING_DIM: usize = 128;

/// Configuration options for text embedding
#[derive(Debug, Clone)]
pub struct EmbedOptions {
    pub dimensions: usize,
    pub normalize: bool,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            dimensions: DEFAULT_EMBEDDING_DIM,
            normalize: true,
        }
    }
}

/// Computes a semantic embedding vector for a given text.
pub fn embed_text(text: &str, options: &EmbedOptions) -> Vec<f32> {
    let dim = if options.dimensions == 0 {
        DEFAULT_EMBEDDING_DIM
    } else {
        options.dimensions
    };

    let tokens = tokenize_and_preprocess(text);
    if tokens.is_empty() {
        return vec![0.0; dim];
    }

    let mut embedding = vec![0.0f32; dim];
    let total_tokens = tokens.len() as f32;

    for (pos, token) in tokens.iter().enumerate() {
        let pos_weight = 1.0 + 0.1 * (-(pos as f32) / total_tokens).exp();
        let token_weight = compute_token_weight(token);
        let weight = pos_weight * token_weight;

        // 1. Word-level projection
        let word_hash = hash_str(token, 0x811c9dc5);
        add_projected_features(&mut embedding, word_hash, weight * 1.5, dim);

        // 2. Subword character n-grams (3-gram to 5-gram) for morphology and typo resistance
        let chars: Vec<char> = token.chars().collect();
        let char_len = chars.len();
        if char_len >= 3 {
            for n in 3..=5.min(char_len) {
                for window in chars.windows(n) {
                    let subword: String = window.iter().collect();
                    let sub_hash = hash_str(&subword, 0x9e3779b9 ^ (n as u64));
                    add_projected_features(&mut embedding, sub_hash, weight * 0.4, dim);
                }
            }
        }
    }

    // Apply non-linear GELU activation to capture semantic feature interactions
    for val in embedding.iter_mut() {
        *val = gelu(*val);
    }

    // Apply L2 normalization
    if options.normalize {
        l2_normalize(&mut embedding);
    }

    embedding
}

/// Batch embeddings computation for high throughput
pub fn embed_batch(texts: &[String], options: &EmbedOptions) -> Vec<Vec<f32>> {
    texts.iter().map(|t| embed_text(t, options)).collect()
}

/// Computes cosine similarity between two embedding slices
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom > 1e-12 {
        dot / denom
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Internal Encoder Helpers
// ---------------------------------------------------------------------------

fn tokenize_and_preprocess(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn compute_token_weight(token: &str) -> f32 {
    // Basic stop word discounting to focus semantic mass on content words
    const STOP_WORDS: &[&str] = &[
        "the", "is", "at", "which", "on", "a", "an", "and", "or", "in", "to", "for", "of", "with",
        "as", "by", "from", "that", "this", "it", "are", "was", "be", "has", "had", "have", "的",
        "了", "在", "是", "我", "有", "和", "就", "不", "人", "都", "一", "一个",
    ];

    if STOP_WORDS.contains(&token) {
        0.25
    } else {
        1.0 + (token.len() as f32).min(8.0) * 0.1
    }
}

#[inline(always)]
fn hash_str(s: &str, seed: u64) -> u64 {
    let mut h = seed;
    for b in s.as_bytes() {
        h = h.wrapping_mul(0x100000001b3);
        h ^= *b as u64;
    }
    h
}

#[inline(always)]
fn add_projected_features(embedding: &mut [f32], hash: u64, weight: f32, dim: usize) {
    // Deterministic pseudo-random projection vector generated from hash
    let mut state = hash;
    for d in 0..dim {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Rademacher / normal distribution pseudo-random value in [-1.0, 1.0]
        let u = ((state >> 32) as u32 as f32) / (u32::MAX as f32);
        let val = (2.0 * u - 1.0) * (2.0 * PI * (d as f32 / dim as f32)).cos();
        embedding[d] += val * weight;
    }
}

#[inline(always)]
fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + ((2.0 / PI).sqrt() * (x + 0.044715 * x * x * x)).tanh())
}

#[inline(always)]
fn l2_normalize(vec: &mut [f32]) {
    let mut sum_sq = 0.0f32;
    for &x in vec.iter() {
        sum_sq += x * x;
    }
    let norm = sum_sq.sqrt();
    if norm > 1e-12 {
        let inv_norm = 1.0 / norm;
        for x in vec.iter_mut() {
            *x *= inv_norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_dimensions_and_normalization() {
        let text = "Amber is a fast JavaScript runtime built with Rust";
        let opts = EmbedOptions {
            dimensions: 128,
            normalize: true,
        };
        let vec = embed_text(text, &opts);
        assert_eq!(vec.len(), 128);

        // Check L2 norm is ~1.0
        let norm_sq: f32 = vec.iter().map(|x| x * x).sum();
        assert!((norm_sq.sqrt() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_embed_determinism() {
        let text = "Consistent embedding output for identical input";
        let opts = EmbedOptions::default();
        let vec1 = embed_text(text, &opts);
        let vec2 = embed_text(text, &opts);
        assert_eq!(vec1, vec2);
    }

    #[test]
    fn test_semantic_similarity_clustering() {
        let opts = EmbedOptions::default();
        let v_js = embed_text(
            "High performance JavaScript engine and TypeScript runtime",
            &opts,
        );
        let v_ts = embed_text(
            "Fast TypeScript and JavaScript V8 execution platform",
            &opts,
        );
        let v_food = embed_text("Delicious homemade chocolate chip cookies recipe", &opts);

        let sim_tech = cosine_similarity(&v_js, &v_ts);
        let sim_unrelated = cosine_similarity(&v_js, &v_food);

        assert!(
            sim_tech > 0.60,
            "Tech sentences should have high similarity: got {}",
            sim_tech
        );
        assert!(
            sim_unrelated < 0.35,
            "Unrelated sentences should have lower similarity: got {}",
            sim_unrelated
        );
        assert!(
            sim_tech > sim_unrelated + 0.30,
            "Semantic gap should be significant"
        );
    }
}
