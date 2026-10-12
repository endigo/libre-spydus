//! The one error type every fallible core API returns.
//!
//! "Couldn't place/cancel/edit" is **data** (`ReserveResult { ok: false, .. }`), never an error —
//! it mirrors the middleware's HTTP 409.
//!
//! `flat_error`: foreign code sees each variant with one string (the `Display` text below).
//! Exposing the `message` fields directly generates Kotlin that does not compile (the field
//! collides with `Throwable.message`). The Rust API is unchanged.

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum SpydusError {
    /// Transport failure (DNS, TLS, connection, body read).
    #[error("network error: {message}")]
    Network { message: String },
    /// URL does not start with `ORIGIN + "/"`.
    #[error("forbidden origin")]
    OriginForbidden,
    /// Login issued no session cookie.
    #[error("credentials rejected")]
    CredentialsRejected,
    /// No account meta-refresh / landed on `frmLogin`.
    #[error("not logged in")]
    NotLoggedIn,
    /// Non-2xx upstream status, "no bib record link", …
    #[error("upstream error: {message}")]
    Upstream { message: String },
    #[error("parse error: {message}")]
    Parse { message: String },
    /// AWS WAF answered instead of the library (`x-amzn-waf-action`, e.g. the UK's 405 captcha).
    /// The native app shows the page for the user to complete, then passes the resulting cookie
    /// back; the core never automates or bypasses it.
    #[error("human verification required")]
    HumanVerificationRequired,
}
