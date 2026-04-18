//! AI Gateway specific routing and rate limiting abstractions.

/// Represents semantic metadata extracted from an AI payload (e.g. OpenAI API).
#[derive(Debug, Clone)]
pub struct AiMetadata {
    /// The requested model name
    pub model: String,
    /// The estimated cost in tokens (or literal count if pre-calculated)
    pub estimated_tokens: u64,
    /// Optional semantic similarity hash (used for caching)
    pub semantic_hash: Option<String>,
}

impl AiMetadata {
    /// Provide a default blank metadata extraction for fallback.
    pub fn empty() -> Self {
        Self {
            model: "unknown".to_string(),
            estimated_tokens: 1, // At minimum, costs 1 request limit token
            semantic_hash: None,
        }
    }
}

/// Fallback directives for when a primary AI model provider fails
#[derive(Debug, Clone)]
pub struct ModelFallbackConfig {
    /// The primary backend model string (e.g. "gpt-4")
    pub primary_model: String,
    /// A list of fallback models if the primary is down or rate-limited
    pub fallback_models: Vec<String>,
}
