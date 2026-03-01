use anyhow::Result;

use crate::openrouter;

pub fn run(search: Option<&str>) -> Result<()> {
    println!("Fetching models from OpenRouter...");
    println!();

    let all_models = openrouter::list_models()?;

    let mut models = all_models;
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
        let cost_a = prompt_cost(a).unwrap_or(f64::MAX);
        let cost_b = prompt_cost(b).unwrap_or(f64::MAX);
        cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!(
        "{:<50} {:>10} {:>12} {:>12}",
        "Model ID", "Context", "Input/M", "Output/M"
    );
    println!("{}", "─".repeat(86));

    for model in &models {
        println!(
            "{:<50} {:>10} {:>12} {:>12}",
            model.id,
            format_context(model),
            format_input_cost(model),
            format_output_cost(model),
        );
    }

    println!();
    println!("{} model(s) found", models.len());

    if search.is_none() {
        print_recommendations(&models);
    }

    println!();
    println!("Usage: cargo syntax rewrite src/main.rs --model <MODEL_ID>");
    println!("   or: cargo syntax review 5 --model <MODEL_ID>");

    Ok(())
}

fn print_recommendations(models: &[openrouter::Model]) {
    let picks: &[(&str, &str, &[&str])] = &[
        ("Free", "free, good for trying out", &["qwen/qwen3-coder:free", "deepseek/deepseek-chat:free"]),
        ("Cheap", "best value for code tasks", &["deepseek/deepseek-chat", "deepseek/deepseek-chat-v3-0324"]),
        ("Best", "highest quality rewrites", &["anthropic/claude-sonnet-4", "anthropic/claude-sonnet-4.5"]),
        ("Large", "1M+ context for huge files", &["google/gemini-2.5-flash", "google/gemini-2.5-pro"]),
    ];

    println!();
    println!("Recommended for cargo-syntax:");

    for (label, desc, candidates) in picks {
        let found = candidates.iter().find_map(|id| models.iter().find(|m| m.id == *id));
        let Some(model) = found else { continue };

        let ctx = model.context_length.map(|c| format!("{c}")).unwrap_or_default();
        let cost = format_input_cost(model);
        println!("  {label:<6} {:<40} — {cost}, {ctx} ctx, {desc}", model.id);
    }
}

fn prompt_cost(model: &openrouter::Model) -> Option<f64> {
    model.pricing.as_ref()?.prompt.as_ref()?.parse().ok()
}

fn format_context(model: &openrouter::Model) -> String {
    model.context_length.map(|c| format!("{c}")).unwrap_or_else(|| "—".to_string())
}

fn format_input_cost(model: &openrouter::Model) -> String {
    model.pricing.as_ref()
        .and_then(|p| p.prompt.as_ref())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| format!("${:.4}", v * 1_000_000.0))
        .unwrap_or_else(|| "—".to_string())
}

fn format_output_cost(model: &openrouter::Model) -> String {
    model.pricing.as_ref()
        .and_then(|p| p.completion.as_ref())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| format!("${:.4}", v * 1_000_000.0))
        .unwrap_or_else(|| "—".to_string())
}
