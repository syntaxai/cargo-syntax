use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

#[derive(Serialize)]
struct Request {
    model: String,
    messages: Vec<Message>,
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

pub fn chat(model: &str, system: &str, prompt: &str) -> Result<String> {
    let key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set — get one at https://openrouter.ai/keys"))?;

    let body = Request {
        model: model.to_string(),
        messages: vec![
            Message { role: "system".to_string(), content: system.to_string() },
            Message { role: "user".to_string(), content: prompt.to_string() },
        ],
    };

    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .http_status_as_error(false)
            .build(),
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
