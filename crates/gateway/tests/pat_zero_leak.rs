//! T5 from MISSION: the PAT must never appear in any error string, stderr,
//! or audit event. We trigger a deliberate clone failure with a PAT in the
//! URL and assert the surfaced error contains no trace of the secret.

use securellm_gateway::git_ops;

const SECRET_TOKEN: &str = "ghp_SUPERSECRET_DO_NOT_LEAK_12345xyz";

#[tokio::test(flavor = "multi_thread")]
async fn t5_pat_never_leaks_through_clone_error() {
    // Build a URL that points at a non-existent github repo with the PAT
    // embedded. `git clone` will fail and write stderr — we check the
    // surfaced GatewayError string is PAT-free.
    let bogus_url = format!(
        "https://x-access-token:{SECRET_TOKEN}@github.invalid-tld-does-not-exist/none/none.git"
    );

    let result = git_ops::apply_and_push(&bogus_url, "main", "feat/x", b"dummy").await;
    let err = result.expect_err("clone of bogus host should fail");
    let msg = err.to_string();

    assert!(
        !msg.contains(SECRET_TOKEN),
        "PAT leaked through GatewayError display: {msg}"
    );
    assert!(
        !msg.to_lowercase().contains("supersecret"),
        "case-insensitive secret leak detected: {msg}"
    );

    let debug = format!("{err:?}");
    assert!(
        !debug.contains(SECRET_TOKEN),
        "PAT leaked through GatewayError debug: {debug}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn t5_pat_redacted_when_clone_url_in_stderr() {
    // Even if git itself echoes the URL into stderr (some versions do this
    // for auth failures), the redaction layer must intercept.
    let bogus_url = format!("https://x-access-token:{SECRET_TOKEN}@127.0.0.1:1/no-such-repo.git");
    let result = git_ops::apply_and_push(&bogus_url, "main", "feat/x", b"dummy").await;
    let err = result.expect_err("connection refused on bogus port should fail");
    let msg = err.to_string();
    assert!(
        !msg.contains(SECRET_TOKEN),
        "PAT leaked through connection error: {msg}"
    );
}
