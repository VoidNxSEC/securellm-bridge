use anyhow::{Context, Result};
use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use prometheus::{CounterVec, HistogramVec, Opts, HistogramOpts, Registry};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::services::audit::SqliteAuditSink;
use crate::services::nats::NatsPublisher;
use securellm_core::intelligence::{
    PricingRegistry, PricingTier, QoSObservatory, SmartRoutingEngine,
};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: SqlitePool,
    pub redis_pool: RedisPool,
    pub provider_manager: Arc<ProviderManager>,
    pub metrics: Arc<MetricsCollector>,
    pub nats_publisher: Arc<NatsPublisher>,
    pub audit_logger: securellm_core::audit::AuditLogger,
    pub rate_limiter: Arc<securellm_core::rate_limit::RateLimiter>,
    pub pricing_registry: Arc<PricingRegistry>,
    pub qos_observatory: Arc<QoSObservatory>,
    pub routing_engine: Arc<SmartRoutingEngine>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let db_pool = SqlitePoolOptions::new()
            .max_connections(config.database.max_connections)
            .min_connections(config.database.min_connections)
            .connect(&config.database.url)
            .await
            .context("Failed to connect to database")?;

        sqlx::migrate!("./migrations")
            .run(&db_pool)
            .await
            .context("Failed to run migrations")?;

        let redis_config = RedisConfig::from_url(&config.redis.url);
        let redis_pool = redis_config
            .create_pool(Some(Runtime::Tokio1))
            .context("Failed to create Redis pool")?;

        let provider_manager = Arc::new(ProviderManager::new(config.clone()).await?);
        let metrics = Arc::new(MetricsCollector::new());

        // NATS publisher (non-fatal if NATS is unavailable)
        let nats_url = std::env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let nats_publisher = Arc::new(NatsPublisher::connect(&nats_url).await);

        let audit_sink = Arc::new(SqliteAuditSink::new(db_pool.clone()));
        let audit_logger = securellm_core::audit::AuditLogger::with_sink(audit_sink);

        let rate_limiter = Arc::new(securellm_core::rate_limit::RateLimiter::default());

        let pricing_registry = Arc::new(PricingRegistry::new());
        pricing_registry.load_from_config(vec![
            PricingTier {
                provider: "deepseek".to_string(),
                model_pattern: "deepseek-chat".to_string(),
                input_cost_per_1m: 0.14,
                output_cost_per_1m: 0.28,
                effective_date: "2025-01-01".to_string(),
            },
            PricingTier {
                provider: "gemini".to_string(),
                model_pattern: "gemini-2.0-pro".to_string(),
                input_cost_per_1m: 0.60,
                output_cost_per_1m: 1.80,
                effective_date: "2026-01-01".to_string(),
            },
            PricingTier {
                provider: "groq".to_string(),
                model_pattern: "llama-3.3-70b".to_string(),
                input_cost_per_1m: 0.30,
                output_cost_per_1m: 0.40,
                effective_date: "2026-01-01".to_string(),
            },
        ]);

        let qos_observatory = Arc::new(QoSObservatory::new(2000, 0.05));
        let routing_engine = Arc::new(SmartRoutingEngine::new(
            pricing_registry.clone(),
            qos_observatory.clone(),
        ));

        Ok(Arc::new(Self {
            config: Arc::new(config),
            db_pool,
            redis_pool,
            provider_manager,
            metrics,
            nats_publisher,
            audit_logger,
            rate_limiter,
            pricing_registry,
            qos_observatory,
            routing_engine,
        }))
    }
}

use securellm_core::LLMProvider;
use securellm_providers::anthropic::{AnthropicConfig, AnthropicProvider};
use securellm_providers::deepseek::{DeepSeekConfig, DeepSeekProvider};
use securellm_providers::gemini::{GeminiConfig, GeminiProvider};
use securellm_providers::groq::{GroqConfig, GroqProvider};
use securellm_providers::llamacpp::LlamaCppProvider;
use securellm_providers::nvidia::{NvidiaConfig, NvidiaProvider};
use securellm_providers::openai::{OpenAIConfig, OpenAIProvider};

pub struct ProviderManager {
    providers: RwLock<Vec<Arc<dyn LLMProvider>>>,
    circuit_breakers: RwLock<std::collections::HashMap<String, CircuitBreaker>>,
}

