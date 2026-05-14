use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use securellm_core::*;
use std::io;
use tracing_subscriber;

mod provider_factory;
mod repl;

#[derive(Parser)]
#[command(name = "securellm")]
#[command(author, version, about = "Secure bridge for LLM communication", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Launch TUI mode (Terminal User Interface)
    #[arg(long)]
    tui: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a chat request to an LLM provider
    Chat {
        /// Provider to use (deepseek, openai, anthropic, ollama)
        #[arg(short, long, value_parser = ["deepseek", "openai", "anthropic", "ollama"])]
        provider: String,

        /// Model to use
        #[arg(short, long)]
        model: String,

        /// Message to send
        message: String,

        /// API key (can also use environment variable)
        #[arg(long)]
        api_key: Option<String>,

        /// System prompt
        #[arg(long)]
        system: Option<String>,

        /// Maximum tokens
        #[arg(long, default_value = "1024")]
        max_tokens: u32,

        /// Temperature
        #[arg(long, default_value = "0.7")]
        temperature: f32,
    },

    /// Check health of a provider
    Health {
        /// Provider to check
        #[arg(value_parser = ["deepseek", "openai", "anthropic", "ollama"])]
        provider: String,

        /// API key
        #[arg(long)]
        api_key: Option<String>,
    },

    /// List available models from a provider
    Models {
        /// Provider to query
        #[arg(value_parser = ["deepseek", "openai", "anthropic", "ollama"])]
        provider: String,

        /// API key
        #[arg(long)]
        api_key: Option<String>,
    },

    /// Show provider capabilities
    Info {
        /// Provider name
        #[arg(value_parser = ["deepseek", "openai", "anthropic", "ollama"])]
        provider: String,
    },

    /// Start interactive chat session
    Repl {
        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,

        /// API key
        #[arg(long)]
        api_key: Option<String>,

        /// System prompt
        #[arg(long)]
        system: Option<String>,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completion for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Check if TUI mode is requested
    if cli.tui {
        tracing::info!("Launching TUI mode");
        return securellm_tui::run().await;
    }

    // Handle CLI commands
    match cli.command {
        Some(Commands::Chat {
            provider,
            model,
            message,
            api_key,
            system,
            max_tokens,
            temperature,
        }) => {
            handle_chat(
                provider,
                model,
                message,
                api_key,
                system,
                max_tokens,
                temperature,
            )
            .await?;
        }
        Some(Commands::Health { provider, api_key }) => {
            handle_health(provider, api_key).await?;
        }
        Some(Commands::Models { provider, api_key }) => {
            handle_models(provider, api_key).await?;
        }
        Some(Commands::Info { provider }) => {
            handle_info(provider).await?;
        }
        Some(Commands::Repl {
            provider,
            model,
            api_key,
            system,
        }) => {
            repl::run_repl(provider, model, api_key, system).await?;
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "securellm", &mut io::stdout());
        }
        None => {
            // No command provided - show help
            let mut cmd = Cli::command();
            cmd.print_help()?;
        }
    }

    Ok(())
}

// Handler functions

async fn handle_chat(
    provider: String,
    model: String,
    message: String,
    api_key: Option<String>,
    system: Option<String>,
    max_tokens: u32,
    temperature: f32,
) -> Result<()> {
    let api_key = provider_factory::resolve_api_key(&provider, api_key)?;

    println!("🔒 SecureLLM Bridge");
    println!("Provider: {}", provider);
    println!("Model: {}", model);
    println!();

    let provider_impl = provider_factory::build_provider(&provider, api_key, true)?;

    // Build request
    let mut request = Request::new(provider.clone(), model)
        .add_message(Message {
            role: MessageRole::User,
            content: MessageContent::Text(message),
            name: None,
            metadata: None,
        })
        .with_max_tokens(max_tokens)
        .with_temperature(temperature);

    if let Some(sys) = system {
        request = request.with_system(sys);
    }

    // Send request
    println!("⏳ Sending request...");
    let response = provider_impl
        .send_request(request)
        .await
        .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

    // Print response
    println!();
    println!("✅ Response:");
    println!("{}", response.text().map_err(|e| anyhow::anyhow!(e))?);
    println!();
    println!("📊 Usage:");
    println!("  Prompt tokens: {}", response.usage.prompt_tokens);
    println!("  Completion tokens: {}", response.usage.completion_tokens);
    println!("  Total tokens: {}", response.usage.total_tokens);
    println!(
        "  Processing time: {}ms",
        response.metadata.processing_time_ms
    );

    Ok(())
}

async fn handle_health(provider: String, api_key: Option<String>) -> Result<()> {
    let api_key = provider_factory::resolve_api_key(&provider, api_key)?;
    let provider_impl = provider_factory::build_provider(&provider, api_key, false)?;

    println!("🏥 Checking {} health...", provider);
    let health = provider_impl
        .health_check()
        .await
        .map_err(|e| anyhow::anyhow!("Health check failed: {}", e))?;

    let status_icon = match health.status {
        HealthStatus::Healthy => "✅",
        HealthStatus::Degraded => "⚠️",
        HealthStatus::Unhealthy => "❌",
        HealthStatus::Unknown => "❓",
    };

    println!("{} Status: {:?}", status_icon, health.status);
    if let Some(latency) = health.latency_ms {
        println!("⏱️  Latency: {}ms", latency);
    }

    Ok(())
}

async fn handle_models(provider: String, api_key: Option<String>) -> Result<()> {
    let api_key = provider_factory::resolve_api_key(&provider, api_key)?;
    let provider_impl = provider_factory::build_provider(&provider, api_key, false)?;

    println!("📋 Available {} models:", provider);
    println!();

    let models = provider_impl
        .list_models()
        .await
        .map_err(|e| anyhow::anyhow!("List models failed: {}", e))?;

    for model in models {
        println!("🤖 {}", model.id);
        println!("   Name: {}", model.name);
        if let Some(desc) = &model.description {
            println!("   Description: {}", desc);
        }
        if let Some(ctx) = model.context_window {
            println!("   Context: {} tokens", ctx);
        }
        if let Some(pricing) = &model.pricing {
            println!(
                "   Pricing: ${:.4}/1K input, ${:.4}/1K output",
                pricing.input_cost_per_1k, pricing.output_cost_per_1k
            );
        }
        println!();
    }

    Ok(())
}

async fn handle_info(provider: String) -> Result<()> {
    println!("ℹ️  Provider Information: {}", provider);
    println!();

    let provider_impl = provider_factory::build_info_provider(&provider)?;
    let caps = provider_impl.capabilities();

    println!("Capabilities:");
    println!(
        "  ✓ Streaming: {}",
        if caps.streaming { "Yes" } else { "No" }
    );
    println!(
        "  ✓ Function Calling: {}",
        if caps.function_calling { "Yes" } else { "No" }
    );
    println!("  ✓ Vision: {}", if caps.vision { "Yes" } else { "No" });
    println!(
        "  ✓ Embeddings: {}",
        if caps.embeddings { "Yes" } else { "No" }
    );
    println!(
        "  ✓ System Prompts: {}",
        if caps.supports_system_prompts {
            "Yes"
        } else {
            "No"
        }
    );

    if let Some(max) = caps.max_tokens {
        println!("  ✓ Max Output Tokens: {}", max);
    }
    if let Some(ctx) = caps.max_context_window {
        println!("  ✓ Max Context Window: {}", ctx);
    }

    Ok(())
}
