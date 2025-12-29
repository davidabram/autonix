use autonix::*;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "autonix")]
#[command(about = "Detect project languages and build tools", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, value_enum, default_value = "debug")]
    format: OutputFormat,

    #[arg(long, value_enum, default_value = "all", global = true)]
    detect_scope: DetectScope,

    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Parser, Debug)]
enum Command {
    Generate {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Debug,
    Json,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum DetectScope {
    All,
    Root,
}

impl From<DetectScope> for DetectionScope {
    fn from(value: DetectScope) -> Self {
        match value {
            DetectScope::All => DetectionScope::All,
            DetectScope::Root => DetectionScope::Root,
        }
    }
}

fn main() {
    let args = Args::parse();
    let engine = DetectionEngine;
    let detect_scope: DetectionScope = args.detect_scope.into();

    match args.command {
        Some(Command::Generate { path }) => {
            let metadata = engine.detect_with_scope(&path, detect_scope);
            let flake = generate_dev_flake(&metadata, &path);
            print!("{flake}");
        }
        None => {
            let metadata = engine.detect_with_scope(&args.path, detect_scope);
            match args.format {
                OutputFormat::Debug => {
                    println!("{:#?}", metadata);
                }
                OutputFormat::Json => match serde_json::to_string_pretty(&metadata) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        eprintln!("Error serializing to JSON: {}", e);
                        std::process::exit(1);
                    }
                },
            }
        }
    }
}
