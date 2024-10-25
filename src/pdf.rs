use std::{future::Future, vec};

use eyre::Result;
use lopdf::{
    content::{Content, Operation},
    dictionary,
    xobject::PdfImage,
    Document, Object, Stream,
};
use regex::Regex;

#[derive(Debug)]
struct PagesState {
    pages: Vec<Content>,
    y_pos: f64,
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

    let mut page_ids = vec![];
    let mut image_resources = dictionary! {};
    let mut pages_state = PagesState {
        pages: vec![Content {
            operations: new_page_operations(),
        }],
        y_pos: 750.0,
    };

    for (page_num, page_id) in doc.get_pages() {
        let text = doc.extract_text(&[page_num])?;
        let edited_text = edit_func(&text).await?;
        let images = doc.get_page_images(page_id).unwrap_or_default();
        format_content(&mut pages_state, &edited_text, &images);

        for image in &images {
            let image_stream = create_image_stream(image);
            let image_id = edited_doc.add_object(image_stream);

            image_resources.set(format!("Im{}", image.id.0).into_bytes(), image_id);
        }
    }

    for content in &pages_state.pages {
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

fn create_image_stream(image: &PdfImage) -> Stream {
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
}

fn format_content(pages_state: &mut PagesState, text: &str, images: &[PdfImage]) {
    const MAX_WIDTH: f64 = 500.0;
    const LINE_HEIGHT: f64 = 14.0;
    const PARAGRAPH_SPACING: f64 = 20.0;
    const MIN_Y_POS: f64 = 50.0;
    const MAX_Y_POS: f64 = 750.0;
    const MAX_IMAGE_WIDTH: f64 = 500.0;
    const MAX_IMAGE_HEIGHT: f64 = 700.0;

    let paragraph_split = Regex::new(r"\n\s*\n").unwrap();
    let paragraphs: Vec<&str> = paragraph_split.split(text).collect();

    for paragraph in paragraphs {
        format_paragraph(
            pages_state,
            paragraph,
            MAX_WIDTH,
            LINE_HEIGHT,
            PARAGRAPH_SPACING,
            MIN_Y_POS,
            MAX_Y_POS,
        );
    }

    end_text_section(pages_state);

    for image in images {
        add_image(
            pages_state,
            image,
            MAX_IMAGE_WIDTH,
            MAX_IMAGE_HEIGHT,
            MIN_Y_POS,
            MAX_Y_POS,
        );
    }

    end_text_section(pages_state);
}

fn format_paragraph(
    pages_state: &mut PagesState,
    paragraph: &str,
    max_width: f64,
    line_height: f64,
    paragraph_spacing: f64,
    min_y_pos: f64,
    max_y_pos: f64,
) {
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    let mut current_line = String::new();

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if string_width(&test_line) > max_width {
            add_line_to_page(pages_state, &current_line, line_height);
            current_line = word.to_string();
        } else {
            current_line = test_line;
        }

        check_and_create_new_page(pages_state, min_y_pos, max_y_pos);
    }

    if !current_line.is_empty() {
        add_line_to_page(pages_state, &current_line, 0.0);
    }

    add_paragraph_spacing(pages_state, paragraph_spacing, min_y_pos, max_y_pos);
}

fn add_line_to_page(pages_state: &mut PagesState, line: &str, line_height: f64) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        last_page.operations.push(Operation::new(
            "Tj",
            vec![Object::string_literal(line.to_string())],
        ));
        last_page
            .operations
            .push(Operation::new("Td", vec![0.into(), (-line_height).into()]));
        pages_state.y_pos -= line_height;
    }
}

fn check_and_create_new_page(pages_state: &mut PagesState, min_y_pos: f64, max_y_pos: f64) {
    if pages_state.y_pos < min_y_pos {
        pages_state.pages.push(Content {
            operations: new_page_operations(),
        });
        pages_state.y_pos = max_y_pos;
    }
}

fn add_paragraph_spacing(
    pages_state: &mut PagesState,
    paragraph_spacing: f64,
    min_y_pos: f64,
    max_y_pos: f64,
) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        pages_state.y_pos -= paragraph_spacing;
        last_page.operations.push(Operation::new(
            "Td",
            vec![0.into(), (-paragraph_spacing).into()],
        ));

        if pages_state.y_pos < min_y_pos {
            last_page.operations.extend_from_slice(&[
                Operation::new("ET", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Td", vec![50.into(), max_y_pos.into()]),
            ]);
            pages_state.pages.push(Content {
                operations: new_page_operations(),
            });
            pages_state.y_pos = max_y_pos;
        }
    }
}

fn end_text_section(pages_state: &mut PagesState) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        last_page.operations.push(Operation::new("ET", vec![]));
    }
}

fn add_image(
    pages_state: &mut PagesState,
    image: &PdfImage,
    max_image_width: f64,
    max_image_height: f64,
    min_y_pos: f64,
    max_y_pos: f64,
) {
    let scale = calculate_image_scale(image, max_image_width, max_image_height);
    let scaled_width = image.width as f64 * scale;
    let scaled_height = image.height as f64 * scale;

    if pages_state.y_pos - scaled_height < min_y_pos {
        create_new_page_for_image(pages_state, max_y_pos);
    }

    if let Some(last_page) = pages_state.pages.last_mut() {
        add_image_operations(
            last_page,
            image,
            scaled_width,
            scaled_height,
            pages_state.y_pos,
        );
        pages_state.y_pos -= scaled_height + 10.0;
    }
}

fn calculate_image_scale(image: &PdfImage, max_width: f64, max_height: f64) -> f64 {
    let width_scale = max_width / image.width as f64;
    let height_scale = max_height / image.height as f64;
    width_scale.min(height_scale).min(1.0)
}

fn create_new_page_for_image(pages_state: &mut PagesState, max_y_pos: f64) {
    pages_state.pages.push(Content {
        operations: new_page_operations(),
    });
    pages_state.y_pos = max_y_pos;
    if let Some(last_page) = pages_state.pages.last_mut() {
        last_page.operations.extend_from_slice(&[
            Operation::new("BT", vec![]),
            Operation::new("Td", vec![50.into(), max_y_pos.into()]),
            Operation::new("ET", vec![]),
        ]);
    }
}

fn add_image_operations(page: &mut Content, image: &PdfImage, width: f64, height: f64, y_pos: f64) {
    page.operations.extend_from_slice(&[
        Operation::new("q", vec![]),
        Operation::new(
            "cm",
            vec![
                width.into(),
                0.into(),
                0.into(),
                height.into(),
                50.into(),
                (y_pos - height).into(),
            ],
        ),
        Operation::new(
            "Do",
            vec![Object::Name(format!("Im{}", image.id.0).into_bytes())],
        ),
        Operation::new("Q", vec![]),
    ]);
}

fn string_width(s: &str) -> f64 {
    s.len() as f64 * 7.0
}

fn new_page_operations() -> Vec<Operation> {
    vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new("Td", vec![50.into(), 750.into()]),
    ]
}
