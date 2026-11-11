//! licenses.json model: parsing, validation, formatting and id rules.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct License {
    pub id: u32,
    pub name: String,
    pub server_public_key: String,
    pub revoked: bool,
    pub expires: Option<String>,
    #[serde(default)]
    pub note: String,
    /// Sellable features (FEATURES ids). Missing or empty = none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LicenseFile {
    pub lease_hours: u32,
    pub licenses: Vec<License>,
    #[serde(default)]
    pub retired_ids: Vec<u32>,
}

/// Sellable DLL features. Append-only: the index is the bit in the v2 token.
pub const FEATURES: [&str; 16] = [
    "store", "market", "forge", "supply_stash", "task_hunt", "daily_reward", "hunting",
    "creature_icons", "guild_emblems", "skills", "side_columns", "store_inbox_purse",
    "login_screen", "discord_rpc", "mount", "auto_updater",
];

/// Bitmask for the v2 lease token (bit = index in FEATURES).
pub fn feature_mask(ids: &[String]) -> Result<u64, String> {
    let mut mask = 0u64;
    for id in ids {
        let bit = FEATURES
            .iter()
            .position(|f| f == id)
            .ok_or_else(|| format!("recurso desconhecido: {id}"))?;
        mask |= 1u64 << bit;
    }
    Ok(mask)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ativa,
    Revogada,
    Vencida,
}

