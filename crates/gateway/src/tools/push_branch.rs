use crate::audit::AuditEvent;
use crate::errors::{GatewayError, Result};
use crate::{git_ops, validators, GatewayContext, RepoSlug};
use base64::Engine;
use serde_json::json;

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct PushBranchArgs {
    /// Repository in `owner/name` form. Must be in the configured allowlist.
    pub repo: String,
    /// Target branch name to push (e.g. `feat/x`).
    pub branch: String,
    /// Base64-encoded `git format-patch` mbox to apply onto `base` before push.
    pub patch: String,
    /// Why the agent is pushing this branch. Must be >= 20 chars.
    pub rationale: String,
    /// Base branch to clone (default: `main`).
    #[serde(default)]
    pub base: Option<String>,
}

pub async fn handle(ctx: &GatewayContext, args: PushBranchArgs) -> Result<serde_json::Value> {
    let event = AuditEvent::new(&ctx.config.agent_id, "gateway_push_branch")
        .with_repo(&args.repo)
        .with_rationale(&args.rationale);

    let repo = match RepoSlug::parse(&args.repo) {
        Ok(r) => r,
        Err(e) => {
            ctx.audit.emit(&event.rejected(e.to_string())).await?;
            return Err(e);
        }
    };

    if let Err(e) = validators::repo_in_allowlist(&repo, &ctx.config.allowlist) {
        ctx.audit.emit(&event.rejected(e.to_string())).await?;
        return Err(e);
    }
    if let Err(e) = validators::rationale(&args.rationale) {
        ctx.audit.emit(&event.rejected(e.to_string())).await?;
        return Err(e);
    }

    let patch_bytes = match base64::engine::general_purpose::STANDARD.decode(args.patch.trim()) {
        Ok(bytes) => bytes,
        Err(e) => {
            let err = GatewayError::Validation(format!("invalid base64 patch: {e}"));
            ctx.audit.emit(&event.rejected(err.to_string())).await?;
            return Err(err);
        }
    };
    let patch_sha = git_ops::patch_sha256_hex(&patch_bytes);
    let event = event.with_patch_sha256(&patch_sha);

    let base = args.base.as_deref().unwrap_or("main");
    let clone_url = git_ops::build_clone_url(&repo, &ctx.config.pat);
    match git_ops::apply_and_push(&clone_url, base, &args.branch, &patch_bytes).await {
        Ok(sha) => {
            let response = json!({
                "pushed": true,
                "sha": sha,
                "url": format!("https://github.com/{}/tree/{}", repo.as_path(), args.branch),
                "branch": args.branch,
                "patch_sha256": patch_sha,
            });
            ctx.audit.emit(&event.ok(response.clone())).await?;
            Ok(response)
        }
        Err(e) => {
            ctx.audit.emit(&event.failed(e.to_string())).await?;
            Err(e)
        }
    }
}
