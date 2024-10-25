use std::{future::Future, vec};

use eyre::Result;
use lopdf::{
    content::{Content, Operation},
    dictionary,
    xobject::PdfImage,
    Dictionary, Document, Object, ObjectId, Stream,
};
use regex::Regex;

#[derive(Debug)]
struct PagesState {
    pages: Vec<Content>,
    y_pos: f64,
}

impl PagesState {
    pub fn new() -> Self {
        Self {
            pages: vec![Content {
                operations: new_page_operations(),
            }],
            y_pos: MAX_Y_POS,
        }
    }
}

const MAX_WIDTH: f64 = 500.0;
const LINE_HEIGHT: f64 = 14.0;
const PARAGRAPH_SPACING: f64 = 20.0;
const MIN_Y_POS: f64 = 50.0;
const MAX_Y_POS: f64 = 750.0;
const MAX_IMAGE_WIDTH: f64 = 500.0;
const MAX_IMAGE_HEIGHT: f64 = 700.0;

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
    let font_id = add_font(&mut edited_doc);

    let mut page_ids = Vec::new();
    let mut image_resources = dictionary! {};
    let mut pages_state = PagesState::new();

    for (page_num, page_id) in doc.get_pages() {
        let text = doc.extract_text(&[page_num])?;
        let edited_text = edit_func(&text).await?;
        let images = doc.get_page_images(page_id).unwrap_or_default();

        format_content(&mut pages_state, &edited_text, &images);
        add_images_to_resources(&mut edited_doc, &mut image_resources, &images);
    }

    add_pages_to_document(&mut edited_doc, &pages_state, pages_id, &mut page_ids)?;

    let resources_id = add_resources(&mut edited_doc, font_id, image_resources);
    add_pages_object(&mut edited_doc, pages_id, &page_ids, resources_id);
    add_catalog(&mut edited_doc, pages_id);

    edited_doc.compress();
    Ok(edited_doc)
}

fn add_font(doc: &mut Document) -> ObjectId {
    doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    })
}

fn add_images_to_resources(doc: &mut Document, resources: &mut Dictionary, images: &[PdfImage]) {
    for image in images {
        let image_stream = create_image_stream(image);
        let image_id = doc.add_object(image_stream);
        resources.set(format!("Im{}", image.id.0).into_bytes(), image_id);
    }
}

fn add_pages_to_document(
    doc: &mut Document,
    pages_state: &PagesState,
    pages_id: ObjectId,
    page_ids: &mut Vec<Object>,
) -> Result<()> {
    for content in &pages_state.pages {
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        page_ids.push(page_id.into());
    }

    Ok(())
}

fn add_resources(doc: &mut Document, font_id: ObjectId, image_resources: Dictionary) -> ObjectId {
    doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
        "XObject" => image_resources,
    })
}

fn add_pages_object(
    doc: &mut Document,
    pages_id: ObjectId,
    page_ids: &[Object],
    resources_id: ObjectId,
) {
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids.to_vec(),
        "Count" => page_ids.len() as u32,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));
}

fn add_catalog(doc: &mut Document, pages_id: ObjectId) {
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
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
    let paragraph_split = Regex::new(r"\n\s*\n").unwrap();
    let paragraphs: Vec<&str> = paragraph_split.split(text).collect();

    for paragraph in paragraphs {
        format_paragraph(pages_state, paragraph);
    }

    end_text_section(pages_state);

    for image in images {
        add_image(pages_state, image);
    }

    end_text_section(pages_state);
}

fn format_paragraph(pages_state: &mut PagesState, paragraph: &str) {
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    let mut current_line = String::new();

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if string_width(&test_line) > MAX_WIDTH {
            add_line_to_page(pages_state, &current_line, LINE_HEIGHT);
            current_line = word.to_string();
        } else {
            current_line = test_line;
        }

        check_and_create_new_page(pages_state);
    }

    if !current_line.is_empty() {
        add_line_to_page(pages_state, &current_line, 0.0);
    }

    add_paragraph_spacing(pages_state, PARAGRAPH_SPACING);
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

fn check_and_create_new_page(pages_state: &mut PagesState) {
    if pages_state.y_pos < MIN_Y_POS {
        pages_state.pages.push(Content {
            operations: new_page_operations(),
        });
        pages_state.y_pos = MAX_Y_POS;
    }
}

fn add_paragraph_spacing(pages_state: &mut PagesState, paragraph_spacing: f64) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        pages_state.y_pos -= paragraph_spacing;
        last_page.operations.push(Operation::new(
            "Td",
            vec![0.into(), (-paragraph_spacing).into()],
        ));

        if pages_state.y_pos < MIN_Y_POS {
            last_page.operations.extend_from_slice(&[
                Operation::new("ET", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Td", vec![50.into(), MAX_Y_POS.into()]),
            ]);
            pages_state.pages.push(Content {
                operations: new_page_operations(),
            });
            pages_state.y_pos = MAX_Y_POS;
        }
    }
}

fn end_text_section(pages_state: &mut PagesState) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        last_page.operations.push(Operation::new("ET", vec![]));
    }
}

fn add_image(pages_state: &mut PagesState, image: &PdfImage) {
    let scale = calculate_image_scale(image, MAX_IMAGE_WIDTH, MAX_IMAGE_HEIGHT);
    let scaled_width = image.width as f64 * scale;
    let scaled_height = image.height as f64 * scale;

    if pages_state.y_pos - scaled_height < MIN_Y_POS {
        create_new_page_for_image(pages_state, MAX_Y_POS);
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
