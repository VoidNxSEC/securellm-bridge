use crate::audit::AuditEvent;
use crate::errors::{GatewayError, Result};
use crate::{validators, GatewayContext, RepoSlug};
use secrecy::ExposeSecret;
use serde_json::json;

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CommentPrArgs {
    pub repo: String,
    pub pr_number: u64,
    pub body: String,
    pub rationale: String,
}

pub async fn handle(ctx: &GatewayContext, args: CommentPrArgs) -> Result<serde_json::Value> {
    let event = AuditEvent::new(&ctx.config.agent_id, "gateway_comment_pr")
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
    if let Err(e) = validators::comment_body(&args.body) {
        ctx.audit.emit(&event.rejected(e.to_string())).await?;
        return Err(e);
    }

    let client = octocrab::Octocrab::builder()
        .personal_token(ctx.config.pat.expose_secret().to_string())
        .build()
        .map_err(|e| GatewayError::GithubApi(format!("client build: {e}")))?;

    let result = client
        .issues(&repo.owner, &repo.name)
        .create_comment(args.pr_number, &args.body)
        .await;

    match result {
        Ok(comment) => {
            let response = json!({
                "comment_id": comment.id.0,
                "url": comment.html_url.to_string(),
            });
            ctx.audit.emit(&event.ok(response.clone())).await?;
            Ok(response)
        }
        Err(e) => {
            let msg = format!("github issues.create_comment failed: {}", e);
            let err = GatewayError::GithubApi(msg.clone());
            ctx.audit.emit(&event.failed(msg)).await?;
            Err(err)
        }
    }
}
