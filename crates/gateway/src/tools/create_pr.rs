use crate::audit::AuditEvent;
use crate::errors::{GatewayError, Result};
use crate::{validators, GatewayContext, RepoSlug};
use secrecy::ExposeSecret;
use serde_json::json;

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CreatePrArgs {
    pub repo: String,
    pub head: String,
    pub title: String,
    pub body: String,
    pub rationale: String,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub draft: Option<bool>,
}

pub async fn handle(ctx: &GatewayContext, args: CreatePrArgs) -> Result<serde_json::Value> {
    let event = AuditEvent::new(&ctx.config.agent_id, "gateway_create_pr")
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
    if let Err(e) = validators::pr_title(&args.title) {
        ctx.audit.emit(&event.rejected(e.to_string())).await?;
        return Err(e);
    }
    if let Err(e) = validators::pr_body(&args.body) {
        ctx.audit.emit(&event.rejected(e.to_string())).await?;
        return Err(e);
    }

    let base = args.base.as_deref().unwrap_or("main");
    let draft = args.draft.unwrap_or(false);

    let client = octocrab::Octocrab::builder()
        .personal_token(ctx.config.pat.expose_secret().to_string())
        .build()
        .map_err(|e| GatewayError::GithubApi(format!("client build: {e}")))?;

    let pr_result = client
        .pulls(&repo.owner, &repo.name)
        .create(&args.title, &args.head, base)
        .body(&args.body)
        .draft(draft)
        .send()
        .await;

    match pr_result {
        Ok(pr) => {
            let response = json!({
                "pr_number": pr.number,
                "url": pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
                "state": format!("{:?}", pr.state),
            });
            ctx.audit.emit(&event.ok(response.clone())).await?;
            Ok(response)
        }
        Err(e) => {
            // Manually build error string — never embed octocrab's Debug which
            // might contain the client (and thus PAT in some versions).
            let msg = format!("github pulls.create failed: {}", e);
            let err = GatewayError::GithubApi(msg.clone());
            ctx.audit.emit(&event.failed(msg)).await?;
            Err(err)
        }
    }
}
