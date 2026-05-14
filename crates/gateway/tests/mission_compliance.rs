//! Mission-level integration tests:
//! T1 — short rationale rejected, no git/octocrab invocation
//! T2 — non-allowlisted repo rejected
//! T4 — audit JSONL accumulates valid lines, one per call
//! T6 — boot without PAT exits non-zero quickly

use secrecy::SecretString;
use securellm_gateway::{
    audit::{AuditEvent, JsonlSink},
    config::{GatewayConfig, GatewayTransport, RepoSlug},
    tools::push_branch::{self, PushBranchArgs},
    GatewayContext,
};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

fn test_config(allowlist: Vec<&str>, log_dir: std::path::PathBuf) -> GatewayConfig {
    GatewayConfig {
        pat: SecretString::new("ghp_test_token_not_real_value".into()),
        allowlist: allowlist
            .into_iter()
            .map(|s| RepoSlug::parse(s).unwrap())
            .collect(),
        agent_id: "test-agent".into(),
        log_dir,
        transport: GatewayTransport::Stdio,
        listen_addr: "127.0.0.1:8765".parse().unwrap(),
    }
}

async fn build_ctx(allowlist: Vec<&str>) -> (TempDir, GatewayContext) {
    let tmp = TempDir::new().unwrap();
    let config = Arc::new(test_config(allowlist, tmp.path().to_path_buf()));
    let audit = JsonlSink::open(tmp.path()).await.unwrap();
    let ctx = GatewayContext::new(config, audit);
    (tmp, ctx)
}

fn dummy_patch_b64() -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b"placeholder mbox body")
}

async fn read_audit(dir: &std::path::Path) -> Vec<AuditEvent> {
    let raw = tokio::fs::read_to_string(dir.join("events.jsonl"))
        .await
        .unwrap();
    raw.lines()
        .map(|l| serde_json::from_str(l).expect("each line must be valid AuditEvent JSON"))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn t1_rationale_too_short_is_rejected() {
    let (tmp, ctx) = build_ctx(vec!["acme/widgets"]).await;
    let args = PushBranchArgs {
        repo: "acme/widgets".into(),
        branch: "feat/x".into(),
        patch: dummy_patch_b64(),
        rationale: "too short".into(),
        base: None,
    };
    let err = push_branch::handle(&ctx, args)
        .await
        .expect_err("should reject");
    assert!(err.to_string().contains("rationale too short"));

    let events = read_audit(tmp.path()).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].outcome, "rejected");
    assert_eq!(events[0].tool, "gateway_push_branch");
    assert!(events[0]
        .error
        .as_deref()
        .unwrap()
        .contains("rationale too short"));
}

#[tokio::test(flavor = "multi_thread")]
async fn t2_repo_not_in_allowlist_is_rejected() {
    let (tmp, ctx) = build_ctx(vec!["acme/widgets"]).await;
    let args = PushBranchArgs {
        repo: "evil/leak".into(),
        branch: "feat/x".into(),
        patch: dummy_patch_b64(),
        rationale: "long enough rationale to pass the validator check".into(),
        base: None,
    };
    let err = push_branch::handle(&ctx, args)
        .await
        .expect_err("should reject");
    assert!(err.to_string().contains("evil/leak"));

    let events = read_audit(tmp.path()).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].outcome, "rejected");
    assert_eq!(events[0].repo.as_deref(), Some("evil/leak"));
}

#[tokio::test(flavor = "multi_thread")]
async fn t4_audit_jsonl_accumulates_valid_lines() {
    let (tmp, ctx) = build_ctx(vec!["acme/widgets"]).await;

    // Three rejects in sequence.
    let bad_rationale = PushBranchArgs {
        repo: "acme/widgets".into(),
        branch: "feat/a".into(),
        patch: dummy_patch_b64(),
        rationale: "nope".into(),
        base: None,
    };
    let bad_repo = PushBranchArgs {
        repo: "other/repo".into(),
        branch: "feat/b".into(),
        patch: dummy_patch_b64(),
        rationale: "this is a long enough rationale to pass validators".into(),
        base: None,
    };
    let bad_patch = PushBranchArgs {
        repo: "acme/widgets".into(),
        branch: "feat/c".into(),
        patch: "!!!not-valid-base64!!!".into(),
        rationale: "this is a long enough rationale to pass validators".into(),
        base: None,
    };

    let _ = push_branch::handle(&ctx, bad_rationale).await;
    let _ = push_branch::handle(&ctx, bad_repo).await;
    let _ = push_branch::handle(&ctx, bad_patch).await;

    let events = read_audit(tmp.path()).await;
    assert_eq!(
        events.len(),
        3,
        "expected 3 JSONL lines, got {}",
        events.len()
    );
    for ev in &events {
        assert_eq!(ev.tool, "gateway_push_branch");
        assert!(matches!(ev.outcome.as_str(), "rejected" | "failed"));
        assert!(ev.error.is_some());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn t6_boot_without_pat_fails_fast() {
    // The cargo test runner sets CARGO_BIN_EXE_<name> for each crate binary.
    let bin = env!("CARGO_BIN_EXE_gateway-mcp");

    let mut child = tokio::process::Command::new(bin)
        .env_remove("GATEWAY_GITHUB_PAT")
        .env("GATEWAY_REPO_ALLOWLIST", "acme/widgets")
        .env("GATEWAY_AGENT_ID", "test")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway-mcp");

    // Close stdin so the binary can't block waiting on input.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.shutdown().await;
    }

    let status = timeout(Duration::from_secs(2), child.wait())
        .await
        .expect("binary should exit within 2s when PAT missing")
        .expect("wait failed");

    assert!(
        !status.success(),
        "expected non-zero exit when PAT missing, got {status:?}"
    );

    let output = child.wait_with_output().await.unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stderr}");
    assert!(
        combined.to_lowercase().contains("missing") && combined.to_lowercase().contains("pat"),
        "stderr should mention missing PAT, got: {combined}"
    );
}
