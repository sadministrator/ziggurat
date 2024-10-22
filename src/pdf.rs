use std::{future::Future, vec};

use eyre::Result;
use lopdf::{
    content::{Content, Operation},
    dictionary,
    xobject::PdfImage,
    Document, Object, Stream,
};

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

    let mut page_ids: Vec<Object> = vec![];
    let mut image_resources = dictionary! {};

    for (page_num, page_id) in doc.get_pages() {
        let text = doc.extract_text(&[page_num])?;
        let edited_text = edit_func(&text).await?;
        let images = doc.get_page_images(page_id).unwrap_or_default();

        for image in &images {
            let image_stream = {
                let mut dict = dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => image.width,
                    "Height" => image.height,
                    "ColorSpace" => image.color_space.clone().unwrap_or("DeviceRGB".to_owned()),
                    "BitsPerComponent" => image.bits_per_component.unwrap_or(8),
                };

                if let Some(filters) = &image.filters {
                    if !filters.is_empty() {
                        if filters.len() == 1 {
                            dict.set("Filter", Object::Name(filters[0].clone().into_bytes()));
                        } else {
                            dict.set(
                                "Filter",
                                Object::Array(
                                    filters
                                        .iter()
                                        .map(|f| Object::Name(f.clone().into_bytes()))
                                        .collect(),
                                ),
                            );
                        }
                    }
                }

                Stream::new(dict, image.content.to_vec())
            };
            let image_id = edited_doc.add_object(image_stream);

            image_resources.set(format!("Im{}", image.id.0).into_bytes(), image_id);
        }

        let content = Content {
            operations: format_content(&edited_text, &images),
        };
        let content_id = edited_doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = edited_doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });

        page_ids.push(page_id.into());
    }

    let resources_id = edited_doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
        "XObject" => image_resources,
    });
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

fn format_content(text: &str, images: &Vec<PdfImage>) -> Vec<Operation> {
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

    let max_image_width = 500.0;
    let max_image_height = 700.0;

    for image in images {
        let width_scale = max_image_width / image.width as f64;
        let height_scale = max_image_height / image.height as f64;
        let scale = width_scale.min(height_scale).min(1.0);
        let scaled_width = image.width as f64 * scale;
        let scaled_height = image.height as f64 * scale;

        // check if there's enough space for the image on the current page
        if y_position - (image.height as f64) < 50.0 {
            operations.push(Operation::new("BT", vec![]));
            operations.push(Operation::new("Td", vec![50.into(), 750.into()]));
            y_position = 750.0;
            operations.push(Operation::new("ET", vec![]));
        }

        operations.push(Operation::new("q", vec![]));
        operations.push(Operation::new(
            "cm",
            vec![
                scaled_width.into(),
                0.into(),
                0.into(),
                scaled_height.into(),
                50.into(),
                (y_position - scaled_height as f64).into(),
            ],
        ));
        operations.push(Operation::new(
            "Do",
            vec![Object::Name(format!("Im{}", image.id.0).into_bytes())],
        ));
        operations.push(Operation::new("Q", vec![]));

        y_position -= (scaled_height as f64) + 10.0;
    }

    operations
}

fn string_width(s: &str) -> f32 {
    s.len() as f32 * 7.0
}
