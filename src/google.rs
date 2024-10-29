use eyre::Result;
use serde::{Deserialize, Serialize};

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

pub async fn translate_text(
    snippets: Vec<String>,
    target_language: String,
    api_key: String,
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
