use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input PDF file
    #[arg(short, long)]
    pub input: String,

    /// Output PDF file
    #[arg(short, long)]
    pub output: String,

    /// Target language
    #[arg(short, long)]
    pub to: String,

    /// Enable verbose mode
    #[arg(short, long)]
    pub verbose: bool,
}
