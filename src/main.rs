mod detection;

use clap::Parser;
use detection::*;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "autonix")]
#[command(about = "Detect project languages and build tools", long_about = None)]
struct Args {
    /// Output format
    #[arg(long, value_enum, default_value = "debug")]
    format: OutputFormat,

    /// Path to analyze
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    /// Debug format (pretty-printed)
    Debug,
    /// JSON format
    Json,
}

fn main() {
    let args = Args::parse();
    let engine = DetectionEngine::new();
    let metadata = engine.detect(&args.path);

    match args.format {
        OutputFormat::Debug => {
            println!("{:#?}", metadata);
        }
        OutputFormat::Json => {
            match serde_json::to_string_pretty(&metadata) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Error serializing to JSON: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}


