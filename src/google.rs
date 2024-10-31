use std::{fs::File, io::Read};

use base64::{prelude::BASE64_STANDARD, Engine};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
struct TranslateRequest {
    q: String,
    target: String,
}

#[derive(Deserialize)]
struct TranslateResponse {
    data: TranslateData,
}

#[derive(Deserialize)]
struct TranslateData {
    translations: Vec<Translation>,
}

#[derive(Deserialize)]
struct Translation {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

async fn translate_pdf(
    pdf_path: &str,
    target_language: &str,
    project_id: &str,
    api_key: &str,
) -> Result<Vec<u8>> {
    let mut file = File::open(pdf_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let base64_content = BASE64_STANDARD.encode(&buffer);

    let body = json!({
        "documentInputConfig": {
            "content": base64_content,
            "mimeType": "application/pdf"
        },
        "targetLanguageCode": target_language,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&format!(
            "https://translation.googleapis.com/v3/projects/{}/locations/global:translateDocument",
            project_id
        ))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(eyre!(format!(
            "API request failed: {:?}",
            response.text().await?
        )));
    }

    let response_body: serde_json::Value = response.json().await?;
    let translated_content = response_body["documentOutputConfig"]["pdfOutputConfig"]["pdfData"]
        .as_str()
        .ok_or(eyre!("Failed to get translated content"))?;

    let decoded_content = BASE64_STANDARD.decode(translated_content)?;

    Ok(decoded_content)
}

pub async fn translate_text(
    snippets: Vec<String>,
    target_language: &str,
    api_key: &str,
) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://translation.googleapis.com/language/translate/v2?key={}",
        api_key
    );

    let requests: Vec<TranslateRequest> = snippets
        .into_iter()
        .filter(|s| !is_whitespace(s))
        .map(|s| TranslateRequest {
            q: s.to_string(),
            target: target_language.to_string(),
        })
        .collect();

    let response: TranslateResponse = client
        .post(&url)
        .json(&requests)
        .send()
        .await?
        .json()
        .await?;

    Ok(response
        .data
        .translations
        .into_iter()
        .map(|t| t.translated_text)
        .collect())
}

fn is_whitespace(snippet: &str) -> bool {
    snippet.chars().all(|c| c.is_whitespace())
}
