//! Game-server licence keys: EC P-256, PKCS#8 PEM private key, public key as
//! 128 hex digits (X || Y). Same format as AlurianOt-ClientOld's gen_license.py.

use std::io::Write;
use std::path::{Path, PathBuf};

use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use p256::SecretKey;
use rand_core::OsRng;

pub struct KeyPair {
    pub pem: String,
    pub public_hex: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn public_hex(secret: &SecretKey) -> String {
    let point = secret.public_key().to_encoded_point(false); // 0x04 || X || Y
    hex(&point.as_bytes()[1..65])
}

pub fn generate() -> KeyPair {
    let secret = SecretKey::random(&mut OsRng);
    let pem = secret
        .to_pkcs8_pem(LineEnding::LF)
        .expect("PKCS#8 encoding of a fresh key")
        .to_string();
    KeyPair { public_hex: public_hex(&secret), pem }
}

pub fn public_hex_from_pem(pem: &str) -> Result<String, String> {
    let secret = SecretKey::from_pkcs8_pem(pem).map_err(|e| format!("chave inválida: {e}"))?;
    Ok(public_hex(&secret))
}

pub fn key_path(keys_dir: &Path, name: &str, id: u32) -> PathBuf {
    keys_dir.join(format!("{name}-{id}.key"))
}

/// Writes <name>-<id>.key and <name>-<id>.json. Never overwrites a key.
pub fn save(keys_dir: &Path, name: &str, id: u32, pair: &KeyPair) -> Result<PathBuf, String> {
    std::fs::create_dir_all(keys_dir)
        .map_err(|e| format!("não foi possível criar {}: {e}", keys_dir.display()))?;
    let path = key_path(keys_dir, name, id);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("não foi possível criar {}: {e}", path.display()))?;
    file.write_all(pair.pem.as_bytes())
        .map_err(|e| format!("erro ao gravar {}: {e}", path.display()))?;
    let json = serde_json::json!({ "name": name, "licenseId": id, "publicKey": pair.public_hex });
    let json_path = keys_dir.join(format!("{name}-{id}.json"));
    std::fs::write(&json_path, format!("{}\n", serde_json::to_string_pretty(&json).unwrap()))
        .map_err(|e| format!("erro ao gravar {}: {e}", json_path.display()))?;
    Ok(path)
}

/// Licence ids taken by key files (<anything>-<digits>.key), including the
/// deleted/ subfolder, so a deleted licence's id is never handed out again.
pub fn ids_in_dir(keys_dir: &Path) -> Vec<u32> {
    let mut ids = Vec::new();
    for dir in [keys_dir.to_path_buf(), keys_dir.join("deleted")] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(stem) = name.strip_suffix(".key") else { continue };
            let Some((_, digits)) = stem.rsplit_once('-') else { continue };
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(id) = digits.parse::<u32>() {
                    ids.push(id);
                }
            }
        }
    }
    ids
}

/// Moves <name>-<id>.key/.json into deleted/ (kept, never destroyed).
pub fn move_to_deleted(keys_dir: &Path, name: &str, id: u32) -> Result<(), String> {
    let deleted = keys_dir.join("deleted");
    for ext in ["key", "json"] {
        let from = keys_dir.join(format!("{name}-{id}.{ext}"));
        if !from.exists() {
            continue;
        }
        std::fs::create_dir_all(&deleted).map_err(|e| e.to_string())?;
        let to = deleted.join(format!("{name}-{id}.{ext}"));
        std::fs::rename(&from, &to)
            .map_err(|e| format!("não foi possível mover {}: {e}", from.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_roundtrip() {
        let p = generate();
        assert!(p.pem.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert_eq!(p.public_hex.len(), 128);
        assert_eq!(public_hex_from_pem(&p.pem).unwrap(), p.public_hex);
        assert_ne!(generate().public_hex, p.public_hex);
    }

    #[test]
    fn reads_python_key() {
        let path = Path::new(r"C:\tmp\licenses\tharcio-2.key");
        let Ok(pem) = std::fs::read_to_string(path) else {
            eprintln!("skipped: {} not present", path.display());
            return;
        };
        assert_eq!(
            public_hex_from_pem(&pem).unwrap(),
            "f832528781da14339f557b67c9e2ffe94c4b72df3c39da4266fb8592d73a26ce122869342eada05856ad2e523727afa846643ba0f3ef0f9a2e43a46290f0b40b"
        );
    }

    #[test]
    fn save_writes_key_and_json_and_refuses_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = generate();
        let path = save(dir.path(), "fulano", 3, &p).unwrap();
        assert_eq!(path, key_path(dir.path(), "fulano", 3));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), p.pem);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("fulano-3.json")).unwrap()).unwrap();
        assert_eq!(json["licenseId"], 3);
        assert_eq!(json["publicKey"], p.public_hex.as_str());
        assert!(save(dir.path(), "fulano", 3, &generate()).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), p.pem);
    }

    #[test]
    fn ids_in_dir_reads_deleted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("deleted")).unwrap();
        for f in ["a-3.key", "deleted/b-7.key", "x.key", "c-4.json", "master.key", "tharcio-2.pub.h"] {
            std::fs::write(dir.path().join(f), "x").unwrap();
        }
        let mut ids = ids_in_dir(dir.path());
        ids.sort();
        assert_eq!(ids, vec![3, 7]);
        assert!(ids_in_dir(&dir.path().join("missing")).is_empty());
    }

    #[test]
    fn move_to_deleted_moves_key_and_json() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), "z", 9, &generate()).unwrap();
        move_to_deleted(dir.path(), "z", 9).unwrap();
        assert!(!dir.path().join("z-9.key").exists());
        assert!(dir.path().join("deleted").join("z-9.key").exists());
        assert!(dir.path().join("deleted").join("z-9.json").exists());
        move_to_deleted(dir.path(), "nobody", 1).unwrap();
    }
}
