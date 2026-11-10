//! git through the installed git.exe (the owner's credentials), without
//! console windows. Only licenses.json is ever committed.

use std::path::Path;
use std::process::Command;

fn run_git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = git(repo, args)
        .output()
        .map_err(|e| format!("git não encontrado: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    Err(format!("git {} falhou: {detail}", args[0]))
}

/// Brings in remote changes first; other local edits are stashed and restored.
pub fn pull(repo: &Path) -> Result<(), String> {
    run_git(repo, &["pull", "--rebase", "--autostash"]).map(|_| ())
}

/// Commits licenses.json only, whatever else is staged in the repo.
pub fn commit_licenses(repo: &Path, message: &str) -> Result<(), String> {
    run_git(repo, &["commit", "-m", message, "--", "licenses.json"]).map(|_| ())
}

pub fn push(repo: &Path) -> Result<(), String> {
    run_git(repo, &["push"]).map(|_| ())
}

/// Local commits not on the remote yet (0 when in sync).
pub fn ahead_count(repo: &Path) -> Result<u32, String> {
    let n = run_git(repo, &["rev-list", "--count", "@{u}..HEAD"])?;
    n.parse().map_err(|_| format!("git rev-list: resposta inesperada {n}"))
}

fn git(repo: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn run(dir: &Path, args: &[&str]) -> String {
        let out = git(dir, args).output().unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// bare remote + clone with licenses.json and other.txt pushed.
    fn setup() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let remote = tmp.path().join("remote.git");
        let clone = tmp.path().join("clone");
        run(tmp.path(), &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]);
        run(tmp.path(), &["clone", "-q", remote.to_str().unwrap(), clone.to_str().unwrap()]);
        run(&clone, &["config", "user.email", "t@t"]);
        run(&clone, &["config", "user.name", "t"]);
        run(&clone, &["checkout", "-q", "-b", "main"]);
        std::fs::write(clone.join("licenses.json"), "{}\n").unwrap();
        std::fs::write(clone.join("other.txt"), "a\n").unwrap();
        run(&clone, &["add", "."]);
        run(&clone, &["commit", "-q", "-m", "init"]);
        run(&clone, &["push", "-q", "-u", "origin", "main"]);
        (tmp, clone)
    }

    #[test]
    fn commit_only_licenses() {
        let (_tmp, clone) = setup();
        std::fs::write(clone.join("other.txt"), "staged\n").unwrap();
        run(&clone, &["add", "other.txt"]);
        std::fs::write(clone.join("licenses.json"), "{\"x\":1}\n").unwrap();
        commit_licenses(&clone, "licença nova: x (#3)").unwrap();
        assert_eq!(run(&clone, &["show", "--name-only", "--format=", "HEAD"]), "licenses.json");
        assert_eq!(run(&clone, &["log", "-1", "--format=%s"]), "licença nova: x (#3)");
        assert_eq!(run(&clone, &["diff", "--cached", "--name-only"]), "other.txt");
        assert_eq!(ahead_count(&clone).unwrap(), 1);
        push(&clone).unwrap();
        assert_eq!(ahead_count(&clone).unwrap(), 0);
    }

    #[test]
    fn push_failure_keeps_commit() {
        let (tmp, clone) = setup();
        let missing = tmp.path().join("nope.git");
        run(&clone, &["remote", "set-url", "origin", missing.to_str().unwrap()]);
        std::fs::write(clone.join("licenses.json"), "{\"y\":2}\n").unwrap();
        commit_licenses(&clone, "revogar: y (#2)").unwrap();
        let err = push(&clone).unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(run(&clone, &["log", "-1", "--format=%s"]), "revogar: y (#2)");
    }

    #[test]
    fn pull_with_dirty_tree() {
        let (_tmp, clone) = setup();
        std::fs::write(clone.join("other.txt"), "dirty\n").unwrap();
        pull(&clone).unwrap();
        assert_eq!(std::fs::read_to_string(clone.join("other.txt")).unwrap(), "dirty\n");
    }

    #[test]
    fn errors_carry_git_message() {
        let tmp = tempfile::tempdir().unwrap();
        let err = pull(tmp.path()).unwrap_err();
        assert!(err.to_lowercase().contains("git"), "{err}");
    }
}
