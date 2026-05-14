//! Cgroup Helper — Minimal setuid helper for sandbox cgroup management
//!
//! Phase 2 of ADR-0001: NEUTRON-Audited cgroup Helper
//!
//! Usage:
//!   cgroup-helper [--socket-path /run/securellm/cgroup.sock]
//!
//! Security:
//!   - Listens on a Unix Domain Socket with mode 0600
//!   - Only creates cgroups under /sys/fs/cgroup/securellm/
//!   - Each request validated against known agent profiles
//!   - Designed to run with CAP_SYS_ADMIN, not full root
//!   - Stateless — one request, one response, no persistent state

use securellm_security::cgroup_helper;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let socket_path = std::env::var("CGROUP_SOCKET")
        .unwrap_or_else(|_| "/run/securellm/cgroup.sock".to_string());

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    info!(
        "Starting cgroup-helper on {} (PID: {})",
        socket_path,
        std::process::id()
    );

    if let Err(e) = cgroup_helper::serve_cgroup_socket(&socket_path).await {
        tracing::error!("Cgroup helper fatal error: {}", e);
        std::process::exit(1);
    }
}
