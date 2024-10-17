mod cli;
mod epub;
mod google;
mod pdf;

use cli::Args;
use dotenv::dotenv;
use epub::{edit_epub, read_epub, write_epub};
use google::translate_text;
use pdf::{edit_pdf, read_pdf, write_pdf};

use std::{
    env,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use clap::Parser;
use eyre::Result;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Debug)]
enum FileType {
    PDF,
    EPUB,
    Unsupported,
}

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("GOOGLE_APPLICATION_CREDENTIALS")?;
    let args = Args::parse();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if args.verbose {
            Level::TRACE
        } else {
            Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    dotenv().ok();

    let file_type = get_file_type(&args.input)?;
    tracing::info!(
        "Converting {:?} file {} to {}...",
        file_type,
        args.input,
        args.to
    );

    match file_type {
        FileType::PDF => {
            let doc = read_pdf(&args.input)?;
            let edited = edit_pdf(doc, |text| {
                translate_text(text.to_string(), args.to.clone(), api_key.clone())
            })
            .await?;
            write_pdf(edited, &args.output)?;
        }
        FileType::EPUB => {
            let doc = read_epub(&args.input)?;
            let edited = edit_epub(doc, |text| {
                translate_text(text.to_string(), args.to.clone(), api_key.clone())
            })
            .await?;
            write_epub(edited, &args.output)?;
        }
        FileType::Unsupported => tracing::info!("File type not currently supported"),
    }

    Ok(())
}

fn get_file_type(path: &str) -> Result<FileType> {
    let mut file = File::open(path)?;
    let mut buffer = [0; 4];

    file.read_exact(&mut buffer)?;

    if &buffer == b"%PDF" {
        Ok(FileType::PDF)
    } else {
        file.seek(SeekFrom::Start(0))?;
        let mut zip_buffer = [0; 2];
        file.read_exact(&mut zip_buffer)?;

        if &zip_buffer == b"PK" {
            Ok(FileType::EPUB)
        } else {
            Ok(FileType::Unsupported)
        }
    }
}
