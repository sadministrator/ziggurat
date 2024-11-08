use eyre::{eyre, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Request {
    model: String,
    prompt: String,
    max_tokens: i32,
}

#[derive(Serialize, Deserialize)]
struct Response {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize)]
struct Choice {
    text: String,
    index: i32,
    logprobs: Option<Logprobs>,
    finish_reason: String,
}

#[derive(Serialize, Deserialize)]
struct Logprobs {
    token_logprobs: Vec<f64>,
    text_offset: i32,
}

async fn send_request(endpoint: &str, api_key: &str, request: &Request) -> Result<Response> {
    let url = format!("{}/v1/{}/completions", endpoint, request.model);
    let client = Client::new();
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(eyre!(format!(
            "API request failed: {:?}",
            response.text().await?
        )));
    }

    let inner = response.json::<Response>().await?;

    Ok(inner)
}

pub async fn translate(snippet: &str, to: &str, endpoint: &str, api_key: &str) -> Result<String> {
    let request = Request {
        model: "llama-3.2-3B".to_owned(),
        prompt: format!("Please translate the following into {}:\n{}", to, snippet),
        max_tokens: 100,
    };
    let response = send_request(endpoint, api_key, &request).await?;
    let translation = response.choices.last().unwrap().text.clone();

    Ok(translation)
}
