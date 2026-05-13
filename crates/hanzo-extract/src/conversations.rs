//! Conversation extraction from Claude Code session logs.
//!
//! Extracts and anonymizes conversation turns for training datasets.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[cfg(feature = "conversations")]
use chrono::Utc;
#[cfg(feature = "conversations")]
use rand::seq::SliceRandom;
#[cfg(feature = "conversations")]
use rand::SeedableRng;
#[cfg(feature = "conversations")]
use sha2::{Digest, Sha256};
#[cfg(feature = "conversations")]
use walkdir::WalkDir;

use crate::error::Result;

/// Raw entry from Claude Code JSONL
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    message: Option<RawMessage>,
    timestamp: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    role: Option<String>,
    model: Option<String>,
    content: Option<serde_json::Value>,
    usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
}

/// A single conversation turn (user prompt + assistant response)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationTurn {
    /// User's input/instruction
    pub user: String,
    /// Assistant's response
    pub assistant: String,
    /// Assistant's thinking/reasoning (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Timestamp of the conversation
    pub timestamp: String,
    /// Anonymized session ID
    pub session_id: String,
    /// Model used for response
    pub model: String,
    /// Anonymized working directory
    pub cwd: String,
    /// Tools used in the response
    pub tools_used: Vec<String>,
    /// Quality score (0.0-1.0)
    pub quality: f32,
    /// Token usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read: u32,
}

/// Training format entry
#[derive(Debug, Serialize)]
pub struct TrainingEntry {
    pub instruction: String,
    pub response: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    pub model: String,
    pub tools: Vec<String>,
    pub quality: f32,
}

/// Export statistics
#[derive(Debug, Default)]
pub struct ExportStats {
    pub files_processed: usize,
    pub turns_exported: usize,
    pub skipped_snapshots: usize,
    pub skipped_empty: usize,
    pub tool_usage: HashMap<String, usize>,
    pub model_distribution: HashMap<String, usize>,
}

/// Conversation exporter configuration
#[derive(Debug, Clone)]
pub struct ExporterConfig {
    /// Minimum quality score (0.0-1.0)
    pub min_quality: f32,
    /// Maximum files to process (None = unlimited)
    pub max_files: Option<usize>,
    /// Salt for hashing (for anonymization)
    pub hash_salt: String,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            min_quality: 0.5,
            max_files: None,
            hash_salt: "hanzo".to_string(),
        }
    }
}

/// Conversation exporter
pub struct ConversationExporter {
    config: ExporterConfig,
    stats: ExportStats,
    // Regex patterns for anonymization
    path_regex: Regex,
    secret_patterns: Vec<(Regex, &'static str)>,
}

impl ConversationExporter {
    /// Create a new exporter with default config
    pub fn new() -> Self {
        Self::with_config(ExporterConfig::default())
    }

    /// Create a new exporter with custom config
    pub fn with_config(config: ExporterConfig) -> Self {
        let secret_patterns = vec![
            (Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(), "sk-REDACTED"),
            (Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(), "ghp_REDACTED"),
            (
                Regex::new(r"glpat-[a-zA-Z0-9_-]{20}").unwrap(),
                "glpat-REDACTED",
            ),
            (
                Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
                "email@example.com",
            ),
            (
                Regex::new(r"Bearer [a-zA-Z0-9_-]+").unwrap(),
                "Bearer REDACTED",
            ),
        ];

        Self {
            config,
            stats: ExportStats::default(),
            path_regex: Regex::new(r"(/Users|/home|C:\\Users)/[^/\\\s]+").unwrap(),
            secret_patterns,
        }
    }

    /// Anonymize file paths
    fn anonymize_path(&self, path: &str) -> String {
        self.path_regex.replace_all(path, "$1/user").into_owned()
    }

    /// Anonymize content (secrets, paths, etc.)
    fn anonymize_content(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Redact secrets
        for (pattern, replacement) in &self.secret_patterns {
            result = pattern.replace_all(&result, *replacement).into_owned();
        }

        // Anonymize paths
        result = self.anonymize_path(&result);

        result
    }

    /// Hash a value for anonymization
    #[cfg(feature = "conversations")]
    fn hash_value(&self, value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", self.config.hash_salt, value));
        let result = hasher.finalize();
        hex::encode(&result[..8])
    }

