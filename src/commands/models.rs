use anyhow::Result;

use crate::openrouter;

pub fn run(search: Option<&str>) -> Result<()> {
    println!("Fetching models from OpenRouter...");
    println!();

    let mut models = openrouter::list_models()?;

    if let Some(query) = search {
        let q = query.to_lowercase();
        models.retain(|m| m.id.to_lowercase().contains(&q) || m.name.to_lowercase().contains(&q));
    } else {
        let code_keywords = [
            "deepseek", "codestral", "coder", "qwen", "claude", "gpt-4", "gemini",
        ];
        models.retain(|m| {
            let id = m.id.to_lowercase();
            code_keywords.iter().any(|k| id.contains(k))
        });
    }

    models.sort_by(|a, b| {
        let cost_a = a.pricing.as_ref().and_then(|p| p.prompt.as_ref()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::MAX);
        let cost_b = b.pricing.as_ref().and_then(|p| p.prompt.as_ref()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::MAX);
        cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!(
        "{:<50} {:>10} {:>12} {:>12}",
        "Model ID", "Context", "Input/M", "Output/M"
    );
    println!("{}", "─".repeat(86));

    for model in &models {
        let ctx = model
            .context_length
            .map(|c| format!("{c}"))
            .unwrap_or_else(|| "—".to_string());

        let (input_cost, output_cost) = match &model.pricing {
            Some(p) => {
                let input = p.prompt.as_ref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|v| format!("${:.4}", v * 1_000_000.0))
                    .unwrap_or_else(|| "—".to_string());
                let output = p.completion.as_ref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|v| format!("${:.4}", v * 1_000_000.0))
                    .unwrap_or_else(|| "—".to_string());
                (input, output)
            }
            None => ("—".to_string(), "—".to_string()),
        };

        println!("{:<50} {:>10} {:>12} {:>12}", model.id, ctx, input_cost, output_cost);
    }

    println!();
    println!("{} model(s) found", models.len());
    println!();
    println!("Usage: cargo syntax rewrite src/main.rs --model <MODEL_ID>");
    println!("   or: cargo syntax review 5 --model <MODEL_ID>");

    Ok(())
}
