//! AWS WAF human verification, the pure parts (a port of iOS `VerificationCookie.swift`;
//! core/README.md "Human verification"): find the library's `aws-waf-token` among the helper web
//! view's cookies. Nothing here touches the check itself.

/// The cookie the library's WAF sets once the reader has confirmed they're human.
pub const NAME: &str = "aws-waf-token";

/// What the matcher needs from a web view cookie (tests build these directly).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub value: String,
    pub domain: String,
    /// Seconds since 1970; `None` for a session cookie.
    pub expires: Option<i64>,
}

impl Candidate {
    pub fn new(name: &str, value: &str, domain: &str, expires: Option<i64>) -> Self {
        Self { name: name.to_owned(), value: value.to_owned(), domain: domain.to_owned(), expires }
    }
}

/// `"aws-waf-token=<value>"` for the library at `host`, or `None` when there's no usable token.
///
/// The cookie must be named `aws-waf-token`, have a value, not have expired, and its domain must
/// match `host` (RFC 6265 domain-match: the host itself or a parent domain, with or without a
/// leading dot). A host-only cookie wins over a parent-domain one; among equals the one that
/// expires last wins.
pub fn header(cookies: &[Candidate], host: &str, now: i64) -> Option<String> {
    let rank =
        |cookie: &Candidate| (normalized(&cookie.domain) == normalized(host), cookie.expires.unwrap_or(i64::MAX));
    cookies
        .iter()
        .filter(|cookie| {
            cookie.name == NAME
                && !cookie.value.is_empty()
                && cookie.expires.is_none_or(|expires| expires > now)
                && domain_matches(&cookie.domain, host)
        })
        .max_by_key(|cookie| rank(cookie))
        .map(|cookie| format!("{NAME}={}", cookie.value))
}

/// Whether a cookie set for `domain` is sent to `host`: `canning.spydus.com` and `.spydus.com`
/// match `canning.spydus.com`; `other.spydus.com`, `spydus.co.uk` and `nning.spydus.com` don't.
/// IP addresses only match exactly.
pub fn domain_matches(domain: &str, host: &str) -> bool {
    let domain = normalized(domain);
    let host = normalized(host);
    if domain.is_empty() || host.is_empty() {
        return false;
    }
    if domain == host {
        return true;
    }
    if is_ip_address(&host) || !domain.contains('.') {
        return false;
    }
    host.ends_with(&format!(".{domain}"))
}

/// The host of an origin (`https://camden.spydus.co.uk/` → `camden.spydus.co.uk`).
pub fn host_of(origin: &str) -> String {
    let rest = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let host = match host.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    host.to_ascii_lowercase()
}

/// Lower-cased, without a leading or trailing dot.
fn normalized(domain: &str) -> String {
    domain.trim().trim_matches('.').to_ascii_lowercase()
}

fn is_ip_address(host: &str) -> bool {
    host.contains(':') || host.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "camden.spydus.co.uk";
    const NOW: i64 = 1_800_000_000;

    #[test]
    fn finds_the_token_for_the_library_host() {
        let cookies =
            [Candidate::new("JSESSIONID", "x", HOST, None), Candidate::new(NAME, "abc", HOST, Some(NOW + 60))];
        assert_eq!(header(&cookies, HOST, NOW).as_deref(), Some("aws-waf-token=abc"));
    }

    #[test]
    fn skips_expired_empty_and_foreign_tokens() {
        let cookies = [
            Candidate::new(NAME, "old", HOST, Some(NOW - 1)),
            Candidate::new(NAME, "", HOST, None),
            Candidate::new(NAME, "other", "other.spydus.co.uk", None),
            Candidate::new(NAME, "suffix", "nden.spydus.co.uk", None),
        ];
        assert_eq!(header(&cookies, HOST, NOW), None);
    }

    #[test]
    fn prefers_a_host_cookie_then_the_latest_expiry() {
        let cookies = [
            Candidate::new(NAME, "parent", ".spydus.co.uk", None),
            Candidate::new(NAME, "early", HOST, Some(NOW + 10)),
            Candidate::new(NAME, "late", HOST, Some(NOW + 100)),
        ];
        assert_eq!(header(&cookies, HOST, NOW).as_deref(), Some("aws-waf-token=late"));
        assert_eq!(header(&cookies[..1], HOST, NOW).as_deref(), Some("aws-waf-token=parent"));
    }

    #[test]
    fn domain_match_follows_rfc_6265() {
        assert!(domain_matches("canning.spydus.com", "canning.spydus.com"));
        assert!(domain_matches(".spydus.com", "Canning.Spydus.com"));
        assert!(!domain_matches("other.spydus.com", "canning.spydus.com"));
        assert!(!domain_matches("spydus.co.uk", "canning.spydus.com"));
        assert!(!domain_matches("com", "canning.spydus.com"));
        assert!(domain_matches("10.0.0.1", "10.0.0.1"));
        assert!(!domain_matches("0.0.1", "10.0.0.1"));
    }

    #[test]
    fn host_of_an_origin() {
        assert_eq!(host_of("https://Camden.spydus.co.uk/"), "camden.spydus.co.uk");
        assert_eq!(host_of("https://canning.spydus.com:443/cgi-bin"), "canning.spydus.com");
        assert_eq!(host_of("canning.spydus.com"), "canning.spydus.com");
    }
}
