//! CLI for extracting Claude Code conversations
//!
//! Usage:
//!   extract-conversations --source ~/.claude/projects --output ./conversations

use hanzo_extract::conversations::{ConversationExporter, ExporterConfig};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut source = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".claude/projects");
    let mut output = PathBuf::from("./conversations");
    let mut min_quality = 0.5f32;
    let mut max_files: Option<usize> = None;

    // Simple arg parsing
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--source" | "-s" => {
                if i + 1 < args.len() {
                    source = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--min-quality" | "-q" => {
                if i + 1 < args.len() {
                    min_quality = args[i + 1].parse().unwrap_or(0.5);
                    i += 1;
                }
            }
            "--max-files" | "-m" => {
                if i + 1 < args.len() {
                    max_files = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("extract-conversations - Extract Claude Code conversations for training");
                println!();
                println!("USAGE:");
                println!("    extract-conversations [OPTIONS]");
                println!();
                println!("OPTIONS:");
                println!(
                    "    -s, --source <PATH>       Source directory (default: ~/.claude/projects)"
                );
                println!(
                    "    -o, --output <PATH>       Output directory (default: ./conversations)"
                );
                println!(
                    "    -q, --min-quality <FLOAT> Minimum quality score 0.0-1.0 (default: 0.5)"
                );
                println!("    -m, --max-files <NUM>     Maximum files to process");
                println!("    -h, --help                Print help");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    // Expand ~ in paths
    if source.starts_with("~") {
        if let Ok(home) = std::env::var("HOME") {
            source = PathBuf::from(home).join(source.strip_prefix("~").unwrap());
        }
    }

    if !source.exists() {
        eprintln!("Error: Source directory not found: {:?}", source);
        std::process::exit(1);
    }

    let config = ExporterConfig {
        min_quality,
        max_files,
        hash_salt: "hanzo".to_string(),
    };

    let mut exporter = ConversationExporter::with_config(config);

    match exporter.export(&source, &output) {
        Ok(output_file) => {
            println!("\nDataset ready for training!");
            println!("Output: {:?}", output_file);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
