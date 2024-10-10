mod cli;
mod epub;
mod pdf;

use cli::Args;
use epub::{read_epub, write_epub};
use pdf::{edit_pdf, read_pdf, write_pdf};

use std::{
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

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
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
            let edited_doc = edit_pdf(doc, |text| text.to_string())?;
            write_pdf(edited_doc, &args.output)?;
        }
        FileType::EPUB => {
            let doc = read_epub(&args.input)?;
            write_epub(doc, &args.output)?;
        }
        FileType::Unsupported => tracing::info!("File type not currently supported."),
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
