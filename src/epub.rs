use std::{
    collections::HashMap,
    fs::File,
    future::Future,
    io::{BufReader, BufWriter, Cursor, Write},
    path::Path,
};

use epub::doc::EpubDoc;
use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use eyre::{eyre, Result};
use tl::{Bytes, Node, ParserOptions};

pub struct EditedEpub {
    pub doc: EpubDoc<BufReader<File>>,
    pub content: HashMap<String, String>,
}

pub fn read_epub(path: &str) -> Result<EpubDoc<BufReader<File>>> {
    tracing::info!("Reading {path}...");
    let doc = EpubDoc::new(Path::new(path))?;

    Ok(doc)
}

pub async fn edit_epub<F, Fut>(
    mut doc: EpubDoc<BufReader<File>>,
    edit_func: F,
) -> Result<EditedEpub>
where
    F: Fn(&str) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let mut edited_content = HashMap::new();

    for _ in 0..doc.get_num_pages() {
        if let Some((content, mime)) = doc.get_current_str() {
            if mime == "application/xhtml+xml" {
                let edited_html = edit_html(&content, &edit_func).await?;
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

async fn edit_html<F, Fut>(html: &str, edit_func: F) -> Result<String>
where
    F: Fn(&str) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let (html, special_tags) = replace_special_tags(html);

    let mut dom = tl::parse(&html, ParserOptions::default())?;
    let mut text_nodes = vec![];

    for (index, node) in dom.nodes().iter().enumerate() {
        if let Node::Raw(_) = node {
            text_nodes.push(index);
        }
    }

    let parser = dom.parser_mut();

    for &index in &text_nodes {
        if let Some(Node::Raw(bytes)) = &mut parser.resolve_node_id(index as u32) {
            let text = bytes.as_utf8_str();
            let edited_text = edit_func(&text).await?;
            let mut edited_bytes = Bytes::new();
            edited_bytes.set(edited_text.as_bytes())?;
            if let Some(node) = parser.resolve_node_id_mut(index as u32) {
                *node = Node::Raw(edited_bytes);
            }
        }
    }

    let mut edited_html = dom.outer_html();
    edited_html = restore_special_tags(edited_html, special_tags);

    Ok(edited_html)
}

fn replace_special_tags(html: &str) -> (String, Vec<(String, String)>) {
    let mut special_tags = Vec::new();
    let mut new_html = html.to_string();
    let re = regex::Regex::new(r"<.*pagebreak.*>").unwrap();

    for cap in re.captures_iter(html) {
        tracing::error!("{}", &cap[0]);
        let tag = cap[0].to_string();
        let placeholder = format!("SPECIAL_TAG_{}", special_tags.len());
        special_tags.push((placeholder.clone(), tag.clone()));
        new_html = new_html.replace(&tag, &placeholder);
    }

    (new_html, special_tags)
}

fn restore_special_tags(mut html: String, special_tags: Vec<(String, String)>) -> String {
    for (placeholder, tag) in special_tags {
        html = html.replace(&placeholder, &tag);
    }
    html
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