pub fn parse_date(text: &str) -> Option<NaiveDate> {
    if text.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

fn is_public_key(hex: &str) -> bool {
    hex.len() == 128 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Parses and validates licenses.json. Any problem is an error: the app
/// never writes a file it could not fully understand.
pub fn parse(text: &str) -> Result<LicenseFile, String> {
    let file: LicenseFile =
        serde_json::from_str(text).map_err(|e| format!("licenses.json inválido: {e}"))?;
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for l in &file.licenses {
        let who = format!("licença #{} ({})", l.id, l.name);
        if l.id == 0 {
            return Err(format!("licenses.json inválido: {who}: id 0"));
        }
        if !ids.insert(l.id) {
            return Err(format!("licenses.json inválido: id {} repetido", l.id));
        }
        if normalize_name(&l.name).ok().as_deref() != Some(l.name.as_str()) {
            return Err(format!("licenses.json inválido: {who}: nome fora do padrão"));
        }
        if !names.insert(l.name.clone()) {
            return Err(format!("licenses.json inválido: nome {} repetido", l.name));
        }
        if !is_public_key(&l.server_public_key) {
            return Err(format!(
                "licenses.json inválido: {who}: serverPublicKey precisa de 128 dígitos hex"
            ));
        }
        feature_mask(&l.features).map_err(|e| format!("licenses.json inválido: {who}: {e}"))?;
        if let Some(e) = &l.expires {
            if parse_date(e).is_none() {
                return Err(format!("licenses.json inválido: {who}: data {e} (use AAAA-MM-DD)"));
            }
        }
    }
    Ok(file)
}

/// Same layout as the hand-written file: 2-space indent, trailing newline.
pub fn to_json(file: &LicenseFile) -> String {
    let mut out = serde_json::to_string_pretty(file).expect("serialisable");
    out.push('\n');
    out
}

/// Lowercases and checks a licence name: a-z 0-9 - _, 1..32 characters.
pub fn normalize_name(input: &str) -> Result<String, String> {
    let name = input.trim().to_lowercase();
    if name.is_empty() || name.len() > 32 {
        return Err("O nome precisa ter de 1 a 32 caracteres.".into());
    }
    if !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_') {
        return Err("Use só letras sem acento, números, - e _ (sem espaços).".into());
    }
    Ok(name)
}

/// Ids are never reused: a buyer's DLL is built for a fixed id.
pub fn next_id(file: &LicenseFile, key_file_ids: &[u32]) -> u32 {
    file.licenses
        .iter()
        .map(|l| l.id)
        .chain(file.retired_ids.iter().copied())
        .chain(key_file_ids.iter().copied())
        .max()
        .unwrap_or(0)
        + 1
}

/// "expires" is inclusive: the licence still works on that day (UTC).
pub fn status(l: &License, today: NaiveDate) -> Status {
    if l.revoked {
        return Status::Revogada;
    }
    match l.expires.as_deref().and_then(parse_date) {
        Some(end) if end < today => Status::Vencida,
        _ => Status::Ativa,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "bd5af042deeb3f5028e149b3462d4d9b2393d5e5ff7c809cffe9c6fa3f6d19a87c6fce22f899c65192467c67ed99cc9e4dc3a93350611211153aa6d7bf799406";

    fn sample(key: &str) -> String {
        format!(
            "{{\n  \"leaseHours\": 48,\n  \"licenses\": [\n    {{\n      \"id\": 1,\n      \"name\": \"alurianot\",\n      \"serverPublicKey\": \"{key}\",\n      \"revoked\": false,\n      \"expires\": null,\n      \"note\": \"Servidor oficial\"\n    }}\n  ],\n  \"retiredIds\": []\n}}\n"
        )
    }

    fn lic(id: u32, name: &str) -> License {
        License {
            id,
            name: name.into(),
            server_public_key: KEY.into(),
            revoked: false,
            expires: None,
            note: String::new(),
            features: vec![],
        }
    }

    #[test]
    fn parse_roundtrip_keeps_format() {
        let text = sample(KEY);
        let file = parse(&text).unwrap();
        assert_eq!(file.lease_hours, 48);
        assert_eq!(file.licenses[0].name, "alurianot");
        assert_eq!(to_json(&file), text);
    }

    #[test]
    fn parse_adds_missing_retired_ids() {
        let text = sample(KEY).replace(",\n  \"retiredIds\": []", "");
        let file = parse(&text).unwrap();
        assert!(file.retired_ids.is_empty());
        assert_eq!(to_json(&file), sample(KEY));
    }

    #[test]
    fn parse_rejects_bad_key() {
        let err = parse(&sample(&KEY[..127])).unwrap_err();
        assert!(err.contains("serverPublicKey"), "{err}");
    }

    #[test]
    fn parse_rejects_duplicates_and_bad_dates() {
        let mut f = LicenseFile { lease_hours: 48, licenses: vec![lic(1, "a"), lic(1, "b")], retired_ids: vec![] };
        assert!(parse(&to_json_unchecked(&f)).is_err());
        f.licenses[1].id = 2;
        f.licenses[1].name = "a".into();
        assert!(parse(&to_json_unchecked(&f)).is_err());
        f.licenses[1].name = "b".into();
        f.licenses[1].expires = Some("2026-13-01".into());
        assert!(parse(&to_json_unchecked(&f)).is_err());
        f.licenses[1].expires = Some("2026-12-01".into());
        assert!(parse(&to_json_unchecked(&f)).is_ok());
    }

    #[test]
    fn feature_mask_bits() {
        assert_eq!(feature_mask(&["store".into(), "auto_updater".into()]).unwrap(), 1 | (1 << 15));
        assert_eq!(feature_mask(&[]).unwrap(), 0);
        assert!(feature_mask(&["nope".into()]).unwrap_err().contains("nope"));
    }

    #[test]
    fn parse_validates_features_and_keeps_order() {
        let with = sample(KEY).replace(
            "\"note\": \"Servidor oficial\"",
            "\"note\": \"Servidor oficial\",
      \"features\": [
        \"market\",
        \"mount\"
      ]",
        );
        let file = parse(&with).unwrap();
        assert_eq!(file.licenses[0].features, vec!["market".to_string(), "mount".to_string()]);
        assert_eq!(to_json(&file), with);
        assert!(parse(&with.replace("\"mount\"", "\"xyz\"")).unwrap_err().contains("xyz"));
    }

    #[test]
    fn parse_rejects_unknown_fields() {
        // A typo like "expire" must not be accepted and then silently dropped.
        let err = parse(&sample(KEY).replace("\"expires\": null", "\"expires\": null,\n      \"expire\": \"2026-12-31\"")).unwrap_err();
        assert!(err.contains("expire"), "{err}");
        assert!(parse(&sample(KEY).replace("\"leaseHours\"", "\"leaseHour\": 1,\n  \"leaseHours\"")).is_err());
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse("{ not json").unwrap_err().contains("licenses.json"));
    }

    #[test]
    fn normalize_name_rules() {
        assert_eq!(normalize_name("Tharcio").unwrap(), "tharcio");
        assert_eq!(normalize_name("  fulano_2-x ").unwrap(), "fulano_2-x");
        assert!(normalize_name("joão").is_err());
        assert!(normalize_name("a b").is_err());
        assert!(normalize_name("").is_err());
        assert!(normalize_name(&"a".repeat(33)).is_err());
        assert!(normalize_name(&"a".repeat(32)).is_ok());
    }

    #[test]
    fn next_id_never_reuses() {
        let f = LicenseFile { lease_hours: 48, licenses: vec![lic(1, "a"), lic(2, "b")], retired_ids: vec![5] };
        assert_eq!(next_id(&f, &[3]), 6);
        let empty = LicenseFile { lease_hours: 48, licenses: vec![], retired_ids: vec![] };
        assert_eq!(next_id(&empty, &[]), 1);
        assert_eq!(next_id(&empty, &[9]), 10);
    }

    #[test]
    fn status_rules() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let mut l = lic(1, "a");
        assert_eq!(status(&l, today), Status::Ativa);
        l.expires = Some("2026-11-10".into());
        assert_eq!(status(&l, today), Status::Ativa);
        l.expires = Some("2026-11-09".into());
        assert_eq!(status(&l, today), Status::Vencida);
        l.revoked = true;
        assert_eq!(status(&l, today), Status::Revogada);
    }

    fn to_json_unchecked(f: &LicenseFile) -> String {
        serde_json::to_string_pretty(f).unwrap()
    }
}
