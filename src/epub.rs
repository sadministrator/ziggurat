use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Cursor, Write},
    path::Path,
};

use epub::doc::EpubDoc;
use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use eyre::{eyre, Result};
use html2text::from_read;
use regex::Regex;

pub struct EditedEpub {
    pub doc: EpubDoc<BufReader<File>>,
    pub content: HashMap<String, String>,
}

pub fn read_epub(path: &str) -> Result<EpubDoc<BufReader<File>>> {
    tracing::info!("Reading {path}...");
    let doc = EpubDoc::new(Path::new(path))?;

    Ok(doc)
}

pub fn write_epub(mut edited: EditedEpub, to: &str) -> Result<()> {
    tracing::info!("Writing epub...");
    let mut builder = EpubBuilder::new(ZipLibrary::new()?)?;

    add_metadata(&mut builder, &edited)?;
    add_resources(&mut builder, &mut edited)?;
    add_cover_image(&mut builder, &mut edited)?;
    add_content_with_chapters(&mut builder, &mut edited.doc, &edited.content)?;

    let output = File::create(to)?;
    let mut buf_writer = BufWriter::new(output);
    builder.generate(&mut buf_writer)?;
    buf_writer.flush()?;

    Ok(())
}

fn add_metadata(builder: &mut EpubBuilder<ZipLibrary>, edited: &EditedEpub) -> Result<()> {
    let epub_builder_fields = ["title", "contributor", "description", "subject"];

    for field in epub_builder_fields {
        if let Some(values) = edited.doc.metadata.get(field) {
            for value in values {
                builder.metadata(field, value)?;
            }
        }
    }
    Ok(())
}

fn add_resources(builder: &mut EpubBuilder<ZipLibrary>, edited: &mut EditedEpub) -> Result<()> {
    for (id, (path, mime)) in edited.doc.resources.clone().iter() {
        if let Some((data, _)) = edited.doc.get_resource(&id) {
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

fn add_cover_image(builder: &mut EpubBuilder<ZipLibrary>, edited: &mut EditedEpub) -> Result<()> {
    if let Some((cover_data, mime)) = edited.doc.get_cover() {
        builder.add_cover_image("cover_image", Cursor::new(cover_data), mime)?;
    }
    Ok(())
}

fn add_content_with_chapters(
    builder: &mut EpubBuilder<ZipLibrary>,
    doc: &mut EpubDoc<BufReader<File>>,
    edited_content: &HashMap<String, String>,
) -> Result<()> {
    for item_id in doc.spine.clone().iter() {
        if let Some((path, _mime)) = doc.resources.clone().get(item_id) {
            let path_str = path
                .to_str()
                .ok_or_else(|| eyre!("Invalid path {}", path.to_string_lossy().into_owned()))?;

            let content = if let Some(edited) = edited_content.get(item_id) {
                edited.clone()
            } else {
                doc.get_resource_str_by_path(path_str)
                    .ok_or_else(|| eyre!("Resource not found {}", path_str.to_string()))?
            };

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

pub fn edit_epub(
    mut doc: EpubDoc<BufReader<File>>,
    edit_func: impl Fn(&str) -> String,
) -> Result<EditedEpub> {
    let mut edited_content = HashMap::new();

    for _ in 0..doc.get_num_pages() {
        if let Some((content, mime)) = doc.get_current_str() {
            if mime == "application/xhtml+xml" {
                let text = from_read(content.as_bytes(), 80);
                let cleaned_text = clean_text(&text);
                let edited_text = edit_func(&cleaned_text);
                let edited_html = text_to_html(&edited_text, &content);
                let current_id = doc
                    .get_current_id()
                    .ok_or(eyre!("Unable to get current id"))?;

                edited_content.insert(current_id, edited_html);
            }
        }

        doc.go_next();
    }
    doc.set_current_page(0);

    Ok(EditedEpub {
        doc,
        content: edited_content,
    })
}

fn clean_text(text: &str) -> String {
    let page_number_re = Regex::new(r"\n\d+\n").unwrap();
    let cleaned = page_number_re.replace_all(text, "\n").to_string();
    cleaned.replace("\n\n", "\n").trim().to_string()
}

fn text_to_html(text: &str, original_html: &str) -> String {
    // This is a simplified version. You might need to adjust it based on your specific needs.
    let body_content = text
        .split("\n")
        .map(|line| format!("<p>{}</p>", line))
        .collect::<Vec<_>>()
        .join("\n");
    original_html.replace(
        r#"<body>.*</body>"#,
        &format!("<body>{}</body>", body_content),
    )
}
