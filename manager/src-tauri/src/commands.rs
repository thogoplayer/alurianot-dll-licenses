//! Tauri commands: every change goes pull → validate → edit → commit → push.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use crate::licenses::{self, License, LicenseFile, Status};
use crate::settings::{self, Settings};
use crate::{build, gitops, handoff, keys, profile, worker};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseView {
    pub id: u32,
    pub name: String,
    pub status: String,
    pub expires: Option<String>,
    pub note: String,
    pub has_key: bool,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub pushed: bool,
    pub message: String,
}

// ---- pure core ---------------------------------------------------------

pub fn apply_create(
    file: &mut LicenseFile,
    name: &str,
    id: u32,
    public_hex: &str,
    expires: Option<String>,
    note: &str,
) -> Result<(), String> {
    if file.licenses.iter().any(|l| l.name == name) {
        return Err(format!("Já existe uma licença chamada {name}."));
    }
    if file.licenses.iter().any(|l| l.id == id) || file.retired_ids.contains(&id) {
        return Err(format!("O id {id} já foi usado."));
    }
    let expires = expires.filter(|e| !e.trim().is_empty());
    if let Some(e) = &expires {
        if licenses::parse_date(e).is_none() {
            return Err(format!("Data inválida: {e} (use AAAA-MM-DD)."));
        }
    }
    file.licenses.push(License {
        id,
        name: name.to_string(),
        server_public_key: public_hex.to_string(),
        revoked: false,
        expires,
        note: note.trim().to_string(),
        features: vec![],
    });
    Ok(())
}

/// Sets the licence's features: validated, deduplicated, in FEATURES order.
pub fn apply_features(file: &mut LicenseFile, id: u32, features: &[String]) -> Result<String, String> {
    licenses::feature_mask(features)?;
    let ordered: Vec<String> = licenses::FEATURES
        .iter()
        .filter(|f| features.iter().any(|x| x == *f))
        .map(|f| f.to_string())
        .collect();
    let l = file
        .licenses
        .iter_mut()
        .find(|l| l.id == id)
        .ok_or_else(|| format!("Licença #{id} não encontrada."))?;
    l.features = ordered;
    Ok(format!("recursos: {} (#{id})", l.name))
}

/// New expiry date (inclusive, AAAA-MM-DD) or none.
pub fn apply_expires(file: &mut LicenseFile, id: u32, expires: Option<String>) -> Result<String, String> {
    let expires = expires.map(|e| e.trim().to_string()).filter(|e| !e.is_empty());
    if let Some(e) = &expires {
        if licenses::parse_date(e).is_none() {
            return Err(format!("Data inválida: {e} (use AAAA-MM-DD)."));
        }
    }
    let l = file
        .licenses
        .iter_mut()
        .find(|l| l.id == id)
        .ok_or_else(|| format!("Licença #{id} não encontrada."))?;
    l.expires = expires;
    Ok(format!("validade: {} (#{id})", l.name))
}

pub fn apply_delete(file: &mut LicenseFile, id: u32) -> Result<License, String> {
    let pos = file
        .licenses
        .iter()
        .position(|l| l.id == id)
        .ok_or_else(|| format!("Licença #{id} não encontrada."))?;
    let removed = file.licenses.remove(pos);
    if !file.retired_ids.contains(&id) {
        file.retired_ids.push(id);
        file.retired_ids.sort_unstable();
    }
    Ok(removed)
}