    #[cfg(not(feature = "conversations"))]
    fn hash_value(&self, value: &str) -> String {
        // Simple fallback hash
        format!("{:x}", value.len())
    }

    /// Extract text content from message
    fn extract_text_content(&mut self, content: &serde_json::Value) -> String {
        match content {
            serde_json::Value::String(s) => self.anonymize_content(s),
            serde_json::Value::Array(arr) => {
                let mut text_parts = Vec::new();
                for item in arr {
                    if let serde_json::Value::Object(obj) = item {
                        let item_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        match item_type {
                            "text" => {
                                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                    text_parts.push(self.anonymize_content(text));
                                }
                            }
                            "tool_use" => {
                                let tool_name = obj
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                *self
                                    .stats
                                    .tool_usage
                                    .entry(tool_name.to_string())
                                    .or_insert(0) += 1;

                                let input = obj
                                    .get("input")
                                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                                    .unwrap_or_default();
                                // Safe truncation respecting UTF-8 boundaries
                                let truncated: String = input.chars().take(200).collect();
                                text_parts.push(format!(
                                    "[Tool: {}] {}",
                                    tool_name,
                                    self.anonymize_content(&truncated)
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                text_parts.join("\n")
            }
            _ => String::new(),
        }
    }

    /// Extract thinking from assistant message
    fn extract_thinking(&self, content: &serde_json::Value) -> Option<String> {
        if let serde_json::Value::Array(arr) = content {
            let thinking_parts: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    if let serde_json::Value::Object(obj) = item {
                        if obj.get("type").and_then(|v| v.as_str()) == Some("thinking") {
                            return obj
                                .get("thinking")
                                .and_then(|v| v.as_str())
                                .map(|s| self.anonymize_content(s));
                        }
                    }
                    None
                })
                .collect();

            if !thinking_parts.is_empty() {
                return Some(thinking_parts.join("\n"));
            }
        }
        None
    }

    /// Extract tools used from message
    fn extract_tools(&self, content: &serde_json::Value) -> Vec<String> {
        let mut tools = Vec::new();
        if let serde_json::Value::Array(arr) = content {
            for item in arr {
                if let serde_json::Value::Object(obj) = item {
                    if obj.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                            tools.push(name.to_string());
                        }
                    }
                }
            }
        }
        tools
    }

    /// Calculate quality score for a turn
    fn calculate_quality(&self, turn: &ConversationTurn) -> f32 {
        let mut score: f32 = 0.5;

        // Has thinking
        if turn.thinking.is_some() {
            score += 0.2;
        }

        // Has tools
        if !turn.tools_used.is_empty() {
            score += 0.15;
            // Bonus for agentic tools
            let agentic = ["Task", "dispatch", "batch", "agent"];
            if turn
                .tools_used
                .iter()
                .any(|t| agentic.iter().any(|a| t.contains(a)))
            {
                score += 0.1;
            }
        }

        // Token usage
        if let Some(ref usage) = turn.usage {
            if usage.output_tokens > 100 {
                score += 0.1;
            }
            if usage.cache_read > 0 {
                score += 0.05;
            }
        }

        // Quality model
        let model = turn.model.to_lowercase();
        if model.contains("opus") {
            score += 0.1;
        } else if model.contains("sonnet") {
            score += 0.05;
        }

        // Substantial response
        if turn.assistant.len() > 500 {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// Process a single JSONL file
    fn process_file(&mut self, filepath: &Path) -> Result<Vec<ConversationTurn>> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);

        let mut conversations = Vec::new();
        let mut current_turn: Option<ConversationTurn> = None;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.trim().is_empty() {
                continue;
            }

            let entry: RawEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_type = entry.entry_type.as_deref().unwrap_or("");

            // Skip non-conversation entries
            if entry_type == "file-history-snapshot" || entry_type == "summary" {
                self.stats.skipped_snapshots += 1;
                continue;
            }

            if entry_type == "user" {
                // Save previous turn if complete
                if let Some(turn) = current_turn.take() {
                    if !turn.assistant.is_empty() {
                        conversations.push(turn);
                    }
                }

                // Start new turn
                if let Some(ref msg) = entry.message {
                    if let Some(ref content) = msg.content {
                        let user_text = self.extract_text_content(content);
                        if user_text.trim().is_empty() {
                            self.stats.skipped_empty += 1;
                            continue;
                        }

                        current_turn = Some(ConversationTurn {
                            user: user_text,
                            assistant: String::new(),
                            thinking: None,
                            timestamp: entry.timestamp.unwrap_or_default(),
                            session_id: self.hash_value(&entry.session_id.unwrap_or_default()),
                            model: String::new(),
                            cwd: self.anonymize_path(&entry.cwd.unwrap_or_default()),
                            tools_used: Vec::new(),
                            quality: 0.0,
                            usage: None,
                        });
                    }
                }
            } else if entry_type == "assistant" {
                if let Some(ref mut turn) = current_turn {
                    if let Some(ref msg) = entry.message {
                        // Model
                        if let Some(ref model) = msg.model {
                            if model != "<synthetic>" && turn.model.is_empty() {
                                turn.model = model.clone();
                                *self
                                    .stats
                                    .model_distribution
                                    .entry(model.clone())
                                    .or_insert(0) += 1;
                            }
                        }

                        // Content
                        if let Some(ref content) = msg.content {
                            let assistant_text = self.extract_text_content(content);
                            if !assistant_text.is_empty() {
                                if turn.assistant.is_empty() {
                                    turn.assistant = assistant_text;
                                } else {
                                    turn.assistant.push('\n');
                                    turn.assistant.push_str(&assistant_text);
                                }
                            }

                            // Thinking
                            if let Some(thinking) = self.extract_thinking(content) {
                                turn.thinking = Some(thinking);
                            }

                            // Tools
                            let tools = self.extract_tools(content);
                            turn.tools_used.extend(tools);
                        }

                        // Usage
                        if let Some(ref usage) = msg.usage {
                            turn.usage = Some(UsageStats {
                                input_tokens: usage.input_tokens.unwrap_or(0),
                                output_tokens: usage.output_tokens.unwrap_or(0),
                                cache_read: usage.cache_read_input_tokens.unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }

        // Don't forget last turn
        if let Some(turn) = current_turn {
            if !turn.assistant.is_empty() {
                conversations.push(turn);
            }
        }

        Ok(conversations)
    }

    /// Export conversations from a directory
    #[cfg(feature = "conversations")]
    pub fn export(&mut self, source_dir: &Path, output_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(output_dir)?;

        println!("Exporting conversations from {:?}", source_dir);
        println!("Output: {:?}", output_dir);
        println!("Min quality: {}\n", self.config.min_quality);

        // Find all JSONL files
        let mut jsonl_files: Vec<PathBuf> = WalkDir::new(source_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        if let Some(max) = self.config.max_files {
            jsonl_files.truncate(max);
        }

        println!("Found {} JSONL files\n", jsonl_files.len());

        let mut all_turns = Vec::new();

        for (i, filepath) in jsonl_files.iter().enumerate() {
            self.stats.files_processed += 1;

            if i % 100 == 0 && i > 0 {
                println!("  Processing {}/{}...", i, jsonl_files.len());
            }

            match self.process_file(filepath) {
                Ok(turns) => {
                    for mut turn in turns {
                        let quality = self.calculate_quality(&turn);
                        if quality >= self.config.min_quality {
                            turn.quality = quality;
                            all_turns.push(turn);
                            self.stats.turns_exported += 1;
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // Sort by timestamp
        all_turns.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Write output
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let output_file = output_dir.join(format!("conversations_{}.jsonl", timestamp));

        let mut file = File::create(&output_file)?;
        for turn in &all_turns {
            writeln!(file, "{}", serde_json::to_string(turn)?)?;
        }

        // Write training format
        let training_file = output_dir.join(format!("training_{}.jsonl", timestamp));
        let mut file = File::create(&training_file)?;
        for turn in &all_turns {
            let entry = TrainingEntry {
                instruction: turn.user.clone(),
                response: turn.assistant.clone(),
                thinking: turn.thinking.clone(),
                model: turn.model.clone(),
                tools: turn.tools_used.clone(),
                quality: turn.quality,
            };
            writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        }

        // Print summary
        println!("\n{}", "=".repeat(50));
        println!("Export Complete!");
        println!("{}", "=".repeat(50));
        println!("Files processed: {}", self.stats.files_processed);
        println!("Turns exported: {}", self.stats.turns_exported);
        println!("Skipped (snapshots): {}", self.stats.skipped_snapshots);
        println!("Skipped (empty): {}", self.stats.skipped_empty);
        println!("\nOutput files:");
        println!("  Conversations: {:?}", output_file);
        println!("  Training data: {:?}", training_file);

        println!("\nModels used:");
        let mut models: Vec<_> = self.stats.model_distribution.iter().collect();
        models.sort_by(|a, b| b.1.cmp(a.1));
        for (model, count) in models.iter().take(5) {
            println!("  {}: {}", model, count);
        }

        println!("\nTop tools:");
        let mut tools: Vec<_> = self.stats.tool_usage.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        for (tool, count) in tools.iter().take(10) {
            println!("  {}: {}", tool, count);
        }

        // Create splits
        self.create_splits(&all_turns, output_dir, &timestamp.to_string())?;

        Ok(output_file)
    }

    /// Create train/val/test splits
    #[cfg(feature = "conversations")]
    fn create_splits(
        &self,
        turns: &[ConversationTurn],
        output_dir: &Path,
        timestamp: &str,
    ) -> Result<()> {
        let splits_dir = output_dir.join("splits");
        std::fs::create_dir_all(&splits_dir)?;

        let mut shuffled: Vec<_> = turns.to_vec();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        shuffled.shuffle(&mut rng);

        let n = shuffled.len();
        let train_end = (0.8 * n as f64) as usize;
        let val_end = (0.9 * n as f64) as usize;

        let splits = [
            ("train", &shuffled[..train_end]),
            ("val", &shuffled[train_end..val_end]),
            ("test", &shuffled[val_end..]),
        ];

        println!("\nSplits ({:?}):", splits_dir);
        for (name, data) in splits {
            let path = splits_dir.join(format!("{}_{}.jsonl", name, timestamp));
            let mut file = File::create(&path)?;
            for turn in data {
                let entry = TrainingEntry {
                    instruction: turn.user.clone(),
                    response: turn.assistant.clone(),
                    thinking: turn.thinking.clone(),
                    model: turn.model.clone(),
                    tools: turn.tools_used.clone(),
                    quality: turn.quality,
                };
                writeln!(file, "{}", serde_json::to_string(&entry)?)?;
            }
            println!("  {}: {} turns", name, data.len());
        }

        Ok(())
    }

    /// Get export statistics
    pub fn stats(&self) -> &ExportStats {
        &self.stats
    }
}

impl Default for ConversationExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "conversations"))]
mod tests {
    use super::*;

    #[test]
    fn test_anonymize_path() {
        let exporter = ConversationExporter::new();

        assert_eq!(
            exporter.anonymize_path("/Users/john/work/project"),
            "/Users/user/work/project"
        );
        assert_eq!(
            exporter.anonymize_path("/home/alice/code"),
            "/home/user/code"
        );
    }

    #[test]
    fn test_anonymize_content() {
        let exporter = ConversationExporter::new();

        let content = "My email is test@example.com and key is sk-abcdefghijklmnopqrstuvwxyz";
        let anonymized = exporter.anonymize_content(content);

        assert!(anonymized.contains("email@example.com"));
        assert!(anonymized.contains("sk-REDACTED"));
        assert!(!anonymized.contains("test@example.com"));
    }

    #[test]
    fn test_quality_calculation() {
        let exporter = ConversationExporter::new();

        let mut turn = ConversationTurn {
            user: "Test".to_string(),
            assistant: "Response".to_string(),
            thinking: Some("Thinking about it...".to_string()),
            timestamp: String::new(),
            session_id: String::new(),
            model: "claude-opus-4-5-20251101".to_string(),
            cwd: String::new(),
            tools_used: vec!["Bash".to_string()],
            quality: 0.0,
            usage: Some(UsageStats {
                input_tokens: 100,
                output_tokens: 200,
                cache_read: 50,
            }),
        };

        let quality = exporter.calculate_quality(&turn);
        assert!(quality > 0.8, "Quality should be high: {}", quality);
    }
}
