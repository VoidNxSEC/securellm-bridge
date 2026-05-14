pub mod audit;
pub mod config;
pub mod errors;
pub mod git_ops;
pub mod server;
pub mod tools;
pub mod tracing_redact;
pub mod transport;
pub mod validators;

pub use config::{GatewayConfig, GatewayTransport, RepoSlug};
pub use errors::GatewayError;
pub use server::GatewayHandler;

use std::sync::Arc;

#[derive(Clone)]
pub struct GatewayContext {
    pub config: Arc<GatewayConfig>,
    pub audit: audit::JsonlSink,
}

impl GatewayContext {
    pub fn new(config: Arc<GatewayConfig>, audit: audit::JsonlSink) -> Self {
        Self { config, audit }
    }
}
