use crate::provider_factory;
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Select;
use securellm_core::{Message, MessageContent, MessageRole, Request};
use std::io::{self, Write};

pub async fn run_repl(
    provider_name: Option<String>,
    model_name: Option<String>,
    api_key: Option<String>,
    system_prompt: Option<String>,
) -> Result<()> {
    print_banner();

    // 1. Configure Provider
    let api_key = match api_key {
        Some(k) => k,
        None => rpassword::prompt_password("Enter API Key: ")?,
    };

    let provider_name = match provider_name {
        Some(p) => p,
        None => Select::new(
            "Select Provider:",
            provider_factory::IMPLEMENTED_PROVIDERS.to_vec(),
        )
        .prompt()?
        .to_string(),
    };

    let provider = provider_factory::build_provider(&provider_name, api_key, true)?;

    let model = match model_name {
        Some(m) => m,
        None => provider_factory::default_model(&provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not yet implemented", provider_name))?
            .to_string(),
    };

    println!(
        "🔌 Connected to {} ({})",
        provider_name.cyan(),
        model.yellow()
    );
    println!("{}", "Type 'exit' or 'quit' to end session.".dimmed());
    println!();

    // 2. Chat Loop
    let mut history: Vec<Message> = Vec::new();

    // Add system prompt if present
    if let Some(sys) = system_prompt {
        history.push(Message {
            role: MessageRole::System,
            content: MessageContent::Text(sys),
            name: None,
            metadata: None,
        });
    }

    loop {
        // User Input
        print!("{}", "You ➤ ".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        // Add to history
        history.push(Message {
            role: MessageRole::User,
            content: MessageContent::Text(input.to_string()),
            name: None,
            metadata: None,
        });

        // Spinner
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.blue} {msg}")?,
        );
        pb.set_message("Thinking...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        // Build Request
        // Note: We need to clone history for the request because securellm_core uses owned types
        // Optimization: In a real app we might want to limit history window
        let request = Request {
            id: uuid::Uuid::new_v4(),
            provider: provider_name.clone(),
            model: model.clone(),
            messages: history.clone(),
            parameters: Default::default(),
            metadata: Default::default(),
            system: None,
        };

        // Send Request
        let response_result = provider.send_request(request).await;

        pb.finish_and_clear();

        match response_result {
            Ok(response) => {
                let content = response
                    .text()
                    .unwrap_or_else(|e| format!("Error getting text: {}", e));

                // Render Text
                println!("{}", "Assistant ➤".blue().bold());
                println!("{}", content); // Raw text output for now
                println!();

                // Add to history
                history.push(Message {
                    role: MessageRole::Assistant,
                    content: MessageContent::Text(content),
                    name: None,
                    metadata: None,
                });

                // Show usage stats (dimmed)
                let stats = format!(
                    "tokens: {} ({} in / {} out) | time: {}ms",
                    response.usage.total_tokens,
                    response.usage.prompt_tokens,
                    response.usage.completion_tokens,
                    response.metadata.processing_time_ms
                );
                println!("{}", stats.dimmed().italic());
                println!();
            }
            Err(e) => {
                println!("{} {}", "Error:".red().bold(), e);
                // Don't add failed message to history
            }
        }
    }

    println!("👋 Session ended.");
    Ok(())
}

fn print_banner() {
    println!(
        "{}",
        r#"
   _____                            __    __    __  __ 
  / ___/___  _______  __________   / /   / /   /  |/  |
  \__ \/ _ \/ ___/ / / / ___/ _ \ / /   / /   / /|_/ / 
 ___/ /  __/ /__/ /_/ / /  /  __// /___/ /___/ /  / /  
/____/\___/\___/\__,_/_/   \___//_____/_____/_/  /_/   
    "#
        .cyan()
    );
    println!("Secure Bridge v{}", env!("CARGO_PKG_VERSION"));
    println!();
}
