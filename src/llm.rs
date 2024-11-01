use callm::pipelines::PipelineText;
use eyre::Result;

pub fn llm_translate(snippet: &str, to: &str) -> Result<String> {
    let mut pipeline = PipelineText::builder()
        .with_location("/path/to/model")
        .build()?;

    let translation = pipeline.run(&format!(
        "Please translate the following text into {to}:\n {snippet}"
    ))?;

    Ok(translation)
}
