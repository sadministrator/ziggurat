use std::{
    fs::File,
    io::{BufReader, BufWriter, Cursor, Write},
    path::Path,
};

use epub::doc::EpubDoc;
use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use eyre::{eyre, Result};

pub fn read_epub(path: &str) -> Result<EpubDoc<BufReader<File>>> {
    tracing::info!("Reading {path}...");
    let doc = EpubDoc::new(Path::new(path))?;

    Ok(doc)
}

pub fn write_epub(mut doc: EpubDoc<BufReader<File>>, to: String) -> Result<()> {
    tracing::info!("Writing epub...");
    let mut builder = EpubBuilder::new(ZipLibrary::new()?)?;

    add_metadata(&mut builder, &doc)?;
    add_resources(&mut builder, &mut doc)?;
    add_cover_image(&mut builder, &mut doc)?;
    add_content_with_chapters(&mut builder, &mut doc)?;

    let output = File::create(to)?;
    let mut buf_writer = BufWriter::new(output);
    builder.generate(&mut buf_writer)?;
    buf_writer.flush()?;

    Ok(())
}

fn add_metadata(
    builder: &mut EpubBuilder<ZipLibrary>,
    doc: &EpubDoc<BufReader<File>>,
) -> Result<()> {
    let epub_builder_fields = ["title", "contributor", "description", "subject"];

    for field in epub_builder_fields {
        if let Some(values) = doc.metadata.get(field) {
            for value in values {
                builder.metadata(field, value)?;
            }
        }
    }
    Ok(())
}

fn add_resources(
    builder: &mut EpubBuilder<ZipLibrary>,
    doc: &mut EpubDoc<BufReader<File>>,
) -> Result<()> {
    for (id, (path, mime)) in doc.resources.clone().iter() {
        if let Some((data, _)) = doc.get_resource(&id) {
            builder.add_resource(
                path.to_str()
                    .ok_or_else(|| eyre!("Invalid path for resource: {}", id))?,
                Cursor::new(data),
                mime,
            )?;
        } else {
            tracing::warn!("Failed to get resource data for: {}", id);
        }
    }
    Ok(())
}

fn add_cover_image(
    builder: &mut EpubBuilder<ZipLibrary>,
    doc: &mut EpubDoc<BufReader<File>>,
) -> Result<()> {
    if let Some((cover_data, mime)) = doc.get_cover() {
        builder.add_cover_image("cover_image", Cursor::new(cover_data), mime)?;
    }
    Ok(())
}

fn add_content_with_chapters(
    builder: &mut EpubBuilder<ZipLibrary>,
    doc: &mut EpubDoc<BufReader<File>>,
) -> Result<()> {
    for item_id in doc.spine.clone().iter() {
        if let Some((path, _mime)) = doc.resources.clone().get(item_id) {
            let path_str = path
                .to_str()
                .ok_or_else(|| eyre!("Invalid path {}", path.to_string_lossy().into_owned()))?;

            let content = doc
                .get_resource_str_by_path(path_str)
                .ok_or_else(|| eyre!("Resource not found {}", path_str.to_string()))?;

            builder.add_content(
                EpubContent::new(path_str, Cursor::new(content))
                    .title(item_id)
                    .reftype(ReferenceType::Text),
            )?;
        } else {
            tracing::warn!("Resource not found for spine item: {}", item_id);
        }
    }

    Ok(())
}