impl ProviderManager {
    pub async fn new(config: Config) -> Result<Self> {
        let mut providers: Vec<Arc<dyn LLMProvider>> = Vec::new();
        let mut breakers = std::collections::HashMap::new();

        // Using a block to contain the mutable borrow if I were to use a closure,
        // but to avoid lifetime issues, I'll just append directly.

        if let Some(cfg) = &config.providers.deepseek {
            if cfg.enabled {
                let mut p_config = DeepSeekConfig::new(&cfg.api_key);
                if let Some(url) = &cfg.base_url {
                    p_config = p_config.with_endpoint(url);
                }
                if let Ok(p) = DeepSeekProvider::new(p_config) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "deepseek".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        if let Some(cfg) = &config.providers.gemini {
            if cfg.enabled {
                let mut p_config = GeminiConfig::new(&cfg.api_key);
                if let Some(url) = &cfg.base_url {
                    p_config.endpoint = url.clone();
                }
                if let Ok(p) = GeminiProvider::new(p_config) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "gemini".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        if let Some(cfg) = &config.providers.groq {
            if cfg.enabled {
                let mut p_config = GroqConfig::new(&cfg.api_key);
                if let Some(url) = &cfg.base_url {
                    p_config.endpoint = url.clone();
                }
                if let Ok(p) = GroqProvider::new(p_config) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "groq".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        if let Some(cfg) = &config.providers.nvidia {
            if cfg.enabled {
                let mut p_config = NvidiaConfig::new(&cfg.api_key);
                if let Some(url) = &cfg.base_url {
                    p_config.endpoint = url.clone();
                }
                if let Ok(p) = NvidiaProvider::new(p_config) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "nvidia".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        // Add LlamaCpp provider (local inference)
        if let Some(cfg) = &config.providers.llamacpp {
            if cfg.enabled {
                // Parse port from base_url (e.g., "http://localhost:5001")
                let port = cfg.base_url
                    .split(':')
                    .last()
                    .and_then(|p| p.parse::<u16>().ok())
                    .unwrap_or(5001);

                let model_name = std::env::var("LLAMACPP_MODEL_NAME")
                    .unwrap_or_else(|_| "local-model".to_string());

                if let Ok(p) = LlamaCppProvider::new(port, model_name) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "llamacpp".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        // Add OpenAI provider
        if let Some(cfg) = &config.providers.openai {
            if cfg.enabled {
                let mut p_config = OpenAIConfig::new(&cfg.api_key);
                if let Some(url) = &cfg.base_url {
                    p_config.endpoint = url.clone();
                }
                if let Ok(p) = OpenAIProvider::new(p_config) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "openai".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        // Add Anthropic provider
        if let Some(cfg) = &config.providers.anthropic {
            if cfg.enabled {
                let mut p_config = AnthropicConfig::new(&cfg.api_key);
                if let Some(url) = &cfg.base_url {
                    p_config.endpoint = url.clone();
                }
                if let Ok(p) = AnthropicProvider::new(p_config) {
                    providers.push(Arc::new(p));
                    breakers.insert(
                        "anthropic".to_string(),
                        CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    );
                }
            }
        }

        Ok(Self {
            providers: RwLock::new(providers),
            circuit_breakers: RwLock::new(breakers),
        })
    }

    pub async fn get_provider(&self, name: &str) -> Option<Arc<dyn LLMProvider>> {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(name) {
            if !breaker.can_attempt() {
                tracing::warn!("Circuit breaker OPEN for provider: {}", name);
                return None;
            }
        }

        let providers = self.providers.read().await;
        providers.iter().find(|p| p.name() == name).cloned()
    }

    pub async fn report_result(&self, name: &str, is_success: bool) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(name) {
            if is_success {
                breaker.record_success();
            } else {
                breaker.record_failure();
            }
        }
    }

    pub async fn list_providers(&self) -> Vec<String> {
        self.providers
            .read()
            .await
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<std::time::Instant>,
    pub config: crate::config::CircuitBreakerConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(config: crate::config::CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            config,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            _ => {}
        }
    }

    pub fn record_failure(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count += 1;
                self.last_failure_time = Some(std::time::Instant::now());
                if self.failure_count >= self.config.failure_threshold {
                    self.state = CircuitBreakerState::Open;
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                self.failure_count = 1;
                self.success_count = 0;
                self.last_failure_time = Some(std::time::Instant::now());
            }
            _ => {}
        }
    }

    pub fn can_attempt(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    let timeout = std::time::Duration::from_secs(self.config.timeout_secs);
                    if last_failure.elapsed() >= timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }
}

/// Real Prometheus metrics for SecureLLM Bridge.
pub struct MetricsCollector {
    pub registry: Registry,
    pub requests_total: CounterVec,
    pub request_duration_seconds: HistogramVec,
    pub rate_limited_total: CounterVec,
    pub provider_errors_total: CounterVec,
    pub token_usage_total: CounterVec,
    pub cost_usd_total: CounterVec,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = CounterVec::new(
            Opts::new("securellm_requests_total", "Total number of requests"),
            &["provider", "status"],
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(requests_total.clone()))
            .expect("metric registration failed");

        let request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "securellm_request_duration_seconds",
                "Request duration in seconds",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]),
            &["provider"],
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(request_duration_seconds.clone()))
            .expect("metric registration failed");

        let rate_limited_total = CounterVec::new(
            Opts::new(
                "securellm_rate_limited_total",
                "Total rate limited requests",
            ),
            &["provider"],
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(rate_limited_total.clone()))
            .expect("metric registration failed");

        let provider_errors_total = CounterVec::new(
            Opts::new("securellm_provider_errors_total", "Total provider errors"),
            &["provider"],
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(provider_errors_total.clone()))
            .expect("metric registration failed");

        let token_usage_total = CounterVec::new(
            Opts::new("securellm_token_usage_total", "Total tokens used"),
            &["provider", "type"],
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(token_usage_total.clone()))
            .expect("metric registration failed");

        let cost_usd_total = CounterVec::new(
            Opts::new("securellm_cost_usd_total", "Total cost in USD"),
            &["provider"],
        )
        .expect("metric creation failed");
        registry
            .register(Box::new(cost_usd_total.clone()))
            .expect("metric registration failed");

        Self {
            registry,
            requests_total,
            request_duration_seconds,
            rate_limited_total,
            provider_errors_total,
            token_usage_total,
            cost_usd_total,
        }
    }
}
