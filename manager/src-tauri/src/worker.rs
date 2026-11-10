//! Asks the Cloudflare Worker for a licence's lease token and reads its expiry.
//! Token: 153 bytes, "ALLT" | u8 version | u32 id | u64 issuedAt |
//! u64 expiresAt (bytes 17..25, little-endian) | key[64] | sig[64], base64.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TokenStatus {
    #[serde(rename_all = "camelCase")]
    Served { expires_at: i64 },
    NotServed,
    Invalid,
    Unreachable { error: String },
}

pub fn decode_expiry(body: &str) -> Option<i64> {
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(body.trim()).ok()?;
    if raw.len() != 153 || &raw[..4] != b"ALLT" {
        return None;
    }
    let bytes: [u8; 8] = raw[17..25].try_into().ok()?;
    i64::try_from(u64::from_le_bytes(bytes)).ok()
}

/// Runs the blocking request on its own thread: Tauri calls async commands on
/// a tokio runtime, where reqwest::blocking panics.
pub fn fetch_status(base: &str, id: u32) -> TokenStatus {
    let url = format!("{}/tokens/{id}.tok", base.trim_end_matches('/'));
    std::thread::spawn(move || fetch_blocking(&url))
        .join()
        .unwrap_or_else(|_| TokenStatus::Unreachable { error: "falha interna".into() })
}

fn fetch_blocking(url: &str) -> TokenStatus {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("alurianot-license-manager")
        .build()
    {
        Ok(c) => c,
        Err(e) => return TokenStatus::Unreachable { error: e.to_string() },
    };
    let resp = match client.get(url).send() {
        Ok(r) => r,
        Err(e) => return TokenStatus::Unreachable { error: e.to_string() },
    };
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return TokenStatus::NotServed;
    }
    if !resp.status().is_success() {
        return TokenStatus::Unreachable { error: format!("HTTP {}", resp.status()) };
    }
    match resp.text().ok().as_deref().and_then(decode_expiry) {
        Some(expires_at) => TokenStatus::Served { expires_at },
        None => TokenStatus::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn token(magic: &[u8; 4], expires: u64) -> String {
        let mut t = Vec::new();
        t.extend_from_slice(magic);
        t.push(1);
        t.extend_from_slice(&3u32.to_le_bytes());
        t.extend_from_slice(&1_762_787_791u64.to_le_bytes());
        t.extend_from_slice(&expires.to_le_bytes());
        t.extend_from_slice(&[0u8; 128]);
        assert_eq!(t.len(), 153);
        base64::engine::general_purpose::STANDARD.encode(t) + "\n"
    }

    #[test]
    fn decode_expiry_reads_field() {
        assert_eq!(decode_expiry(&token(b"ALLT", 1_762_960_591)), Some(1_762_960_591));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert_eq!(decode_expiry("abc"), None);
        assert_eq!(decode_expiry(""), None);
        assert_eq!(decode_expiry(&token(b"XXXX", 5)), None);
        assert_eq!(decode_expiry("no such licence\n"), None);
    }

    #[test]
    fn works_inside_the_tauri_async_runtime() {
        // Tauri runs `#[command(async)]` handlers on its tokio runtime, where
        // reqwest::blocking must not be driven directly.
        let status = tauri::async_runtime::block_on(async { fetch_status("http://127.0.0.1:9", 1) });
        assert!(matches!(status, TokenStatus::Unreachable { .. }), "{status:?}");
    }

    #[test]
    fn unreachable_worker() {
        // Port 9 (discard) on localhost: refused immediately.
        match fetch_status("http://127.0.0.1:9", 1) {
            TokenStatus::Unreachable { .. } => {}
            other => panic!("{other:?}"),
        }
    }
}
