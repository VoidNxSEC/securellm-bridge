use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("repo not in allowlist: {0}")]
    RepoNotAllowed(String),

    #[error("github api error: {0}")]
    GithubApi(String),

    #[error("git operation failed: {0}")]
    GitOp(String),

    #[error("audit sink error: {0}")]
    Audit(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GatewayError>;
