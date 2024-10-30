use std::{future::Future, sync::Arc, vec};

use eyre::Result;
use futures::{
    stream::{self, StreamExt},
    TryStreamExt,
};
use lopdf::{
    content::{Content, Operation},
    dictionary,
    xobject::PdfImage,
    Dictionary, Document, Object, ObjectId, Stream,
};
use regex::Regex;
use tokio::sync::Semaphore;

use crate::options::{PdfOptions, RequestOptions};

#[derive(Debug)]
struct PagesState {
    pages: Vec<Content>,
    y_pos: f64,
}

impl PagesState {
    fn new(options: &PdfOptions) -> Self {
        Self {
            pages: vec![Content {
                operations: new_page_operations(),
            }],
            y_pos: options.max_y_pos,
        }
    }
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

pub async fn edit_pdf<F, Fut>(
    doc: Document,
    request_options: RequestOptions,
    pdf_options: PdfOptions,
    edit_func: F,
) -> Result<Document>
where
    F: Fn(Vec<String>) -> Fut,
    Fut: Future<Output = Result<Vec<String>>>,
{
    let mut edited_doc = Document::with_version("1.5");
    let pages_id = edited_doc.new_object_id();
    let font_id = add_font(&mut edited_doc);

    let mut page_ids = Vec::with_capacity(doc.get_pages().len());
    let mut image_resources = dictionary! {};
    let mut pages_state = PagesState::new(&pdf_options);

    let semaphore = Arc::new(Semaphore::new(request_options.max_concurrency));
    let edit_func = Arc::new(edit_func);

    let pages: Vec<_> = doc.get_pages().into_iter().collect();
    let mut snippet_batches = Vec::new();
    let mut current_batch = Vec::new();

    for (page_num, page_id) in pages {
        let text = doc.extract_text(&[page_num])?;
        current_batch.push((text, page_id));

        if current_batch.len() >= request_options.batch_size {
            snippet_batches.push(std::mem::take(&mut current_batch))
        }
    }

    if !current_batch.is_empty() {
        snippet_batches.push(std::mem::take(&mut current_batch));
    }

    let results: Result<Vec<(Vec<String>, Vec<ObjectId>)>> = stream::iter(snippet_batches)
        .map(|batch| {
            let edit_func = Arc::clone(&edit_func);
            let semaphore = Arc::clone(&semaphore);
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let (snippets, page_ids): (Vec<_>, Vec<_>) = batch.into_iter().unzip();
                let edited_text = edit_func(snippets).await?;
                Ok((edited_text, page_ids))
            }
        })
        .buffer_unordered(request_options.max_concurrency)
        .try_collect()
        .await;
    let results = results?;

    for (snippets, page_ids) in results {
        for (snippet, page_id) in snippets.into_iter().zip(page_ids) {
            let images = doc.get_page_images(page_id).unwrap_or_default();
            format_content(&pdf_options, &mut pages_state, &snippet, &images);
            add_images_to_resources(&mut edited_doc, &mut image_resources, &images);
        }
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

fn format_content(
    options: &PdfOptions,
    pages_state: &mut PagesState,
    text: &str,
    images: &[PdfImage],
) {
    let paragraph_split = Regex::new(r"\n\s*\n").unwrap();
    let paragraphs: Vec<&str> = paragraph_split.split(text).collect();

    for paragraph in paragraphs {
        format_paragraph(options, pages_state, paragraph);
    }

    end_text_section(pages_state);

    for image in images {
        add_image(options, pages_state, image);
    }

    end_text_section(pages_state);
}

fn format_paragraph(options: &PdfOptions, pages_state: &mut PagesState, paragraph: &str) {
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    let mut current_line = String::new();

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if string_width(&test_line) > options.max_width {
            add_line_to_page(pages_state, &current_line, options.line_height);
            current_line = word.to_string();
        } else {
            current_line = test_line;
        }

        check_and_create_new_page(pages_state, &options);
    }

    if !current_line.is_empty() {
        add_line_to_page(pages_state, &current_line, 0.0);
    }

    add_paragraph_spacing(options, pages_state, options.paragraph_spacing);
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

fn check_and_create_new_page(pages_state: &mut PagesState, options: &PdfOptions) {
    if pages_state.y_pos < options.min_y_pos {
        pages_state.pages.push(Content {
            operations: new_page_operations(),
        });
        pages_state.y_pos = options.max_y_pos;
    }
}

fn add_paragraph_spacing(
    options: &PdfOptions,
    pages_state: &mut PagesState,
    paragraph_spacing: f64,
) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        pages_state.y_pos -= paragraph_spacing;
        last_page.operations.push(Operation::new(
            "Td",
            vec![0.into(), (-paragraph_spacing).into()],
        ));

        if pages_state.y_pos < options.min_y_pos {
            last_page.operations.extend_from_slice(&[
                Operation::new("ET", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Td", vec![50.into(), options.max_y_pos.into()]),
            ]);
            pages_state.pages.push(Content {
                operations: new_page_operations(),
            });
            pages_state.y_pos = options.max_y_pos;
        }
    }
}

fn end_text_section(pages_state: &mut PagesState) {
    if let Some(last_page) = pages_state.pages.last_mut() {
        last_page.operations.push(Operation::new("ET", vec![]));
    }
}

fn add_image(options: &PdfOptions, pages_state: &mut PagesState, image: &PdfImage) {
    let scale = calculate_image_scale(image, options.max_image_width, options.max_image_height);
    let scaled_width = image.width as f64 * scale;
    let scaled_height = image.height as f64 * scale;

    if pages_state.y_pos - scaled_height < options.min_y_pos {
        create_new_page_for_image(pages_state, options.max_y_pos);
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
    const CHAR_WIDTH: f64 = 7.0;
    s.len() as f64 * CHAR_WIDTH
}

fn new_page_operations() -> Vec<Operation> {
    vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new("Td", vec![50.into(), 750.into()]),
    ]
}
