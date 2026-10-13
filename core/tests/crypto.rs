//! crypto.rs — encrypt.js → PEM extraction and JSEncrypt-compatible RSA PKCS#1 v1.5 encryption.
//! Offline only: the key comes from the captured `fixtures/encrypt.js` or is generated locally.

mod common;

use std::sync::LazyLock;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use librespydus_core::SpydusError;
use librespydus_core::crypto::{public_key_pem_from_encrypt_js, rsa_encrypt_base64};
use rsa::pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding};
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

/// The key block of `fixtures/encrypt.js`, rewrapped as auth.ts:38-40 does.
const FIXTURE_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQC82Gt72tYMWhZsxs79+VFHgFP2
kxeFvCuD/eg3NJOXAG9MMMDGzdQCCcbHDUXSHpwRE4ghacQrbr//JfBv+X5ZOeWU
fnw3GUdlC+1CiPLlGzK/5oAl/hoKwaVt8K6MKqZphFsm5ftJbnc8xGNrbVG+AXB0
bIAHZuvUILyfQV0+RwIDAQAB
-----END PUBLIC KEY-----";

/// One locally generated 1024-bit keypair (the size Canning uses), shared by the tests.
static KEYPAIR: LazyLock<RsaPrivateKey> =
    LazyLock::new(|| RsaPrivateKey::new(&mut rand::thread_rng(), 1024).expect("generate RSA key"));

fn local_public_pem() -> String {
    KEYPAIR.to_public_key().to_public_key_pem(LineEnding::LF).expect("encode PEM")
}

fn decrypt(ciphertext_b64: &str) -> String {
    let bytes = STANDARD.decode(ciphertext_b64).expect("base64");
    let plain = KEYPAIR.decrypt(Pkcs1v15Encrypt, &bytes).expect("PKCS#1 v1.5 decrypt");
    String::from_utf8(plain).expect("utf-8 plaintext")
}

/// Embed a PEM the way Canning's encrypt.js does: one JS string with `\` line continuations.
fn as_encrypt_js(pem: &str) -> String {
    let continued = pem.trim_end().lines().collect::<Vec<_>>().join("\\\n");
    format!("function rsaEncrypt(val) {{\n    crypt.setPublicKey('{continued}\\\n');\n}}\n")
}

#[test]
fn fixture_encrypt_js_yields_the_rewrapped_pem() {
    let pem = public_key_pem_from_encrypt_js(&common::fixture("encrypt.js")).unwrap();
    assert_eq!(pem, FIXTURE_PEM);
}

#[test]
fn fixture_pem_parses_as_a_1024_bit_rsa_public_key() {
    let pem = public_key_pem_from_encrypt_js(&common::fixture("encrypt.js")).unwrap();
    let key = RsaPublicKey::from_public_key_pem(&pem).expect("SPKI PEM parses");
    assert_eq!(key.size(), 128);
}

#[test]
fn fixture_key_ciphertext_is_172_base64_chars() {
    let pem = public_key_pem_from_encrypt_js(&common::fixture("encrypt.js")).unwrap();
    let ciphertext = rsa_encrypt_base64(&pem, "1234").unwrap();
    assert_eq!(ciphertext.len(), 172);
    assert_eq!(STANDARD.decode(&ciphertext).unwrap().len(), 128);
    // PKCS#1 v1.5 type-2 padding is randomized, like JSEncrypt: same input, different output.
    assert_ne!(ciphertext, rsa_encrypt_base64(&pem, "1234").unwrap());
}

#[test]
fn pkcs1v15_roundtrip_with_a_local_keypair() {
    let pem = local_public_pem();
    for secret in ["1234", "correct horse battery staple", "pïn-ünïcode ✓", ""] {
        assert_eq!(decrypt(&rsa_encrypt_base64(&pem, secret).unwrap()), secret);
    }
}

#[test]
fn local_key_roundtrips_through_encrypt_js_extraction() {
    let js = as_encrypt_js(&local_public_pem());
    let pem = public_key_pem_from_encrypt_js(&js).unwrap();
    assert!(pem.lines().all(|line| line.len() <= 64));
    assert_eq!(decrypt(&rsa_encrypt_base64(&pem, "s3cret PIN").unwrap()), "s3cret PIN");
}

#[test]
fn pem_extraction_strips_js_whitespace_and_takes_the_first_block() {
    // Tabs, CRLF, NBSP and BOM are JS `\s`; a second key block is ignored (non-greedy match).
    let js = "x('-----BEGIN PUBLIC KEY-----\\\r\n\tAB CD\u{a0}\u{feff}EF\\\n-----END PUBLIC KEY-----');\
              y('-----BEGIN PUBLIC KEY-----ZZ-----END PUBLIC KEY-----');";
    let pem = public_key_pem_from_encrypt_js(js).unwrap();
    assert_eq!(pem, "-----BEGIN PUBLIC KEY-----\nABCDEF\n-----END PUBLIC KEY-----");
}

#[test]
fn pem_extraction_rewraps_at_64_columns() {
    let body = "A".repeat(130);
    let js = format!("-----BEGIN PUBLIC KEY-----{body}-----END PUBLIC KEY-----");
    let pem = public_key_pem_from_encrypt_js(&js).unwrap();
    let lines: Vec<&str> = pem.lines().collect();
    assert_eq!(lines.iter().map(|l| l.len()).collect::<Vec<_>>(), [26, 64, 64, 2, 24]);
}

#[test]
fn missing_or_empty_key_block_is_a_parse_error() {
    for js in [
        "function rsaEncrypt(val) { return val; }",
        "'-----BEGIN PUBLIC KEY-----\\\n \\\n-----END PUBLIC KEY-----'",
    ] {
        assert!(
            matches!(public_key_pem_from_encrypt_js(js), Err(SpydusError::Parse { .. })),
            "{js}"
        );
    }
}

#[test]
fn invalid_pem_is_a_parse_error() {
    let err = rsa_encrypt_base64("-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----", "x");
    assert!(matches!(err, Err(SpydusError::Parse { .. })));
}

#[test]
fn plaintext_longer_than_the_key_allows_is_a_parse_error() {
    // PKCS#1 v1.5 caps a 1024-bit key at 128 - 11 = 117 bytes.
    let pem = public_key_pem_from_encrypt_js(&common::fixture("encrypt.js")).unwrap();
    assert!(rsa_encrypt_base64(&pem, &"x".repeat(117)).is_ok());
    assert!(matches!(rsa_encrypt_base64(&pem, &"x".repeat(118)), Err(SpydusError::Parse { .. })));
}
