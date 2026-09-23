//! High-performance vector similarity search engine for Amber (`amber:vector`).
//!
//! Designed for Agentic AI workflows: stores embeddings, computes cosine/euclidean/dot
//! distance, and returns Top-K nearest neighbors. Directly interoperates with `amber:ai`'s
//! Tensor and Float32Array.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorMetric {
    Cosine,
    Euclidean,
    Dot,
}

impl Default for VectorMetric {
    fn default() -> Self {
        Self::Cosine
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: Value,
}

pub struct VectorDB {
    pub dimensions: usize,
    pub metric: VectorMetric,
    pub entries: HashMap<String, VectorEntry>,
}

impl VectorDB {
    pub fn new(dimensions: usize, metric: VectorMetric) -> Self {
        Self {
            dimensions,
            metric,
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: String, vector: Vec<f32>, metadata: Value) -> Result<()> {
        if self.dimensions > 0 && vector.len() != self.dimensions {
            return Err(anyhow!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimensions,
                vector.len()
            ));
        }
        if self.dimensions == 0 {
            self.dimensions = vector.len();
        }
        self.entries.insert(
            id.clone(),
            VectorEntry {
                id,
                vector,
                metadata,
            },
        );
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.entries.remove(id).is_some()
    }

    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.entries.get(id)
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn search(
        &self,
        query: &[f32],
        top_k: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<SearchResult>> {
        if self.dimensions > 0 && query.len() != self.dimensions {
            return Err(anyhow!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimensions,
                query.len()
            ));
        }

        let mut scored: Vec<SearchResult> = self
            .entries
            .values()
            .map(|entry| {
                let score = match self.metric {
                    VectorMetric::Cosine => cosine_similarity(query, &entry.vector),
                    VectorMetric::Euclidean => {
                        // Invert euclidean distance so higher = closer
                        let dist = euclidean_distance(query, &entry.vector);
                        1.0 / (1.0 + dist)
                    }
                    VectorMetric::Dot => dot_product(query, &entry.vector),
                };
                SearchResult {
                    id: entry.id.clone(),
                    score,
                    metadata: entry.metadata.clone(),
                }
            })
            .collect();

        // Sort descending by score
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(th) = threshold {
            scored.retain(|res| res.score >= th);
        }

        scored.truncate(top_k);
        Ok(scored)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json_str = serde_json::to_string(&self.entries.values().collect::<Vec<_>>())?;
        fs::write(path, json_str)?;
        Ok(())
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let entries: Vec<VectorEntry> = serde_json::from_str(&content)?;
        for entry in entries {
            self.insert(entry.id, entry.vector, entry.metadata)?;
        }
        Ok(())
    }
}

pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
