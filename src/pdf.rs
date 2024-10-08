use eyre::Result;
use lopdf::Document;

pub fn read_pdf(path: &str) -> Result<Document> {
    tracing::info!("Reading {path}...");
    let doc = Document::load(path)?;

    Ok(doc)
}

pub fn write_pdf(mut doc: Document, to: String) -> Result<()> {
    tracing::info!("Writing pdf to {to}...");
    doc.save(to)?;

    Ok(())
}
