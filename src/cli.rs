use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input file
    #[arg(short, long)]
    pub input: String,

    /// Output file
    #[arg(short, long)]
    pub output: String,

    /// Target language
    #[arg(short, long)]
    pub to: String,

    /// Enable verbose mode
    #[arg(short, long)]
    pub verbose: bool,
}
