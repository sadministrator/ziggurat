use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Google API key
    #[arg(long)]
    pub api_key: Option<String>,

    /// Path to configuration file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Input file
    #[arg(short, long)]
    pub input: String,

    /// Output file
    #[arg(short, long)]
    pub output: String,

    /// Target language
    #[arg(long)]
    pub to: String,

    /// Enable verbose mode
    #[arg(short, long)]
    pub verbose: bool,
}
