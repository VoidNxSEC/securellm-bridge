use anyhow::{bail, Result};
use securellm_core::LLMProvider;
use securellm_providers::{
    anthropic::{AnthropicConfig, AnthropicProvider},
    deepseek::{DeepSeekConfig, DeepSeekProvider},
    gemini::{GeminiConfig, GeminiProvider},
    groq::{GroqConfig, GroqProvider},
    llamacpp::LlamaCppProvider,
    ml_ops::MlOpsProvider,
    nvidia::{NvidiaConfig, NvidiaProvider},
    openai::{OpenAIConfig, OpenAIProvider},
};

pub const IMPLEMENTED_PROVIDERS: &[&str] = &[
    "deepseek",
    "openai",
    "anthropic",
    "gemini",
    "groq",
    "llamacpp",
    "nvidia",
    "mlops",
];

pub fn implemented_providers_display() -> String {
    IMPLEMENTED_PROVIDERS.join(", ")
}

pub fn default_model(provider: &str) -> Option<&'static str> {
    match provider {
        "deepseek" => Some("deepseek-chat"),
        "openai" => Some("gpt-4o-mini"),
        "anthropic" => Some("claude-sonnet-4-20250514"),
        "gemini" => Some("gemini-2.0-flash"),
        "groq" => Some("llama-3.3-70b-versatile"),
        "llamacpp" => Some("local-model"),
        "nvidia" => Some("meta/llama-3.3-70b-instruct"),
        "mlops" => Some("local-model"),
        _ => None,
    }
}

pub fn resolve_api_key(provider: &str, api_key: Option<String>) -> Result<String> {
    resolve_api_key_with(provider, api_key, |name| std::env::var(name).ok())
}

fn resolve_api_key_with(
    provider: &str,
    api_key: Option<String>,
    mut lookup: impl FnMut(&str) -> Option<String>,
) -> Result<String> {
    if let Some(api_key) = api_key {
        return Ok(api_key);
    }

    // Local providers don't need API keys
    if is_local_provider(provider) {
        return Ok("local".to_string());
    }

    let provider_env = provider_env_name(provider, "API_KEY");
    lookup(&provider_env)
        .or_else(|| lookup("SECURELLM_API_KEY"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "API key is required for {}. Use --api-key or set {} / SECURELLM_API_KEY",
                provider,
                provider_env
            )
        })
}

fn is_local_provider(provider: &str) -> bool {
    matches!(provider, "llamacpp" | "mlops")
}

