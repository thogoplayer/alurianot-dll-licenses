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
/// A rebase that stops on a conflict is aborted so the working copy is left
/// exactly as it was.
pub fn pull(repo: &Path) -> Result<(), String> {
    match run_git(repo, &["pull", "--rebase", "--autostash"]) {
        Ok(_) => Ok(()),
        Err(e) => {
            let git_dir = repo.join(".git");
            if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
                let _ = run_git(repo, &["rebase", "--abort"]);
                return Err(format!(
                    "{e}\n\nO licenses.json mudou no GitHub e conflita com uma alteração local \
                     ainda não enviada. Nada foi alterado; resolva o conflito no repositório."
                ));
            }
            Err(e)
        }
    }
}

/// Commits licenses.json only, whatever else is staged in the repo.
pub fn commit_licenses(repo: &Path, message: &str) -> Result<(), String> {
    run_git(repo, &["commit", "-m", message, "--", "licenses.json"]).map(|_| ())
}

pub fn push(repo: &Path) -> Result<(), String> {
    run_git(repo, &["push"]).map(|_| ())
}

/// The Worker reads `main`: refuse to commit or push anywhere else.
pub fn check_branch(repo: &Path) -> Result<(), String> {
    let branch = run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch != "main" {
        return Err(format!(
            "O repositório de licenças está no branch {branch}, não no main. \
             Faça checkout do main antes de alterar licenças."
        ));
    }
    run_git(repo, &["rev-parse", "--abbrev-ref", "main@{u}"])
        .map(|_| ())
        .map_err(|_| "O branch main não tem um remoto (upstream) configurado.".to_string())
}

/// Local commits not on the remote yet (0 when in sync).
pub fn ahead_count(repo: &Path) -> Result<u32, String> {
    let n = run_git(repo, &["rev-list", "--count", "@{u}..HEAD"])?;
    n.parse().map_err(|_| format!("git rev-list: resposta inesperada {n}"))
}

fn git(repo: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo).args(args);
    // Never wait for a username/password prompt nobody can see.
    cmd.env("GIT_TERMINAL_PROMPT", "0");
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

    fn second_clone(tmp: &Path) -> PathBuf {
        let other = tmp.join("other");
        run(tmp, &["clone", "-q", tmp.join("remote.git").to_str().unwrap(), other.to_str().unwrap()]);
        run(&other, &["config", "user.email", "o@o"]);
        run(&other, &["config", "user.name", "o"]);
        other
    }

    #[test]
    fn pull_conflict_is_aborted() {
        let (tmp, clone) = setup();
        let other = second_clone(tmp.path());
        std::fs::write(other.join("licenses.json"), "{\"remote\":1}\n").unwrap();
        run(&other, &["commit", "-q", "-am", "remote edit"]);
        run(&other, &["push", "-q"]);
        std::fs::write(clone.join("licenses.json"), "{\"local\":1}\n").unwrap();
        commit_licenses(&clone, "local edit").unwrap();
        let head = run(&clone, &["rev-parse", "HEAD"]);

        let err = pull(&clone).unwrap_err();
        assert!(err.contains("git pull"), "{err}");
        assert_eq!(run(&clone, &["rev-parse", "HEAD"]), head);
        // Same content; core.autocrlf may rewrite the line ending on abort.
        assert_eq!(
            std::fs::read_to_string(clone.join("licenses.json")).unwrap().replace("\r\n", "\n"),
            "{\"local\":1}\n"
        );
        assert!(!clone.join(".git/rebase-merge").exists());
        assert!(!clone.join(".git/rebase-apply").exists());
    }

    #[test]
    fn git_never_prompts() {
        let (_tmp, clone) = setup();
        let out = run(&clone, &["-c", "alias.envtest=!echo \"$GIT_TERMINAL_PROMPT\"", "envtest"]);
        assert_eq!(out, "0");
    }

    #[test]
    fn check_branch_requires_main_with_upstream() {
        let (_tmp, clone) = setup();
        check_branch(&clone).unwrap();
        run(&clone, &["checkout", "-q", "-b", "feature"]);
        let err = check_branch(&clone).unwrap_err();
        assert!(err.contains("main"), "{err}");
        run(&clone, &["checkout", "-q", "main"]);
        run(&clone, &["branch", "--unset-upstream"]);
        assert!(check_branch(&clone).is_err());
    }

    #[test]
    fn errors_carry_git_message() {
        let tmp = tempfile::tempdir().unwrap();
        let err = pull(tmp.path()).unwrap_err();
        assert!(err.to_lowercase().contains("git"), "{err}");
    }
}
