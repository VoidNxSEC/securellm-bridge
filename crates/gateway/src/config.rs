use crate::errors::{GatewayError, Result};
use secrecy::SecretString;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RepoSlug {
    pub owner: String,
    pub name: String,
}

impl RepoSlug {
    pub fn parse(s: &str) -> Result<Self> {
        let (owner, name) = s.split_once('/').ok_or_else(|| {
            GatewayError::Config(format!("invalid repo slug '{s}', expected owner/name"))
        })?;
        if owner.is_empty() || name.is_empty() {
            return Err(GatewayError::Config(format!(
                "empty owner or name in '{s}'"
            )));
        }
        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    pub fn as_path(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

pub struct GatewayConfig {
    pub pat: SecretString,
    pub allowlist: Vec<RepoSlug>,
    pub agent_id: String,
    pub log_dir: PathBuf,
    pub transport: GatewayTransport,
    pub listen_addr: SocketAddr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayTransport {
    Stdio,
    Http,
}

impl std::fmt::Display for GatewayTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayTransport::Stdio => f.write_str("stdio"),
            GatewayTransport::Http => f.write_str("http"),
        }
    }
}

impl FromStr for GatewayTransport {
    type Err = GatewayError;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            other => Err(GatewayError::Config(format!(
                "invalid GATEWAY_TRANSPORT '{other}', expected stdio or http"
            ))),
        }
    }
}

impl std::fmt::Debug for GatewayConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayConfig")
            .field("pat", &"<redacted>")
            .field("allowlist", &self.allowlist)
            .field("agent_id", &self.agent_id)
            .field("log_dir", &self.log_dir)
            .field("transport", &self.transport)
            .field("listen_addr", &self.listen_addr)
            .finish()
    }
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self> {
        let pat_raw = std::env::var("GATEWAY_GITHUB_PAT")
            .map_err(|_| GatewayError::Config("missing GATEWAY_GITHUB_PAT env var".into()))?;
        if pat_raw.trim().is_empty() {
            return Err(GatewayError::Config("GATEWAY_GITHUB_PAT is empty".into()));
        }
        let pat = SecretString::new(pat_raw);

        let allowlist_raw = std::env::var("GATEWAY_REPO_ALLOWLIST")
            .map_err(|_| GatewayError::Config("missing GATEWAY_REPO_ALLOWLIST env var".into()))?;
        let allowlist: Vec<RepoSlug> = allowlist_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(RepoSlug::parse)
            .collect::<Result<_>>()?;
        if allowlist.is_empty() {
            return Err(GatewayError::Config(
                "GATEWAY_REPO_ALLOWLIST is empty (fail-closed)".into(),
            ));
        }

        let agent_id = std::env::var("GATEWAY_AGENT_ID")
            .map_err(|_| GatewayError::Config("missing GATEWAY_AGENT_ID env var".into()))?;
        if agent_id.trim().is_empty() {
            return Err(GatewayError::Config("GATEWAY_AGENT_ID is empty".into()));
        }

        let log_dir = match std::env::var("GATEWAY_LOG_DIR") {
            Ok(s) if !s.trim().is_empty() => PathBuf::from(s),
            _ => {
                let base = std::env::var("XDG_DATA_HOME")
                    .ok()
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var("HOME")
                            .ok()
                            .map(|h| PathBuf::from(h).join(".local/share"))
                    })
                    .ok_or_else(|| GatewayError::Config("cannot resolve default log dir".into()))?;
                base.join("voidnx-gateway")
            }
        };

        let transport = match std::env::var("GATEWAY_TRANSPORT") {
            Ok(s) => s.parse()?,
            Err(_) => GatewayTransport::Stdio,
        };

        let listen_addr = match std::env::var("GATEWAY_LISTEN_ADDR") {
            Ok(s) => parse_listen_addr(&s)?,
            Err(_) => default_listen_addr(),
        };

        Ok(Self {
            pat,
            allowlist,
            agent_id,
            log_dir,
            transport,
            listen_addr,
        })
    }

    pub fn repo_in_allowlist(&self, repo: &RepoSlug) -> bool {
        self.allowlist.iter().any(|r| r == repo)
    }
}

fn default_listen_addr() -> SocketAddr {
    "127.0.0.1:8765"
        .parse()
        .expect("hard-coded gateway listen addr is valid")
}

fn parse_listen_addr(raw: &str) -> Result<SocketAddr> {
    raw.trim().parse().map_err(|e| {
        GatewayError::Config(format!(
            "invalid GATEWAY_LISTEN_ADDR '{raw}', expected host:port: {e}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_transport_parses_supported_modes() {
        assert_eq!(
            "stdio".parse::<GatewayTransport>().unwrap(),
            GatewayTransport::Stdio
        );
        assert_eq!(
            "HTTP".parse::<GatewayTransport>().unwrap(),
            GatewayTransport::Http
        );
    }

    #[test]
    fn gateway_transport_rejects_unknown_modes() {
        let err = "websocket"
            .parse::<GatewayTransport>()
            .expect_err("must reject unknown mode");
        assert!(err.to_string().contains("invalid GATEWAY_TRANSPORT"));
    }

    #[test]
    fn default_listen_addr_is_loopback() {
        assert_eq!(default_listen_addr(), "127.0.0.1:8765".parse().unwrap());
    }

    #[test]
    fn listen_addr_parser_rejects_missing_port() {
        let err = parse_listen_addr("127.0.0.1").expect_err("must require port");
        assert!(err.to_string().contains("invalid GATEWAY_LISTEN_ADDR"));
    }
}