pub fn build_provider(
    provider: &str,
    api_key: String,
    logging: bool,
) -> Result<Box<dyn LLMProvider>> {
    match provider {
        "deepseek" => {
            let mut config = DeepSeekConfig::new(api_key).with_logging(logging);
            if let Ok(endpoint) = std::env::var(provider_env_name(provider, "BASE_URL")) {
                config = config.with_endpoint(endpoint);
            }
            Ok(Box::new(DeepSeekProvider::new(config)?))
        }
        "openai" => {
            let mut config = OpenAIConfig::new(api_key).with_logging(logging);
            if let Ok(endpoint) = std::env::var(provider_env_name(provider, "BASE_URL")) {
                config = config.with_endpoint(endpoint);
            }
            if let Ok(org_id) = std::env::var(provider_env_name(provider, "ORGANIZATION_ID")) {
                config = config.with_organization(org_id);
            }
            Ok(Box::new(OpenAIProvider::new(config)?))
        }
        "anthropic" => {
            let config = AnthropicConfig::new(api_key).with_logging(logging);
            Ok(Box::new(AnthropicProvider::new(config)?))
        }
        "gemini" => {
            let config = GeminiConfig::new(api_key);
            Ok(Box::new(GeminiProvider::new(config)?))
        }
        "groq" => {
            let config = GroqConfig::new(api_key);
            Ok(Box::new(GroqProvider::new(config)?))
        }
        "nvidia" => {
            let config = NvidiaConfig::new(api_key);
            Ok(Box::new(NvidiaProvider::new(config)?))
        }
        "llamacpp" => {
            let port: u16 = std::env::var("LLAMACPP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8081);
            let model =
                std::env::var("LLAMACPP_MODEL_NAME").unwrap_or_else(|_| "local-model".to_string());
            Ok(Box::new(LlamaCppProvider::new(port, model)?))
        }
        "mlops" => {
            let base_url = std::env::var("MLOPS_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:9000".to_string());
            Ok(Box::new(MlOpsProvider::new(base_url)?))
        }
        _ => bail!(
            "Provider '{}' not yet implemented. Available: {}",
            provider,
            implemented_providers_display()
        ),
    }
}

pub fn build_info_provider(provider: &str) -> Result<Box<dyn LLMProvider>> {
    if is_local_provider(provider) {
        build_provider(provider, String::new(), false)
    } else {
        build_provider(provider, "dummy".to_string(), false)
    }
}

fn provider_env_name(provider: &str, suffix: &str) -> String {
    format!(
        "{}_{}",
        provider.replace('-', "_").to_ascii_uppercase(),
        suffix
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Provider Registry ────────────────────────────────────────

    #[test]
    fn all_eight_providers_listed() {
        assert_eq!(IMPLEMENTED_PROVIDERS.len(), 8);
        assert!(IMPLEMENTED_PROVIDERS.contains(&"deepseek"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"openai"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"anthropic"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"gemini"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"groq"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"llamacpp"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"nvidia"));
        assert!(IMPLEMENTED_PROVIDERS.contains(&"mlops"));
    }

    #[test]
    fn all_providers_have_default_model() {
        for provider in IMPLEMENTED_PROVIDERS {
            let model = default_model(provider);
            assert!(
                model.is_some(),
                "Provider '{}' should have a default model",
                provider
            );
        }
    }

    #[test]
    fn implemented_providers_display_is_human_readable() {
        let display = implemented_providers_display();
        assert!(display.contains("deepseek"));
        assert!(display.contains("openai"));
        assert!(display.contains("mlops"));
    }

    // ── Info Providers ───────────────────────────────────────────

    #[test]
    fn info_provider_supports_openai_capabilities() {
        let provider = build_info_provider("openai").unwrap();
        assert_eq!(provider.name(), "openai");
        assert!(provider.capabilities().function_calling);
    }

    #[test]
    fn info_provider_deepseek() {
        let provider = build_info_provider("deepseek").unwrap();
        assert_eq!(provider.name(), "deepseek");
    }

    #[test]
    fn info_provider_anthropic() {
        let provider = build_info_provider("anthropic").unwrap();
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn info_provider_gemini() {
        let provider = build_info_provider("gemini").unwrap();
        assert_eq!(provider.name(), "gemini");
    }

    #[test]
    fn info_provider_groq() {
        let provider = build_info_provider("groq").unwrap();
        assert_eq!(provider.name(), "groq");
    }

    #[test]
    fn info_provider_nvidia() {
        let provider = build_info_provider("nvidia").unwrap();
        assert_eq!(provider.name(), "nvidia");
    }

    #[test]
    fn info_provider_llamacpp() {
        let provider = build_info_provider("llamacpp").unwrap();
        assert_eq!(provider.name(), "llamacpp");
    }

    #[test]
    fn info_provider_mlops() {
        let provider = build_info_provider("mlops").unwrap();
        assert_eq!(provider.name(), "ml-ops");
    }

    // ── API Key Resolution ───────────────────────────────────────

    #[test]
    fn local_providers_dont_need_api_key() {
        assert!(is_local_provider("llamacpp"));
        assert!(is_local_provider("mlops"));
        assert!(!is_local_provider("openai"));
    }

    #[test]
    fn local_provider_resolves_empty_key() {
        let key = resolve_api_key_with("llamacpp", None, |_| None).unwrap();
        assert_eq!(key, "local");

        let key = resolve_api_key_with("mlops", None, |_| None).unwrap();
        assert_eq!(key, "local");
    }

    #[test]
    fn provider_env_name_normalizes_provider_names() {
        assert_eq!(provider_env_name("ml-ops", "API_KEY"), "ML_OPS_API_KEY");
        assert_eq!(
            provider_env_name("deepseek", "BASE_URL"),
            "DEEPSEEK_BASE_URL"
        );
    }

    #[test]
    fn provider_specific_key_wins_over_generic_env() {
        let key = resolve_api_key_with("openai", None, |name| match name {
            "OPENAI_API_KEY" => Some("openai-key".to_string()),
            "SECURELLM_API_KEY" => Some("generic-key".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(key, "openai-key");
    }

    // ── Default Models ───────────────────────────────────────────

    #[test]
    fn deepseek_default_model() {
        assert_eq!(default_model("deepseek"), Some("deepseek-chat"));
    }

    #[test]
    fn openai_default_model() {
        assert_eq!(default_model("openai"), Some("gpt-4o-mini"));
    }

    #[test]
    fn anthropic_default_model() {
        assert_eq!(default_model("anthropic"), Some("claude-sonnet-4-20250514"));
    }

    #[test]
    fn unknown_provider_has_no_default() {
        assert_eq!(default_model("nonexistent"), None);
    }
}
