use crate::errors::{GatewayError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub ts: DateTime<Utc>,
    pub event_id: Uuid,
    pub agent_id: String,
    pub tool: String,
    pub repo: Option<String>,
    pub rationale: Option<String>,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_response: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AuditEvent {
    pub fn new(agent_id: impl Into<String>, tool: impl Into<String>) -> Self {
        Self {
            ts: Utc::now(),
            event_id: Uuid::now_v7(),
            agent_id: agent_id.into(),
            tool: tool.into(),
            repo: None,
            rationale: None,
            outcome: "pending".into(),
            github_response: None,
            patch_sha256: None,
            error: None,
        }
    }

    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }

    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    pub fn ok(mut self, response: serde_json::Value) -> Self {
        self.outcome = "ok".into();
        self.github_response = Some(response);
        self
    }

    pub fn rejected(mut self, error: impl Into<String>) -> Self {
        self.outcome = "rejected".into();
        self.error = Some(error.into());
        self
    }

    pub fn failed(mut self, error: impl Into<String>) -> Self {
        self.outcome = "failed".into();
        self.error = Some(error.into());
        self
    }

    pub fn with_patch_sha256(mut self, sha: impl Into<String>) -> Self {
        self.patch_sha256 = Some(sha.into());
        self
    }
}

#[derive(Clone)]
pub struct JsonlSink {
    writer: Arc<Mutex<File>>,
    path: PathBuf,
}

impl JsonlSink {
    pub async fn open(log_dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(log_dir)
            .await
            .map_err(|e| GatewayError::Audit(format!("create log dir {log_dir:?}: {e}")))?;
        let path = log_dir.join("events.jsonl");
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
            .map_err(|e| GatewayError::Audit(format!("open {path:?}: {e}")))?;
        Ok(Self {
            writer: Arc::new(Mutex::new(file)),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn emit(&self, event: &AuditEvent) -> Result<()> {
        let mut line = serde_json::to_vec(event)?;
        line.push(b'\n');
        let mut w = self.writer.lock().await;
        w.write_all(&line)
            .await
            .map_err(|e| GatewayError::Audit(format!("write: {e}")))?;
        w.flush()
            .await
            .map_err(|e| GatewayError::Audit(format!("flush: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn writes_valid_jsonl_lines() {
        let tmp = TempDir::new().unwrap();
        let sink = JsonlSink::open(tmp.path()).await.unwrap();
        let e1 = AuditEvent::new("agent-a", "gateway_push_branch")
            .with_repo("acme/widgets")
            .with_rationale("rationale 1 long enough to pass validation")
            .ok(serde_json::json!({"sha": "abc123"}));
        let e2 = AuditEvent::new("agent-a", "gateway_create_pr")
            .with_repo("acme/widgets")
            .rejected("rationale too short");
        sink.emit(&e1).await.unwrap();
        sink.emit(&e2).await.unwrap();

        let raw = tokio::fs::read_to_string(sink.path()).await.unwrap();
        let lines: Vec<&str> = raw.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let _: AuditEvent = serde_json::from_str(line).unwrap();
        }
    }

    #[tokio::test]
    async fn pat_never_appears_in_event_struct() {
        let event = AuditEvent::new("agent-a", "gateway_push_branch")
            .with_repo("acme/widgets")
            .ok(serde_json::json!({"sha": "abc"}));
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.to_lowercase().contains("pat"));
        assert!(!json.contains("token"));
        assert!(!json.contains("secret"));
    }
}
