use crate::config::RepoSlug;
use crate::errors::{GatewayError, Result};
use crate::tracing_redact::redact_pat_in_string;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Stdio;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;

pub fn patch_sha256_hex(patch: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(patch);
    format!("{:x}", hasher.finalize())
}

/// Build an HTTPS clone URL with the PAT embedded as `x-access-token:`.
/// The returned String contains the secret in plaintext — do NOT log it.
pub fn build_clone_url(repo: &RepoSlug, pat: &SecretString) -> String {
    format!(
        "https://x-access-token:{}@github.com/{}.git",
        pat.expose_secret(),
        repo.as_path()
    )
}

const COMMIT_DEFAULTS: &[&str] = &[
    "-c",
    "commit.gpgsign=false",
    "-c",
    "user.email=gateway@securellm.local",
    "-c",
    "user.name=SecureLLM Gateway",
];

const MAX_COMMITS_IN_PATCH: usize = 20;

/// Clone `clone_url` (which may carry credentials), apply a `git format-patch`
/// mbox, then push the resulting branch. Returns the HEAD commit SHA.
///
/// The `clone_url` is treated as a secret for the purpose of error reporting:
/// any stderr or error string containing `x-access-token:<pat>@` is rewritten
/// to `x-access-token:REDACTED@` before being surfaced.
pub async fn apply_and_push(
    clone_url: &str,
    base_branch: &str,
    branch: &str,
    patch_bytes: &[u8],
) -> Result<String> {
    reject_oversized_patch(patch_bytes)?;

    let tmp = TempDir::new().map_err(|e| GatewayError::GitOp(format!("tempdir create: {e}")))?;
    let work = tmp.path();

    // Shallow clone of the target branch into work/.
    run_git(
        work.parent().unwrap_or_else(|| Path::new("/tmp")),
        &[
            "clone",
            "--depth=50",
            "--branch",
            base_branch,
            clone_url,
            work.to_str()
                .ok_or_else(|| GatewayError::GitOp("tempdir path not utf-8".into()))?,
        ],
    )
    .await?;

    // Create the target branch from the freshly cloned HEAD.
    run_git(work, &["checkout", "-b", branch]).await?;

    // Write the mbox to disk verbatim.
    let mbox_path = work.join("incoming.mbox");
    fs::write(&mbox_path, patch_bytes)
        .await
        .map_err(|e| GatewayError::GitOp(format!("write mbox: {e}")))?;

    // Apply with `git am`. --keep-cr survives CRLF in headers; --3way lets
    // patches reach context they couldn't find directly (shallow clone caveat
    // handled by fetch_unshallow fallback below).
    let mut am_args = Vec::with_capacity(COMMIT_DEFAULTS.len() + 6);
    am_args.extend_from_slice(COMMIT_DEFAULTS);
    am_args.extend_from_slice(&[
        "am",
        "--keep-cr",
        "--3way",
        "--committer-date-is-author-date",
        mbox_path
            .to_str()
            .ok_or_else(|| GatewayError::GitOp("mbox path not utf-8".into()))?,
    ]);

    if let Err(am_err) = run_git(work, &am_args).await {
        // 3-way merge may fail if shallow clone lacks the blob OIDs.
        // Retry once after unshallowing, then propagate.
        let _ = run_git(work, &["am", "--abort"]).await;
        run_git(work, &["fetch", "--unshallow"]).await.ok();
        run_git(work, &am_args).await.map_err(|_| am_err)?;
    }

    let sha = run_git_capture(work, &["rev-parse", "HEAD"]).await?;
    let sha = sha.trim().to_string();

    run_git(work, &["push", "origin", branch]).await?;

    Ok(sha)
}

fn reject_oversized_patch(patch_bytes: &[u8]) -> Result<()> {
    // Cheap heuristic: count `From ` lines at the start of mbox entries.
    // `git format-patch` emits one per commit.
    let count = patch_bytes
        .windows(5)
        .filter(|w| {
            *w == b"\nFrom" && {
                // followed by space + 40 hex chars + space typically; but the
                // first commit has no leading `\n`, so add 1 implicitly.
                true
            }
        })
        .count()
        + if patch_bytes.starts_with(b"From ") {
            1
        } else {
            0
        };
    if count > MAX_COMMITS_IN_PATCH {
        return Err(GatewayError::Validation(format!(
            "patch contains {count} commits, exceeds cap of {MAX_COMMITS_IN_PATCH}"
        )));
    }
    Ok(())
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| GatewayError::GitOp(format!("spawn git: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let redacted_stderr = redact_pat_in_string(&stderr);
        let cmd_hint = redact_pat_in_string(&args.join(" "));
        return Err(GatewayError::GitOp(format!(
            "git {cmd_hint} exited {:?}: {redacted_stderr}",
            out.status.code()
        )));
    }
    Ok(())
}

async fn run_git_capture(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| GatewayError::GitOp(format!("spawn git: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(GatewayError::GitOp(format!(
            "git {} exited {:?}: {}",
            args.join(" "),
            out.status.code(),
            redact_pat_in_string(&stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_deterministic() {
        let a = patch_sha256_hex(b"hello world");
        let b = patch_sha256_hex(b"hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn sha256_differs_for_different_input() {
        let a = patch_sha256_hex(b"hello world");
        let b = patch_sha256_hex(b"hello world!");
        assert_ne!(a, b);
    }

    #[test]
    fn rejects_patch_with_too_many_commits() {
        let mut payload = Vec::new();
        for _ in 0..21 {
            payload.extend_from_slice(
                b"From 0123456789abcdef0123456789abcdef01234567 Mon Sep 17 00:00:00 2001\n",
            );
        }
        assert!(reject_oversized_patch(&payload).is_err());
    }

    #[test]
    fn accepts_patch_within_commit_cap() {
        let mut payload = Vec::new();
        for _ in 0..5 {
            payload.extend_from_slice(
                b"From 0123456789abcdef0123456789abcdef01234567 Mon Sep 17 00:00:00 2001\n",
            );
        }
        assert!(reject_oversized_patch(&payload).is_ok());
    }
}
