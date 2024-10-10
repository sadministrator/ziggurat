use eyre::Result;
use lopdf::{Document, Object};

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
        let page = doc.get_object(page_id)?.as_dict()?;
        let contents = page.get(b"Contents")?;
        let mut new_content = Vec::new();

        match contents {
            &Object::Reference(id) => {
                if let Some((id, edited_data)) = edit_object(&doc, id, &edit_func) {
                    new_content.push((id, edited_data));
                }
            }
            &Object::Array(ref arr) => {
                for content in arr.iter() {
                    if let &Object::Reference(id) = content {
                        if let Some((id, edited_data)) = edit_object(&doc, id, &edit_func) {
                            new_content.push((id, edited_data));
                        }
                    }
                }
            }
            _ => {}
        }

        for (id, data) in new_content {
            let stream = doc.get_object_mut(id)?.as_stream_mut()?;
            stream.set_content(data);
        }
    }

    Ok(doc)
}

fn edit_object(
    doc: &Document,
    id: (u32, u16),
    edit_func: &impl Fn(&str) -> String,
) -> Option<((u32, u16), Vec<u8>)> {
    doc.get_object(id)
        .ok()
        .and_then(|obj| obj.as_stream().ok())
        .map(|stream| {
            let data = stream.content.clone();
            let content = String::from_utf8_lossy(&data);
            let edited_content = edit_func(&content);
            let edited_data = edited_content.into_bytes();
            (id, edited_data)
        })
}
