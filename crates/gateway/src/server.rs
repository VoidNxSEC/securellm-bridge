use crate::{
    tools::{
        comment_pr::{self, CommentPrArgs},
        create_pr::{self, CreatePrArgs},
        push_branch::{self, PushBranchArgs},
    },
    GatewayContext,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

#[derive(Clone)]
pub struct GatewayHandler {
    ctx: GatewayContext,
    #[allow(dead_code)]
    tool_router: ToolRouter<GatewayHandler>,
}

#[tool_router]
impl GatewayHandler {
    pub fn new(ctx: GatewayContext) -> Self {
        Self {
            ctx,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Apply a base64-encoded `git format-patch` mbox onto a fresh shallow clone of \
                       an allowlisted repo and push the resulting branch. Requires a rationale of \
                       at least 20 characters. Returns the resulting commit SHA and URL."
    )]
    async fn gateway_push_branch(
        &self,
        Parameters(args): Parameters<PushBranchArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match push_branch::handle(&self.ctx, args).await {
            Ok(value) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
            )])),
            Err(e) => Err(McpError::invalid_params(e.to_string(), None)),
        }
    }

    #[tool(
        description = "Open a pull request from `head` into `base` on an allowlisted repo. Requires \
                       a rationale (>=20 chars), non-empty title, and body (>=50 chars). Returns \
                       the PR number and URL."
    )]
    async fn gateway_create_pr(
        &self,
        Parameters(args): Parameters<CreatePrArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match create_pr::handle(&self.ctx, args).await {
            Ok(value) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
            )])),
            Err(e) => Err(McpError::invalid_params(e.to_string(), None)),
        }
    }

    #[tool(
        description = "Post a comment on a pull request in an allowlisted repo. Requires a rationale \
                       (>=20 chars) and a comment body (>=10 chars)."
    )]
    async fn gateway_comment_pr(
        &self,
        Parameters(args): Parameters<CommentPrArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match comment_pr::handle(&self.ctx, args).await {
            Ok(value) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
            )])),
            Err(e) => Err(McpError::invalid_params(e.to_string(), None)),
        }
    }
}

#[tool_handler]
impl ServerHandler for GatewayHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "SecureLLM Gateway MCP — pushes branches, opens PRs, comments on PRs against an \
                 allowlisted set of repos. Credentials live server-side; each call must justify \
                 itself via `rationale`."
                    .to_string(),
            )
    }
}
