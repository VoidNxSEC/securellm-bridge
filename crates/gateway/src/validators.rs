use crate::config::RepoSlug;
use crate::errors::{GatewayError, Result};

const MIN_RATIONALE_CHARS: usize = 20;
const MIN_PR_BODY_CHARS: usize = 50;
const MIN_COMMENT_BODY_CHARS: usize = 10;
const MAX_PR_TITLE_CHARS: usize = 256;

pub fn rationale(s: &str) -> Result<()> {
    let n = s.trim().chars().count();
    if n < MIN_RATIONALE_CHARS {
        return Err(GatewayError::Validation(format!(
            "rationale too short: {n} chars, need >= {MIN_RATIONALE_CHARS}"
        )));
    }
    Ok(())
}

pub fn pr_body(s: &str) -> Result<()> {
    let n = s.trim().chars().count();
    if n < MIN_PR_BODY_CHARS {
        return Err(GatewayError::Validation(format!(
            "pr body too short: {n} chars, need >= {MIN_PR_BODY_CHARS}"
        )));
    }
    Ok(())
}

pub fn comment_body(s: &str) -> Result<()> {
    let n = s.trim().chars().count();
    if n < MIN_COMMENT_BODY_CHARS {
        return Err(GatewayError::Validation(format!(
            "comment body too short: {n} chars, need >= {MIN_COMMENT_BODY_CHARS}"
        )));
    }
    Ok(())
}

pub fn pr_title(s: &str) -> Result<()> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(GatewayError::Validation("pr title is empty".into()));
    }
    let n = trimmed.chars().count();
    if n > MAX_PR_TITLE_CHARS {
        return Err(GatewayError::Validation(format!(
            "pr title too long: {n} chars, max {MAX_PR_TITLE_CHARS}"
        )));
    }
    Ok(())
}

pub fn repo_in_allowlist(repo: &RepoSlug, allowlist: &[RepoSlug]) -> Result<()> {
    if allowlist.iter().any(|r| r == repo) {
        Ok(())
    } else {
        Err(GatewayError::RepoNotAllowed(repo.as_path()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rationale_too_short_rejects() {
        assert!(rationale("too short").is_err());
        assert!(rationale("").is_err());
        assert!(rationale("   ").is_err());
    }

    #[test]
    fn rationale_at_boundary_accepts() {
        assert!(rationale("a".repeat(20).as_str()).is_ok());
        assert!(rationale("a".repeat(100).as_str()).is_ok());
    }

    #[test]
    fn pr_body_under_50_rejects() {
        assert!(pr_body(&"x".repeat(49)).is_err());
    }

    #[test]
    fn pr_body_at_50_accepts() {
        assert!(pr_body(&"x".repeat(50)).is_ok());
    }

    #[test]
    fn comment_body_under_10_rejects() {
        assert!(comment_body("hey").is_err());
    }

    #[test]
    fn comment_body_at_10_accepts() {
        assert!(comment_body(&"x".repeat(10)).is_ok());
    }

    #[test]
    fn pr_title_empty_rejects() {
        assert!(pr_title("").is_err());
        assert!(pr_title("    ").is_err());
    }

    #[test]
    fn pr_title_at_256_accepts() {
        assert!(pr_title(&"x".repeat(256)).is_ok());
        assert!(pr_title(&"x".repeat(257)).is_err());
    }

    #[test]
    fn repo_not_in_allowlist_rejects() {
        let allowlist = vec![RepoSlug::parse("a/b").unwrap()];
        let repo = RepoSlug::parse("c/d").unwrap();
        assert!(repo_in_allowlist(&repo, &allowlist).is_err());
    }

    #[test]
    fn repo_in_allowlist_accepts() {
        let allowlist = vec![RepoSlug::parse("a/b").unwrap()];
        let repo = RepoSlug::parse("a/b").unwrap();
        assert!(repo_in_allowlist(&repo, &allowlist).is_ok());
    }
}
