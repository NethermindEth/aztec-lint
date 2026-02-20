use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::diagnostics::normalize_file_path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedFiles {
    pub repo_root: PathBuf,
    pub files: BTreeSet<String>,
}

impl ChangedFiles {
    pub fn files_for_root(&self, root: &Path) -> BTreeSet<String> {
        let repo_root = canonical_or_absolute(&self.repo_root);
        let target_root = canonical_or_absolute(root);
        let Ok(prefix) = target_root.strip_prefix(&repo_root) else {
            return BTreeSet::new();
        };
        let normalized_prefix = normalize_file_path(&prefix.to_string_lossy());
        let prefix = normalized_prefix.trim_matches('/');

        if prefix.is_empty() {
            return self.files.clone();
        }

        let prefix_with_sep = format!("{prefix}/");
        self.files
            .iter()
            .filter_map(|file| {
                file.strip_prefix(&prefix_with_sep)
                    .map(str::to_string)
                    .and_then(|value| (!value.is_empty()).then_some(value))
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum ChangedOnlyError {
    NotGitRepository { path: PathBuf },
    GitCommandFailed { args: Vec<String>, stderr: String },
}

impl Display for ChangedOnlyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotGitRepository { path } => write!(
                f,
                "'{}' is not inside a git repository (required for --changed-only)",
                path.display()
            ),
            Self::GitCommandFailed { args, stderr } => write!(
                f,
                "failed to query changed files with git ({}): {}",
                args.join(" "),
                stderr.trim()
            ),
        }
    }
}

impl Error for ChangedOnlyError {}

pub fn changed_files_from_git(path: &Path) -> Result<ChangedFiles, ChangedOnlyError> {
    let probe = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };

    let repo_root = git_repo_root(probe)?;
    let mut files = BTreeSet::<String>::new();
    collect_changed_paths(
        &repo_root,
        &["diff", "--name-only", "--diff-filter=ACMR"],
        &mut files,
    )?;
    collect_changed_paths(
        &repo_root,
        &["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        &mut files,
    )?;
    collect_changed_paths(
        &repo_root,
        &["ls-files", "--others", "--exclude-standard"],
        &mut files,
    )?;

    Ok(ChangedFiles { repo_root, files })
}

fn collect_changed_paths(
    repo_root: &Path,
    args: &[&str],
    out: &mut BTreeSet<String>,
) -> Result<(), ChangedOnlyError> {
    let stdout = run_git(repo_root, args)?;
    for line in stdout.lines() {
        let normalized = normalize_file_path(line.trim());
        if !normalized.is_empty() {
            out.insert(normalized);
        }
    }
    Ok(())
}

fn git_repo_root(path: &Path) -> Result<PathBuf, ChangedOnlyError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .map_err(|_| ChangedOnlyError::NotGitRepository {
            path: path.to_path_buf(),
        })?;
    if !output.status.success() {
        return Err(ChangedOnlyError::NotGitRepository {
            path: path.to_path_buf(),
        });
    }

    let repo_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(repo_root))
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<String, ChangedOnlyError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|err| ChangedOnlyError::GitCommandFailed {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            stderr: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(ChangedOnlyError::GitCommandFailed {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn canonical_or_absolute(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use tempfile::tempdir;

    use super::changed_files_from_git;

    #[test]
    fn changed_files_include_unstaged_staged_and_untracked() {
        let repo = setup_repo();
        let root = repo.path();

        fs::write(root.join("src/main.nr"), "fn main() { let x = 2; }\n")
            .expect("unstaged change should be written");
        fs::write(root.join("src/helper.nr"), "fn helper() { let y = 3; }\n")
            .expect("staged change should be written");
        git(root, &["add", "src/helper.nr"]);
        fs::write(root.join("src/new.nr"), "fn brand_new() {}\n")
            .expect("untracked file should be written");

        let changed = changed_files_from_git(root).expect("changed-only should succeed");
        assert!(changed.files.contains("src/main.nr"));
        assert!(changed.files.contains("src/helper.nr"));
        assert!(changed.files.contains("src/new.nr"));
    }

    #[test]
    fn changed_files_can_be_rebased_to_target_root() {
        let repo = setup_repo();
        let root = repo.path();

        fs::write(root.join("src/main.nr"), "fn main() { let x = 2; }\n")
            .expect("unstaged change should be written");
        fs::write(root.join("src/helper.nr"), "fn helper() { let y = 3; }\n")
            .expect("staged change should be written");
        git(root, &["add", "src/helper.nr"]);
        fs::write(root.join("src/new.nr"), "fn brand_new() {}\n")
            .expect("untracked file should be written");

        let changed =
            changed_files_from_git(root.join("src").as_path()).expect("changed-only should work");
        let rebased = changed.files_for_root(root.join("src").as_path());
        assert!(rebased.contains("main.nr"));
        assert!(rebased.contains("helper.nr"));
        assert!(rebased.contains("new.nr"));
    }

    #[test]
    fn changed_only_reports_non_git_paths() {
        let dir = tempdir().expect("tempdir should be created");
        let err = changed_files_from_git(dir.path()).expect_err("non-git path should fail");
        assert!(err.to_string().contains("git repository"));
    }

    fn setup_repo() -> tempfile::TempDir {
        let repo = tempdir().expect("tempdir should be created");
        let root = repo.path();

        git(root, &["init", "--quiet"]);
        git(root, &["config", "user.name", "Test User"]);
        git(root, &["config", "user.email", "test@example.com"]);
        fs::create_dir_all(root.join("src")).expect("src directory should be created");
        fs::write(root.join("src/main.nr"), "fn main() { let x = 1; }\n")
            .expect("main file should be written");
        fs::write(root.join("src/helper.nr"), "fn helper() { let y = 2; }\n")
            .expect("helper file should be written");
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "init", "--quiet"]);

        repo
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git command should execute");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
