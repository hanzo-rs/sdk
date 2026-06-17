//! Search module for Rust MCP.
//!
//! `ast_search` is the live, tested path (tree-sitter semantic search).
//! The text/symbol/vector/unified searchers predate the active build graph and
//! do not yet compile against current deps; they stay disabled until refactored
//! — same convention as the disabled workspace crates in LLM.md.

pub mod ast_search;
// pub mod unified_search;
// pub mod symbol_search;
// pub mod vector_store;
// pub mod search;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Search result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column: usize,
    pub match_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub match_type: MatchType,
    pub score: f32,
    pub node_type: Option<String>,
    pub semantic_context: Option<String>,
}

/// Match type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    Text,
    Ast,
    Symbol,
    Vector,
    Memory,
    File,
}
