use crate::core::traits::{Result, AppError};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct VectorEntry {
    pub text: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

pub struct VectorDB {
    model: TextEmbedding,
    entries: Vec<VectorEntry>,
    storage_path: PathBuf,
}

impl VectorDB {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_default();
        let storage_path = PathBuf::from(home).join(".config/lasada/vectors.json");
        
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true)
        ).map_err(|e| AppError::ExecutionError(format!("Failed to init embedding model: {}", e)))?;

        let mut entries = Vec::new();
        if storage_path.exists() {
            let content = fs::read_to_string(&storage_path)
                .map_err(|e| AppError::ConfigError(format!("Failed to read vectors: {}", e)))?;
            entries = serde_json::from_str(&content)
                .unwrap_or_else(|_| Vec::new());
        }

        Ok(Self {
            model,
            entries,
            storage_path,
        })
    }

    pub fn add(&mut self, text: &str, metadata: HashMap<String, String>) -> Result<()> {
        if text.trim().is_empty() { return Ok(()); }
        
        let embeddings = self.model.embed(vec![text], None)
            .map_err(|e| AppError::ExecutionError(format!("Failed to generate embedding: {}", e)))?;
        
        if let Some(vector) = embeddings.get(0) {
            self.entries.push(VectorEntry {
                text: text.to_string(),
                vector: vector.clone(),
                metadata,
            });
            self.save()?;
        }
        
        Ok(())
    }

    pub fn search(&self, query: &str, top_k: usize) -> Result<Vec<VectorEntry>> {
        if self.entries.is_empty() { return Ok(Vec::new()); }

        let embeddings = self.model.embed(vec![query], None)
            .map_err(|e| AppError::ExecutionError(format!("Search embedding failed: {}", e)))?;
        
        let query_vec = match embeddings.get(0) {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        let mut scored_entries: Vec<(f32, &VectorEntry)> = self.entries.iter()
            .map(|entry| {
                let score = self.cosine_similarity(query_vec, &entry.vector);
                (score, entry)
            })
            .collect();

        // Sort by score descending
        scored_entries.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results = scored_entries.into_iter()
            .take(top_k)
            .filter(|(score, _)| *score > 0.5) // Threshold
            .map(|(_, entry)| entry.clone())
            .collect();

        Ok(results)
    }

    fn cosine_similarity(&self, v1: &[f32], v2: &[f32]) -> f32 {
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
        if norm1 == 0.0 || norm2 == 0.0 { return 0.0; }
        dot_product / (norm1 * norm2)
    }

    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| AppError::ConfigError(format!("Failed to serialize vectors: {}", e)))?;
        fs::write(&self.storage_path, content)
            .map_err(|e| AppError::ConfigError(format!("Failed to write vectors: {}", e)))?;
        Ok(())
    }
}
