use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BASE_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

#[derive(Serialize)]
struct Request {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
    json_schema: JsonSchemaWrapper,
}

#[derive(Serialize)]
struct JsonSchemaWrapper {
    name: String,
    strict: bool,
    schema: Value,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct Response {
    choices: Option<Vec<Choice>>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub context_length: Option<u64>,
    pub pricing: Option<Pricing>,
}

#[derive(Deserialize)]
pub struct Pricing {
    pub prompt: Option<String>,
    pub completion: Option<String>,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

pub fn list_models() -> Result<Vec<Model>> {
    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder().http_status_as_error(false).build(),
    );

    let mut response = agent.get(MODELS_URL).call()?;

    let status = response.status();
    if status != 200 {
        let body = response.body_mut().read_to_string()?;
        bail!("OpenRouter API error (HTTP {status}): {body}");
    }

    let resp: ModelsResponse = response.body_mut().read_json()?;
    Ok(resp.data)
}

pub fn chat(model: &str, system: &str, prompt: &str) -> Result<String> {
    chat_with_options(model, system, prompt, None, None)
}

pub fn chat_json<T: serde::de::DeserializeOwned>(
    model: &str,
    system: &str,
    prompt: &str,
    schema_name: &str,
    schema: Value,
) -> Result<T> {
    let format = ResponseFormat {
        r#type: "json_schema".to_string(),
        json_schema: JsonSchemaWrapper { name: schema_name.to_string(), strict: true, schema },
    };

    let raw = chat_with_options(model, system, prompt, Some(format), Some(4096))?;
    serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("Failed to parse structured response: {e}"))
}

fn chat_with_options(
    model: &str,
    system: &str,
    prompt: &str,
    response_format: Option<ResponseFormat>,
    max_tokens: Option<u32>,
) -> Result<String> {
    let key = std::env::var("OPENROUTER_API_KEY").map_err(|_| {
        anyhow::anyhow!("OPENROUTER_API_KEY not set — get one at https://openrouter.ai/keys")
    })?;

    let body = Request {
        model: model.to_string(),
        messages: vec![
            Message { role: "system".to_string(), content: system.to_string() },
            Message { role: "user".to_string(), content: prompt.to_string() },
        ],
        response_format,
        max_tokens,
    };

    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder().http_status_as_error(false).build(),
    );

    let mut response = agent
        .post(BASE_URL)
        .header("Authorization", &format!("Bearer {key}"))
        .header("X-OpenRouter-Title", "cargo-syntax")
        .send_json(&body)?;

    let status = response.status();
    if status != 200 {
        let body = response.body_mut().read_to_string()?;
        bail!("OpenRouter API error (HTTP {status}): {body}");
    }

    let resp: Response = response.body_mut().read_json()?;

    if let Some(err) = resp.error {
        bail!("OpenRouter API error: {}", err.message);
    }

    resp.choices
        .and_then(|c| c.into_iter().next())
        .map(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("Empty response from OpenRouter"))
}
