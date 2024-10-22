use std::{collections::HashMap, future::Future};

use eyre::Result;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};

#[derive(Debug)]
struct ImageData {
    content: Vec<u8>,
    width: u32,
    height: u32,
}

pub fn read_pdf(path: &str) -> Result<Document> {
    tracing::info!("Reading {path}...");
    let doc = Document::load(path)?;

    Ok(doc)
}

pub fn write_pdf(mut doc: Document, to: &str) -> Result<()> {
    tracing::info!("Writing pdf to {to}...");
    doc.save(to)?;

    Ok(())
}

pub async fn edit_pdf<F, Fut>(doc: Document, edit_func: F) -> Result<Document>
where
    F: Fn(&str) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let mut edited_doc = Document::with_version("1.5");
    let pages_id = edited_doc.new_object_id();
    let font_id = edited_doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });

    let images = extract_images(&doc)?;
    let mut image_resources = dictionary! {};
    for (name, image_data) in images {
        let image_stream = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => image_data.width,
                "Height" => image_data.height,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
            },
            image_data.content,
        );
        let image_id = edited_doc.add_object(image_stream);

        image_resources.set(name, image_id);
    }

    let resources_id = edited_doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
        "XObject" => image_resources,
    });

    let mut page_ids: Vec<Object> = vec![];
    for (page_num, _) in doc.get_pages() {
        let page_text = doc.extract_text(&[page_num])?;
        let edited_text = edit_func(&page_text).await?;
        let content = Content {
            operations: format_text(&edited_text),
        };
        let content_id = edited_doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = edited_doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });

        page_ids.push(page_id.into());
    }
    let page_count = page_ids.len() as u32;
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids,
        "Count" => page_count,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    };

    edited_doc
        .objects
        .insert(pages_id, Object::Dictionary(pages));

    let catalog_id = edited_doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    edited_doc.trailer.set("Root", catalog_id);
    edited_doc.compress();

    Ok(edited_doc)
}

fn extract_images(doc: &Document) -> Result<HashMap<String, ImageData>> {
    let mut images = HashMap::new();

    for (id, object) in doc.objects.iter() {
        if let Object::Stream(ref stream) = object {
            if let Ok(subtype) = stream.dict.get(b"Subtype") {
                if let Ok(subtype_name) = subtype.as_name() {
                    if subtype_name == b"Image" {
                        let content = stream.content.clone();
                        let width = stream.dict.get(b"Width")?.as_i64()? as u32;
                        let height = stream.dict.get(b"Height")?.as_i64()? as u32;
                        let name = stream
                            .dict
                            .get(b"Name")
                            .and_then(|n| n.as_name())
                            .map(|n| String::from_utf8_lossy(n).into_owned())
                            .unwrap_or_else(|_| format!("image_{}_{}", id.0, id.1));

                        images.insert(
                            name,
                            ImageData {
                                content,
                                width,
                                height,
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(images)
}

fn format_text(text: &str) -> Vec<Operation> {
    let mut operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]), // font style
        Operation::new("Td", vec![50.into(), 750.into()]),  // cursor position
    ];

    let max_width = 500.0;
    let line_height = 14.0;

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut current_line = String::new();
    let mut y_position = 750.0;

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if string_width(&test_line) > max_width {
            operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(current_line.clone())],
            ));
            operations.push(Operation::new("Td", vec![0.into(), (-line_height).into()]));

            y_position -= line_height;
            current_line = word.to_string();
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }

        // check if we need to start a new page
        if y_position < 50.0 {
            operations.push(Operation::new("ET", vec![]));
            operations.push(Operation::new("BT", vec![]));
            operations.push(Operation::new("Td", vec![50.into(), 750.into()]));
            y_position = 750.0;
        }
    }

    // add any remaining text
    if !current_line.is_empty() {
        operations.push(Operation::new(
            "Tj",
            vec![Object::string_literal(current_line)],
        ));
    }

    operations.push(Operation::new("ET", vec![]));

    operations
}

fn string_width(s: &str) -> f32 {
    s.len() as f32 * 7.0
}
