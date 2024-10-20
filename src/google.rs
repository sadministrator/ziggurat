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
    text: String,
    target_language: String,
    api_key: String,
) -> Result<String> {
    if is_whitespace(&text) {
        Ok(text)
    } else {
        let client = reqwest::Client::new();
        let url = format!(
            "https://translation.googleapis.com/language/translate/v2?key={}",
            api_key
        );

        let request = TranslateRequest {
            q: text.to_string(),
            target: target_language.to_string(),
        };

        let response: TranslateResponse = client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(response.data.translations[0].translated_text.clone())
    }
}

fn is_whitespace(snippet: &str) -> bool {
    snippet.chars().all(|c| c.is_whitespace())
}