fn read_file(repo: &Path) -> Result<LicenseFile, String> {
    let path = repo.join("licenses.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("não foi possível ler {}: {e}", path.display()))?;
    licenses::parse(&text)
}

fn write_atomic(repo: &Path, file: &LicenseFile) -> Result<(), String> {
    let path = repo.join("licenses.json");
    let tmp = repo.join("licenses.json.tmp");
    std::fs::write(&tmp, licenses::to_json(file)).map_err(|e| format!("erro ao gravar: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("erro ao gravar: {e}"))
}

/// pull → parse (abort untouched on error) → edit → atomic write → commit → push.
pub fn modify(
    repo: &Path,
    edit: impl FnOnce(&mut LicenseFile) -> Result<String, String>,
) -> Result<OpResult, String> {
    gitops::check_branch(repo)?;
    gitops::pull(repo)?;
    let mut file = read_file(repo)?;
    let message = edit(&mut file)?;
    // Validate the result exactly as it will be read back.
    licenses::parse(&licenses::to_json(&file))?;
    write_atomic(repo, &file)?;
    gitops::commit_licenses(repo, &message)?;
    Ok(match gitops::push(repo) {
        Ok(()) => OpResult { pushed: true, message: format!("{message} — enviado ao GitHub.") },
        Err(e) => OpResult { pushed: false, message: e },
    })
}

/// "Tentar enviar de novo": bring in remote changes first, then push.
pub fn retry(repo: &Path) -> Result<OpResult, String> {
    gitops::check_branch(repo)?;
    gitops::pull(repo)?;
    Ok(match gitops::push(repo) {
        Ok(()) => OpResult { pushed: true, message: "Alterações enviadas ao GitHub.".into() },
        Err(e) => OpResult { pushed: false, message: e },
    })
}

// ---- Tauri commands ----------------------------------------------------

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

fn current(app: &tauri::AppHandle) -> Result<Settings, String> {
    Ok(settings::load(&config_dir(app)?))
}

fn find(file: &LicenseFile, id: u32) -> Result<License, String> {
    file.licenses
        .iter()
        .find(|l| l.id == id)
        .cloned()
        .ok_or_else(|| format!("Licença #{id} não encontrada."))
}

fn open_explorer(arg: String) {
    // Explorer itself must show a window; its exit code is meaningless.
    let _ = std::process::Command::new("explorer").arg(arg).spawn();
}

#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    current(&app)
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    settings::save(&config_dir(&app)?, &settings)
}

#[tauri::command(async)]
pub fn list_licenses(app: tauri::AppHandle) -> Result<Vec<LicenseView>, String> {
    let s = current(&app)?;
    let file = read_file(Path::new(&s.repo_dir))?;
    let today = chrono::Utc::now().date_naive();
    let keys_dir = PathBuf::from(&s.keys_dir);
    Ok(file
        .licenses
        .iter()
        .map(|l| LicenseView {
            id: l.id,
            name: l.name.clone(),
            status: match licenses::status(l, today) {
                Status::Ativa => "ativa",
                Status::Revogada => "revogada",
                Status::Vencida => "vencida",
            }
            .into(),
            expires: l.expires.clone(),
            note: l.note.clone(),
            has_key: keys::key_path(&keys_dir, &l.name, l.id).is_file(),
            features: l.features.clone(),
        })
        .collect())
}

#[tauri::command(async)]
pub fn pull_latest(app: tauri::AppHandle) -> Result<(), String> {
    gitops::pull(Path::new(&current(&app)?.repo_dir))
}

#[tauri::command(async)]
pub fn create_license(
    app: tauri::AppHandle,
    name: String,
    expires: Option<String>,
    note: String,
    features: Vec<String>,
) -> Result<OpResult, String> {
    let s = current(&app)?;
    let name = licenses::normalize_name(&name)?;
    let keys_dir = PathBuf::from(&s.keys_dir);
    modify(Path::new(&s.repo_dir), |file| {
        if file.licenses.iter().any(|l| l.name == name) {
            return Err(format!("Já existe uma licença chamada {name}."));
        }
        let id = licenses::next_id(file, &keys::ids_in_dir(&keys_dir));
        let pair = keys::generate();
        keys::save(&keys_dir, &name, id, &pair)?;
        apply_create(file, &name, id, &pair.public_hex, expires, &note)?;
        apply_features(file, id, &features)?;
        Ok(format!("licença nova: {name} (#{id})"))
    })
}

#[tauri::command(async)]
pub fn set_revoked(app: tauri::AppHandle, id: u32, revoked: bool) -> Result<OpResult, String> {
    let s = current(&app)?;
    modify(Path::new(&s.repo_dir), |file| {
        let l = file
            .licenses
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or_else(|| format!("Licença #{id} não encontrada."))?;
        l.revoked = revoked;
        let verb = if revoked { "revogar" } else { "reativar" };
        Ok(format!("{verb}: {} (#{id})", l.name))
    })
}

#[tauri::command(async)]
pub fn set_features(app: tauri::AppHandle, id: u32, features: Vec<String>) -> Result<OpResult, String> {
    let s = current(&app)?;
    modify(Path::new(&s.repo_dir), |file| apply_features(file, id, &features))
}

#[tauri::command(async)]
pub fn set_expires(app: tauri::AppHandle, id: u32, expires: Option<String>) -> Result<OpResult, String> {
    let s = current(&app)?;
    modify(Path::new(&s.repo_dir), |file| apply_expires(file, id, expires))
}

#[tauri::command(async)]
pub fn delete_license(app: tauri::AppHandle, id: u32) -> Result<OpResult, String> {
    let s = current(&app)?;
    let keys_dir = PathBuf::from(&s.keys_dir);
    let mut removed_name = String::new();
    let result = modify(Path::new(&s.repo_dir), |file| {
        let removed = apply_delete(file, id)?;
        removed_name = removed.name.clone();
        Ok(format!("apagar: {} (#{id})", removed.name))
    })?;
    keys::move_to_deleted(&keys_dir, &removed_name, id)?;
    Ok(result)
}

#[tauri::command(async)]
pub fn retry_push(app: tauri::AppHandle) -> Result<OpResult, String> {
    retry(Path::new(&current(&app)?.repo_dir))
}

#[tauri::command(async)]
pub fn pending_push(app: tauri::AppHandle) -> Result<bool, String> {
    let s = current(&app)?;
    Ok(gitops::ahead_count(Path::new(&s.repo_dir)).map(|n| n > 0).unwrap_or(false))
}

#[tauri::command]
pub fn config_text(app: tauri::AppHandle, id: u32) -> Result<String, String> {
    Ok(handoff::config_text(&current(&app)?.worker_base, id))
}

#[tauri::command]
pub fn prepare_handoff(app: tauri::AppHandle, id: u32) -> Result<String, String> {
    let s = current(&app)?;
    let l = find(&read_file(Path::new(&s.repo_dir))?, id)?;
    let out = handoff::prepare(Path::new(&s.keys_dir), &l.name, id, &s.worker_base)?;
    let shown = out.display().to_string().replace('/', "\\");
    open_explorer(shown.clone());
    Ok(shown)
}

#[tauri::command]
pub fn open_key_folder(app: tauri::AppHandle, id: u32) -> Result<(), String> {
    let s = current(&app)?;
    let l = find(&read_file(Path::new(&s.repo_dir))?, id)?;
    let key = keys::key_path(Path::new(&s.keys_dir), &l.name, id);
    if !key.is_file() {
        return Err("A chave desta licença não está neste PC.".into());
    }
    open_explorer(format!("/select,{}", key.display()));
    Ok(())
}

fn licence(app: &tauri::AppHandle, id: u32) -> Result<(Settings, License), String> {
    let s = current(app)?;
    let l = find(&read_file(Path::new(&s.repo_dir))?, id)?;
    Ok((s, l))
}

/// Saved buyer profile, or a new one filled from the current config.h.
#[tauri::command]
pub fn get_profile(app: tauri::AppHandle, id: u32) -> Result<profile::Profile, String> {
    let (s, l) = licence(&app, id)?;
    if let Some(p) = profile::load(Path::new(&s.keys_dir), &l.name, id) {
        return Ok(p);
    }
    let config = Path::new(&s.client_dir).join("dll").join("src").join("config.h");
    let text = std::fs::read_to_string(&config)
        .map_err(|e| format!("não foi possível ler {}: {e}", config.display()))?;
    Ok(profile::for_new_buyer(&text))
}

#[tauri::command]
pub fn save_profile(app: tauri::AppHandle, id: u32, profile: profile::Profile) -> Result<(), String> {
    let (s, l) = licence(&app, id)?;
    profile::save(Path::new(&s.keys_dir), &l.name, id, &profile)
}

#[tauri::command(async)]
pub fn build_dll(app: tauri::AppHandle, id: u32) -> Result<build::BuildReport, String> {
    let (s, l) = licence(&app, id)?;
    let keys_dir = PathBuf::from(&s.keys_dir);
    let p = profile::load(&keys_dir, &l.name, id).ok_or("Salve o perfil do comprador antes de gerar a DLL.")?;
    if p.server_host.trim().is_empty() || p.server_name.trim().is_empty() || p.login_store_dir.trim().is_empty() {
        return Err("Preencha pelo menos IP/domínio, nome do servidor e pasta do login.".into());
    }
    if !p.updater_url.is_empty() && !p.updater_url.ends_with('/') {
        return Err("O link do auto updater precisa terminar com /".into());
    }
    let report = build::build(&build::BuildInput {
        client_dir: Path::new(&s.client_dir),
        msbuild: Path::new(&s.msbuild_path),
        keys_dir: &keys_dir,
        worker_base: &s.worker_base,
        name: &l.name,
        id,
        profile: &p,
    })?;
    open_explorer(report.folder.clone());
    Ok(report)
}

#[tauri::command]
pub fn prepare_updater_handoff(app: tauri::AppHandle, id: u32) -> Result<String, String> {
    let (s, l) = licence(&app, id)?;
    let out = build::prepare_updater_handoff(Path::new(&s.client_dir), Path::new(&s.keys_dir), &l.name, id)?;
    let shown = out.display().to_string().replace('/', "\\");
    open_explorer(shown.clone());
    Ok(shown)
}

#[tauri::command(async)]
pub fn token_status(app: tauri::AppHandle, id: u32) -> Result<worker::TokenStatus, String> {
    Ok(worker::fetch_status(&current(&app)?.worker_base, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "bd5af042deeb3f5028e149b3462d4d9b2393d5e5ff7c809cffe9c6fa3f6d19a87c6fce22f899c65192467c67ed99cc9e4dc3a93350611211153aa6d7bf799406";

    fn base_file() -> LicenseFile {
        LicenseFile {
            lease_hours: 48,
            licenses: vec![License {
                id: 1,
                name: "alurianot".into(),
                server_public_key: KEY.into(),
                revoked: false,
                expires: None,
                note: String::new(),
                features: vec![],
            }],
            retired_ids: vec![],
        }
    }

    #[test]
    fn create_adds_and_rejects_duplicates() {
        let mut f = base_file();
        apply_create(&mut f, "fulano", 3, KEY, Some("2027-01-31".into()), "teste").unwrap();
        let l = f.licenses.last().unwrap();
        assert_eq!((l.id, l.name.as_str(), l.expires.as_deref(), l.note.as_str()), (3, "fulano", Some("2027-01-31"), "teste"));
        assert!(!l.revoked);
        assert!(apply_create(&mut f, "fulano", 4, KEY, None, "").is_err());
        assert!(apply_create(&mut f, "outro", 5, KEY, Some("31/01/2027".into()), "").is_err());
        assert!(licenses::parse(&licenses::to_json(&f)).is_ok());
    }

    #[test]
    fn apply_features_validates_orders_and_dedups() {
        let mut f = base_file();
        let msg = apply_features(&mut f, 1, &["mount".into(), "store".into(), "mount".into()]).unwrap();
        assert_eq!(msg, "recursos: alurianot (#1)");
        assert_eq!(f.licenses[0].features, vec!["store".to_string(), "mount".to_string()]);
        assert!(apply_features(&mut f, 1, &["xyz".into()]).unwrap_err().contains("xyz"));
        assert!(apply_features(&mut f, 9, &[]).is_err());
        apply_features(&mut f, 1, &[]).unwrap();
        assert!(f.licenses[0].features.is_empty());
    }

    #[test]
    fn apply_expires_sets_and_clears() {
        let mut f = base_file();
        let msg = apply_expires(&mut f, 1, Some("2027-05-31".into())).unwrap();
        assert_eq!(msg, "validade: alurianot (#1)");
        assert_eq!(f.licenses[0].expires.as_deref(), Some("2027-05-31"));
        assert!(apply_expires(&mut f, 1, Some("31/05/2027".into())).is_err());
        assert_eq!(f.licenses[0].expires.as_deref(), Some("2027-05-31"));
        apply_expires(&mut f, 1, None).unwrap();
        assert_eq!(f.licenses[0].expires, None);
        apply_expires(&mut f, 1, Some("  ".into())).unwrap();
        assert_eq!(f.licenses[0].expires, None);
    }

    #[test]
    fn delete_retires_id() {
        let mut f = base_file();
        let removed = apply_delete(&mut f, 1).unwrap();
        assert_eq!(removed.name, "alurianot");
        assert!(f.licenses.is_empty());
        assert_eq!(f.retired_ids, vec![1]);
        assert!(apply_delete(&mut f, 1).is_err());
    }

    fn run(dir: &Path, args: &[&str]) {
        let st = std::process::Command::new("git").current_dir(dir).args(args).status().unwrap();
        assert!(st.success(), "git {args:?}");
    }

    fn repo_with(text: &str) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let remote = tmp.path().join("remote.git");
        let clone = tmp.path().join("clone");
        run(tmp.path(), &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]);
        run(tmp.path(), &["clone", "-q", remote.to_str().unwrap(), clone.to_str().unwrap()]);
        run(&clone, &["config", "user.email", "t@t"]);
        run(&clone, &["config", "user.name", "t"]);
        run(&clone, &["checkout", "-q", "-b", "main"]);
        std::fs::write(clone.join("licenses.json"), text).unwrap();
        run(&clone, &["add", "."]);
        run(&clone, &["commit", "-q", "-m", "init"]);
        run(&clone, &["push", "-q", "-u", "origin", "main"]);
        (tmp, clone)
    }

    #[test]
    fn retry_pulls_before_pushing() {
        let (tmp, clone) = repo_with(&licenses::to_json(&base_file()));
        // Remote moves on (another PC) while a local licence commit is pending.
        let other = tmp.path().join("other");
        run(tmp.path(), &["clone", "-q", tmp.path().join("remote.git").to_str().unwrap(), other.to_str().unwrap()]);
        run(&other, &["config", "user.email", "o@o"]);
        run(&other, &["config", "user.name", "o"]);
        std::fs::write(other.join("README.md"), "x\n").unwrap();
        run(&other, &["add", "README.md"]);
        run(&other, &["commit", "-q", "-m", "readme"]);
        run(&other, &["push", "-q"]);
        let mut f = base_file();
        f.licenses[0].note = "pendente".into();
        std::fs::write(clone.join("licenses.json"), licenses::to_json(&f)).unwrap();
        gitops::commit_licenses(&clone, "nota").unwrap();

        let r = retry(&clone).unwrap();
        assert!(r.pushed, "{r:?}");
        assert_eq!(gitops::ahead_count(&clone).unwrap(), 0);
    }

    #[test]
    fn modify_refuses_other_branch() {
        let (_tmp, clone) = repo_with(&licenses::to_json(&base_file()));
        run(&clone, &["checkout", "-q", "-b", "feature"]);
        let err = modify(&clone, |f| {
            f.licenses[0].revoked = true;
            Ok("revogar".into())
        })
        .unwrap_err();
        assert!(err.contains("main"), "{err}");
        assert!(!licenses::parse(&std::fs::read_to_string(clone.join("licenses.json")).unwrap()).unwrap().licenses[0].revoked);
        assert!(retry(&clone).is_err());
    }

    #[test]
    fn modify_refuses_invalid_json() {
        let (_tmp, clone) = repo_with("{ broken\n");
        let err = modify(&clone, |_| Ok("x".into())).unwrap_err();
        assert!(err.contains("licenses.json inválido"), "{err}");
        assert_eq!(std::fs::read_to_string(clone.join("licenses.json")).unwrap(), "{ broken\n");
    }

    #[test]
    fn modify_commits_pushes_and_reports_push_failure() {
        let (tmp, clone) = repo_with(&licenses::to_json(&base_file()));
        let ok = modify(&clone, |f| {
            f.licenses[0].revoked = true;
            Ok("revogar: alurianot (#1)".into())
        })
        .unwrap();
        assert!(ok.pushed, "{ok:?}");
        let saved = licenses::parse(&std::fs::read_to_string(clone.join("licenses.json")).unwrap()).unwrap();
        assert!(saved.licenses[0].revoked);

        // Remote reachable but refusing pushes: committed locally, pushed=false.
        std::fs::write(tmp.path().join("remote.git/hooks/pre-receive"), "#!/bin/sh\nexit 1\n").unwrap();
        let refused = modify(&clone, |f| {
            f.licenses[0].note = "x".into();
            Ok("nota: alurianot (#1)".into())
        })
        .unwrap();
        assert!(!refused.pushed);
        assert!(refused.message.contains("git push"), "{refused:?}");
        assert_eq!(gitops::ahead_count(&clone).unwrap(), 1);

        let missing = tmp.path().join("nope.git");
        run(&clone, &["remote", "set-url", "origin", missing.to_str().unwrap()]);
        // pull fails first: nothing is written.
        let err = modify(&clone, |f| {
            f.licenses[0].revoked = false;
            Ok("reativar: alurianot (#1)".into())
        })
        .unwrap_err();
        assert!(err.contains("git pull"), "{err}");
        assert!(licenses::parse(&std::fs::read_to_string(clone.join("licenses.json")).unwrap()).unwrap().licenses[0].revoked);
    }
}
