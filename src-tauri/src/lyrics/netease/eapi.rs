//! Minimal NetEase eapi request encryption (song/lyric/v1).

use aes::cipher::{block_padding::Pkcs7, KeyInit};
use aes::Aes128;
use ecb::cipher::BlockEncryptMut;
use ecb::Encryptor;
use std::time::Duration;

type AesEcb = Encryptor<Aes128>;

pub const LYRIC_V1_PATH: &str = "/api/song/lyric/v1";

pub fn encrypt_eapi_body(api_path: &str, plaintext: &str) -> String {
    let digest = md5::compute(format!("nobody{api_path}use{plaintext}md5forencrypt").as_bytes());
    let cipher = AesEcb::new_from_slice(digest.0.as_slice()).expect("md5 key len");
    let mut buf = vec![0u8; plaintext.len() + 16];
    let enc_len = cipher
        .encrypt_padded_b2b_mut::<Pkcs7>(plaintext.as_bytes(), &mut buf)
        .expect("pkcs7")
        .len();
    hex_encode(&buf[..enc_len])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn post_eapi(
    client: &reqwest::blocking::Client,
    api_path: &str,
    url_suffix: &str,
    json_body: &str,
) -> Result<Vec<u8>, String> {
    let enc = encrypt_eapi_body(api_path, json_body);
    let body = format!("params={enc}");
    let url = format!("https://interface.music.163.com/eapi{url_suffix}");
    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", super::USER_AGENT)
        .header("Referer", "https://music.163.com/")
        .timeout(Duration::from_secs(4))
        .body(body)
        .send()
        .map_err(|e| format!("eapi request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("eapi HTTP {}", resp.status()));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("eapi body: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_is_deterministic() {
        let a = encrypt_eapi_body(LYRIC_V1_PATH, r#"{"id":"1"}"#);
        let b = encrypt_eapi_body(LYRIC_V1_PATH, r#"{"id":"1"}"#);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }
}