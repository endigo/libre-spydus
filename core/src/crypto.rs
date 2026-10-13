//! OPAC password encryption (`middleware/src/fetch/auth.ts:32-49`). Owner: task E.
//!
//! The web client encrypts the PIN with JSEncrypt (RSA PKCS#1 v1.5, base64 output) using the
//! public key embedded in `encrypt.js`; the middleware reproduces that with node `publicEncrypt`.

use std::sync::LazyLock;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use regex::Regex;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

use crate::error::SpydusError;
use crate::parse::decode::is_js_whitespace;

const PEM_BEGIN: &str = "-----BEGIN PUBLIC KEY-----";
const PEM_END: &str = "-----END PUBLIC KEY-----";

// auth.ts:36 — `/-----BEGIN PUBLIC KEY-----([\s\S]*?)-----END PUBLIC KEY-----/`.
static PEM_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN PUBLIC KEY-----(.*?)-----END PUBLIC KEY-----").unwrap()
});

/// encrypt.js source → PEM (strip `[\\\s]` inside the key, rewrap at 64 columns). auth.ts:32-41.
pub fn public_key_pem_from_encrypt_js(js: &str) -> Result<String, SpydusError> {
    let inner = PEM_BLOCK
        .captures(js)
        .and_then(|c| c.get(1))
        .ok_or_else(|| parse_err("public key not found in encrypt.js"))?
        .as_str();
    // auth.ts:38 — strip JS line-continuations + whitespace.
    let body: Vec<char> = inner.chars().filter(|&c| c != '\\' && !is_js_whitespace(c)).collect();
    if body.is_empty() {
        // auth.ts:39 — `body.match(/.{1,64}/g)!` is null here and the `.join` throws.
        return Err(parse_err("public key in encrypt.js is empty"));
    }
    let lines: Vec<String> = body.chunks(64).map(|chunk| chunk.iter().collect()).collect();
    Ok(format!("{PEM_BEGIN}\n{}\n{PEM_END}", lines.join("\n")))
}

/// RSA PKCS#1 v1.5 encrypt → base64, matching `JSEncrypt.encrypt()`. auth.ts:44-49.
pub fn rsa_encrypt_base64(pem: &str, value: &str) -> Result<String, SpydusError> {
    // node `publicEncrypt` accepts both SPKI ("PUBLIC KEY") and PKCS#1 ("RSA PUBLIC KEY") PEMs.
    let key = RsaPublicKey::from_public_key_pem(pem)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(pem))
        .map_err(|e| parse_err(&format!("invalid RSA public key: {e}")))?;
    let ciphertext = key
        .encrypt(&mut rand::rngs::OsRng, Pkcs1v15Encrypt, value.as_bytes())
        .map_err(|e| parse_err(&format!("RSA encrypt failed: {e}")))?;
    Ok(STANDARD.encode(ciphertext))
}

fn parse_err(message: &str) -> SpydusError {
    SpydusError::Parse { message: message.to_owned() }
}
