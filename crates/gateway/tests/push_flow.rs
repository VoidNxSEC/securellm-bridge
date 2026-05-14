//! Integration test for git_ops::apply_and_push using a local bare repo
//! as the push target. Requires `git` on PATH.

use securellm_gateway::git_ops;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct GitFixture {
    _root: TempDir,
    bare_repo: PathBuf,
    agent_repo: PathBuf,
}

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Test Author")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test Author")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .status()
        .expect("spawn git");
    assert!(status.success(), "git {:?} failed in {:?}", args, cwd);
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn setup_fixture() -> GitFixture {
    let root = TempDir::new().unwrap();
    let bare_repo = root.path().join("origin.git");
    let agent_repo = root.path().join("agent-work");

    // Bootstrap bare repo with initial commit on `main`.
    let seed = root.path().join("seed");
    std::fs::create_dir(&seed).unwrap();
    git(&seed, &["init", "-b", "main"]);
    std::fs::write(seed.join("README.md"), "initial\n").unwrap();
    git(&seed, &["add", "."]);
    git(&seed, &["commit", "-m", "initial commit"]);

    Command::new("git")
        .args(["init", "--bare", "-b", "main"])
        .arg(bare_repo.to_str().unwrap())
        .status()
        .unwrap();

    git(
        &seed,
        &["remote", "add", "origin", bare_repo.to_str().unwrap()],
    );
    git(&seed, &["push", "origin", "main"]);

    // Agent-side clone where we'll generate a real format-patch.
    Command::new("git")
        .args([
            "clone",
            bare_repo.to_str().unwrap(),
            agent_repo.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    GitFixture {
        _root: root,
        bare_repo,
        agent_repo,
    }
}

fn make_patch(agent_repo: &Path) -> Vec<u8> {
    // One commit on top of main → mbox via git format-patch -1 --stdout.
    std::fs::write(agent_repo.join("hello.txt"), "hello from gateway test\n").unwrap();
    git(agent_repo, &["add", "hello.txt"]);
    git(agent_repo, &["commit", "-m", "test: add hello.txt"]);
    let out = Command::new("git")
        .args(["format-patch", "-1", "--stdout"])
        .current_dir(agent_repo)
        .output()
        .expect("git format-patch");
    assert!(out.status.success());
    out.stdout
}

#[tokio::test(flavor = "multi_thread")]
async fn t3_push_happy_path_to_bare_repo() {
    let fx = setup_fixture();
    let patch = make_patch(&fx.agent_repo);
    assert!(!patch.is_empty(), "patch should not be empty");

    let clone_url = format!("file://{}", fx.bare_repo.display());
    let sha = git_ops::apply_and_push(&clone_url, "main", "feat/from-gateway", &patch)
        .await
        .expect("apply_and_push should succeed");

    assert_eq!(sha.len(), 40, "expected 40-char SHA, got {sha:?}");

    // Verify branch landed in bare repo.
    let branches = git_output(&fx.bare_repo, &["branch", "--list"]);
    assert!(
        branches.contains("feat/from-gateway"),
        "expected feat/from-gateway in bare repo, got:\n{branches}"
    );

    // Verify the SHA returned matches what the bare repo holds.
    let bare_sha = git_output(&fx.bare_repo, &["rev-parse", "feat/from-gateway"]);
    assert_eq!(bare_sha.trim(), sha, "SHAs should match");

    // Verify author was preserved (Test Author from format-patch's mbox).
    let log = git_output(
        &fx.bare_repo,
        &["log", "-1", "--format=%an <%ae>", "feat/from-gateway"],
    );
    assert!(
        log.contains("Test Author <test@example.com>"),
        "author should be preserved by `git am`, got: {log}"
    );
}
