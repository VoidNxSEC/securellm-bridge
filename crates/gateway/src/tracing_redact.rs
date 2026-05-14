/// Tracing redaction helpers. The full layer that rewrites
/// `x-access-token:<pat>@` → `x-access-token:REDACTED@` lands in Onda 3
/// alongside `git_ops::apply_and_push`. This module owns the regex/state.
pub fn redact_pat_in_string(s: &str) -> String {
    // Lightweight scan for "x-access-token:<token>@". We do not pull in a
    // regex crate just for this — manual split keeps the dep surface small.
    const PREFIX: &str = "x-access-token:";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find(PREFIX) {
        out.push_str(&rest[..idx]);
        out.push_str(PREFIX);
        let after = &rest[idx + PREFIX.len()..];
        if let Some(at_idx) = after.find('@') {
            out.push_str("REDACTED");
            rest = &after[at_idx..];
        } else {
            // No closing `@` — preserve the rest verbatim, but the token
            // would still be exposed; treat as an anomaly the caller must avoid.
            out.push_str(after);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_pat_in_url() {
        let raw = "https://x-access-token:ghp_supersecret123@github.com/owner/repo.git";
        let redacted = redact_pat_in_string(raw);
        assert!(!redacted.contains("ghp_supersecret123"));
        assert!(redacted.contains("REDACTED"));
    }

    #[test]
    fn passes_through_unrelated_strings() {
        let raw = "no token here";
        assert_eq!(redact_pat_in_string(raw), raw);
    }

    #[test]
    fn redacts_multiple_occurrences() {
        let raw = "x-access-token:a@host1 and x-access-token:b@host2";
        let r = redact_pat_in_string(raw);
        assert!(!r.contains(":a@"));
        assert!(!r.contains(":b@"));
    }
}
