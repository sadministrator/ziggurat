use eyre::{eyre, Result};
use lopdf::{
    content::{Content, Operation},
    Document, Object, StringFormat,
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

pub fn edit_pdf(mut doc: Document, edit_func: impl Fn(&str) -> String) -> Result<Document> {
    for (_page_num, page_id) in doc.get_pages() {
        let content_data = doc.get_page_content(page_id)?;
        let content = Content::decode(&content_data)?;

        let mut new_operations = Vec::new();
        let mut current_font = None;
        let mut current_font_size = None;

        for operation in content.operations {
            match operation.operator.as_str() {
                "Tf" => {
                    // font selection
                    current_font = Some(
                        operation
                            .operands
                            .first()
                            .ok_or(eyre!("No font operand"))?
                            .as_name()?
                            .to_vec(),
                    );
                    current_font_size = Some(
                        operation
                            .operands
                            .get(1)
                            .ok_or(eyre!("No font size operand"))?
                            .as_f32()
                            .or_else(|_| operation.operands[1].as_i64().map(|i| i as f32))?
                            as f64,
                    );
                    new_operations.push(operation);
                }
                "Td" | "TD" => {
                    // text positioning
                    new_operations.push(operation);
                }
                "Tj" | "TJ" => {
                    // text showing
                    if let Some(text) = extract_text_from_operation(&operation) {
                        let translated_text = edit_func(&text);
                        let new_operation = create_text_operation(
                            &operation.operator,
                            &translated_text,
                            current_font.as_ref(),
                            current_font_size,
                        );
                        new_operations.push(new_operation);
                    } else {
                        new_operations.push(operation);
                    }
                }
                _ => new_operations.push(operation),
            }
        }

        let new_content = Content {
            operations: new_operations,
        };
        let new_content_data = new_content.encode()?;
        doc.change_page_content(page_id, new_content_data)?;
    }

    Ok(doc)
}

fn extract_text_from_operation(operation: &Operation) -> Option<String> {
    match operation.operator.as_str() {
        "Tj" => {
            // simple text strings
            operation.operands.first().and_then(|op| match op {
                Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
                _ => None,
            })
        }
        "TJ" => {
            // text arrays
            operation.operands.first().and_then(|op| match op {
                Object::Array(arr) => {
                    let mut text = String::new();
                    for item in arr {
                        if let Object::String(bytes, _) = item {
                            text.push_str(&String::from_utf8_lossy(bytes));
                        }
                    }
                    Some(text)
                }
                _ => None,
            })
        }
        _ => None,
    }
}

fn create_text_operation(
    operator: &str,
    text: &str,
    font: Option<&Vec<u8>>,
    font_size: Option<f64>,
) -> Operation {
    match operator {
        "Tj" => {
            // simple text string
            Operation::new(
                "Tj",
                vec![Object::String(
                    text.as_bytes().to_vec(),
                    StringFormat::Literal,
                )],
            )
        }
        "TJ" => {
            // text array
            let text_object = Object::Array(vec![Object::String(
                text.as_bytes().to_vec(),
                StringFormat::Literal,
            )]);
            Operation::new("TJ", vec![text_object])
        }
        _ => {
            // if it's neither Tj nor TJ, we'll create a Tj operation as a fallback
            Operation::new(
                "Tj",
                vec![Object::String(
                    text.as_bytes().to_vec(),
                    StringFormat::Literal,
                )],
            )
        }
    }
}
