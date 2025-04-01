//! Provides asynchronous versions of repository operations using tokio.

use crate::error::GitError;
use crate::models::*;
use crate::repository::CloneOptions;
use crate::types::{BranchName, CommitHash, GitUrl, Remote, Result};

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::{self, FromStr};

use tokio::process::Command;

/// Represents a local Git repository with async operations.
#[derive(Debug, Clone)]
pub struct AsyncRepository {
    location: PathBuf,
}

impl AsyncRepository {
    /// Opens an existing Git repository for analysis.
    ///
    /// This checks if the path is actually a valid Git repository
    /// by running `git rev-parse --is-inside-work-tree`.
    ///
    /// # Arguments
    /// * `p` - The path to the local repository's root directory.
    ///
    /// # Errors
    /// Returns `GitError::NotAGitRepository` if the path is not a valid Git repository.
    pub async fn open<P: AsRef<Path>>(p: P) -> Result<AsyncRepository> {
        let repo = AsyncRepository {
            location: PathBuf::from(p.as_ref()),
        };

        // Verify this is actually a git repository
        match repo.cmd_out(["rev-parse", "--is-inside-work-tree"]).await {
            Ok(output) => {
                if output.is_empty() || output[0] != "true" {
                    return Err(GitError::NotAGitRepository(
                        p.as_ref().display().to_string(),
                    ));
                }
            }
            Err(_) => {
                return Err(GitError::NotAGitRepository(
                    p.as_ref().display().to_string(),
                ));
            }
        }

        Ok(repo)
    }

    /// Clones a remote Git repository into a specified local path asynchronously.
    ///
    /// Equivalent to `git clone <url> <path>` with optional arguments based on CloneOptions.
    ///
    /// # Arguments
    /// * `url` - The URL of the remote repository.
    /// * `p` - The target local path where the repository should be cloned.
    /// * `options` - Options to customize the clone operation.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn clone<P: AsRef<Path>>(
        url: GitUrl,
        p: P,
        options: Option<CloneOptions>,
    ) -> Result<AsyncRepository> {
        let p_ref = p.as_ref();
        let cwd = std::env::current_dir().map_err(|_| GitError::WorkingDirectoryInaccessible)?;

        let mut args = vec!["clone".as_ref()];

        // Apply optional clone arguments
        if let Some(opts) = options {
            if opts.shallow {
                args.push("--depth".as_ref());
                args.push("1".as_ref());
            }

            if opts.no_blob_content {
                args.push("--filter=blob:none".as_ref());
            }

            if let Some(branch) = &opts.single_branch {
                args.push("--single-branch".as_ref());
                args.push("--branch".as_ref());
                args.push(branch.as_ref());
            }

            if opts.bare {
                args.push("--bare".as_ref());
            }

            if opts.mirror {
                args.push("--mirror".as_ref());
            }
        }

        args.push(url.as_ref());
        args.push(p_ref.as_os_str());

        execute_git_async(cwd, args).await?;

        Self::open(p_ref).await
    }

    /// Lists the names of all local branches asynchronously.
    ///
    /// Equivalent to `git branch --format='%(refname:short)'`.
    ///
    /// # Returns
    /// A `Vec<BranchName>` containing the branch names.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn list_branches(&self) -> Result<Vec<BranchName>> {
        execute_git_fn_async(
            &self.location,
            &["branch", "--list", "--format=%(refname:short)"],
            |output| {
                output
                    .lines()
                    .map(|line| BranchName::from_str(line.trim()))
                    .collect::<Result<Vec<BranchName>>>()
            },
        ).await
    }

    /// Lists all files currently tracked by Git in the working directory.
    ///
    /// Equivalent to `git ls-files`.
    ///
    /// # Returns
    /// A `Vec<String>` containing the paths of tracked files relative to the repository root.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn list_tracked(&self) -> Result<Vec<String>> {
        execute_git_fn_async(&self.location, &["ls-files"], |output| {
            Ok(output.lines().map(|line| line.to_owned()).collect())
        }).await
    }

    /// Gets the URL configured for a specific remote asynchronously.
    ///
    /// Equivalent to `git config --get remote.<remote_name>.url`.
    ///
    /// # Arguments
    /// * `remote_name` - The name of the remote.
    ///
    /// # Returns
    /// The URL as a `GitUrl`.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn show_remote_uri(&self, remote_name: &Remote) -> Result<GitUrl> {
        execute_git_fn_async(
            &self.location,
            &[
                "config",
                "--get",
                &format!("remote.{}.url", remote_name),
            ],
            |output| GitUrl::from_str(output.trim()),
        ).await
    }

    /// Lists the names of all configured remotes asynchronously.
    ///
    /// Equivalent to `git remote`.
    ///
    /// # Returns
    /// A `Vec<Remote>` containing the remote names.
    ///
    /// # Errors
    /// Returns `GitError::NoRemoteRepositorySet` if no remotes are configured.
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn list_remotes(&self) -> Result<Vec<Remote>> {
        execute_git_fn_async(&self.location, &["remote"], |output| {
            let remote_names: Vec<&str> = output.lines().map(|line| line.trim()).collect();
            if remote_names.is_empty() {
                let config_check = self.cmd_out(["config", "--get-regexp", r"^remote\..*\.url"]).await;
                match config_check {
                    Ok(lines) if lines.is_empty() => Err(GitError::NoRemoteRepositorySet),
                    Ok(_) => Ok(Vec::new()),
                    Err(e) => Err(e),
                }
            } else {
                remote_names
                    .into_iter()
                    .map(Remote::from_str)
                    .collect::<Result<Vec<Remote>>>()
            }
        }).await
    }

    /// Obtains the commit hash (SHA-1) of the current `HEAD` asynchronously.
    ///
    /// Equivalent to `git rev-parse [--short] HEAD`.
    ///
    /// # Arguments
    /// * `short` - If `true`, returns the abbreviated short hash.
    ///
    /// # Returns
    /// The commit hash as a `CommitHash`.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn get_hash(&self, short: bool) -> Result<CommitHash> {
        let args: &[&str] = if short {
            &["rev-parse", "--short", "HEAD"]
        } else {
            &["rev-parse", "HEAD"]
        };
        execute_git_fn_async(
            &self.location,
            args,
            |output| CommitHash::from_str(output.trim()),
        ).await
    }

    /// Executes an arbitrary Git command and returns its standard output asynchronously.
    ///
    /// # Arguments
    /// * `args` - An iterator yielding command-line arguments for Git.
    ///
    /// # Returns
    /// A `Vec<String>` where each element is a line from the command's standard output.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn cmd_out<I, S>(&self, args: I) -> Result<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        execute_git_fn_async(&self.location, args, |output| {
            Ok(output.lines().map(|line| line.to_owned()).collect())
        }).await
    }

    /// Gets detailed information about a commit asynchronously.
    ///
    /// # Arguments
    /// * `commit_ref` - The commit reference (hash, branch name, etc.). If `None`, uses HEAD.
    ///
    /// # Returns
    /// A `Commit` struct with commit details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn get_commit(&self, commit_ref: Option<&str>) -> Result<Commit> {
        let format = "%H%n\
                     shortcommit %h%n\
                     author_name %an%n\
                     author_email %ae%n\
                     timestamp %at%n\
                     %P%n\
                     message %s";

        let format_string = format!("--format={}", format);
        let args = match commit_ref {
            Some(c) => vec!["show", "--no-patch", &format_string, c],
            None => vec!["show", "--no-patch", &format_string],
        };

        execute_git_fn_async(&self.location, args, |output| {
            Commit::from_show_format(output).ok_or_else(|| GitError::GitError {
                stdout: output.to_string(),
                stderr: "Failed to parse commit information".to_string(),
            })
        }).await
    }

    /// Gets the current status of the repository asynchronously.
    ///
    /// # Returns
    /// A `StatusResult` struct with status details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn status(&self) -> Result<StatusResult> {
        let porcelain_output = execute_git_fn_async(
            &self.location,
            &["status", "--porcelain=v2", "--branch"],
            |output| Ok(output.to_string())
        ).await?;

        let mut branch_name_str = None;
        let mut files = Vec::new();
        let mut merging = false;
        let mut rebasing = false;
        let mut cherry_picking = false;

        for line in porcelain_output.lines() {
            if line.starts_with("# branch.head ") {
                branch_name_str = Some(line.trim_start_matches("# branch.head ").to_string());
            } else if line.starts_with("# branch.oid ") { // Ignore
            } else if line.starts_with("# branch.upstream ") { // Ignore
            } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
                let parts: Vec<&str> = line.split(' ').collect();
                if parts.len() >= 2 {
                    let xy = parts[1];
                    let status_code = if xy.len() >= 2 {
                        (xy.chars().nth(0).unwrap(), xy.chars().nth(1).unwrap())
                    } else {
                        (' ', ' ')
                    };
                    let status = FileStatus::from_porcelain_code(status_code.0, status_code.1);

                    let path_part = line.split('\t').next().unwrap_or(line);
                    let path_components: Vec<&str> = path_part.split(' ').collect();

                    if let Some(path_str) = path_components.iter().rev().find(|s| !s.is_empty()) {
                        let original_path_str = if line.contains('\t') {
                            line.split('\t').nth(1)
                        } else {
                            None
                        };

                        files.push(StatusEntry {
                            path: PathBuf::from(path_str),
                            status,
                            original_path: original_path_str.map(PathBuf::from),
                        });
                    }
                }
            } else if line.starts_with("? ") {
                if line.len() > 2 {
                    let path = line[2..].to_string();
                    files.push(StatusEntry {
                        path: PathBuf::from(path),
                        status: FileStatus::Untracked,
                        original_path: None,
                    });
                }
            }
        }

        // Parse the branch name string into Option<BranchName>
        let branch = branch_name_str.and_then(|s| BranchName::from_str(&s).ok());

        // Check for special states asynchronously
        let git_dir = self.location.join(".git");
        let merge_exists = tokio::fs::try_exists(git_dir.join("MERGE_HEAD")).await.unwrap_or(false);
        let rebase_apply_exists = tokio::fs::try_exists(git_dir.join("rebase-apply")).await.unwrap_or(false);
        let rebase_merge_exists = tokio::fs::try_exists(git_dir.join("rebase-merge")).await.unwrap_or(false);
        let cherry_pick_exists = tokio::fs::try_exists(git_dir.join("CHERRY_PICK_HEAD")).await.unwrap_or(false);

        if merge_exists { merging = true; }
        if rebase_apply_exists || rebase_merge_exists { rebasing = true; }
        if cherry_pick_exists { cherry_picking = true; }

        // Determine if clean (ignoring untracked/ignored)
        let is_clean = files.iter().all(|f|
            matches!(f.status, FileStatus::Unmodified | FileStatus::Ignored)
        );

        Ok(StatusResult {
            branch,
            files,
            merging,
            rebasing,
            cherry_picking,
            is_clean,
        })
    }

    /// Lists branches with detailed information asynchronously.
    ///
    /// # Returns
    /// A vector of `Branch` structs with branch details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn list_branches_info(&self) -> Result<Vec<Branch>> {
        execute_git_fn_async(
            &self.location,
            &["branch", "--list", "-v", "--format=%(refname:short) %(objectname) %(HEAD) %(upstream:short)"],
            |output| {
                let mut branches = Vec::new();

                for line in output.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let name_str = parts[0];
                        let commit_str = parts[1];
                        let is_head = parts[2] == "*";

                        let upstream = if parts.len() >= 4 {
                            Some(parts[3].to_string())
                        } else {
                            None
                        };

                        if let Ok(name) = BranchName::from_str(name_str) {
                            if let Ok(commit_hash) = CommitHash::from_str(commit_str) {
                                branches.push(Branch {
                                    name,
                                    commit: commit_hash,
                                    is_head,
                                    upstream,
                                });
                            } else {
                                eprintln!("Warning: Could not parse commit hash '{}' for branch '{}'", commit_str, name_str);
                            }
                        } else {
                            eprintln!("Warning: Could not parse branch name '{}'", name_str);
                        }
                    }
                }
                Ok(branches)
            }
        ).await
    }

    /// Gets a list of commits in the repository history asynchronously.
    ///
    /// # Arguments
    /// * `limit` - Optional maximum number of commits to return.
    /// * `skip` - Optional number of commits to skip from the beginning.
    /// * `branch` - Optional branch name to get history for. If None, uses current branch.
    ///
    /// # Returns
    /// A vector of `Commit` structs.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub async fn get_commit_history(
        &self,
        limit: Option<usize>,
        skip: Option<usize>,
        branch: Option<&BranchName>,
    ) -> Result<Vec<Commit>> {
        let mut args = vec!["log"];

        // Format string for parsing commit info
        let format_arg = format!(
            "--format=%H%n{}%n{}%n{}%n{}%n{}%n%P%n{}",
            "shortcommit %h",
            "author_name %an",
            "author_email %ae",
            "timestamp %at",
            "message %s",
        );

        args.push(&format_arg);

        // Apply limit and skip
        if let Some(limit_val) = limit {
            args.push("--max-count");
            args.push(&limit_val.to_string());
        }

        if let Some(skip_val) = skip {
            args.push("--skip");
            args.push(&skip_val.to_string());
        }

        // If branch is specified, add it to the command
        if let Some(b) = branch {
            args.push(b.as_ref());
        }

        // Execute command
        let output = self.cmd_out(&args).await?;

        // Parse commits
        let mut commits = Vec::new();
        let mut current_lines = Vec::new();

        for line in output {
            current_lines.push(line);

            // Each commit has 7 lines in our format
            if current_lines.len() == 7 {
                if let Some(commit) = Commit::from_show_format(&current_lines.join("\n")) {
                    commits.push(commit);
                }
                current_lines.clear();
            }
        }

        Ok(commits)
    }
}

// --- Async Helper Functions ---

/// Executes a Git command asynchronously, discarding successful output.
async fn execute_git_async<I, S, P>(p: P, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    P: AsRef<Path>,
{
    execute_git_fn_async(p, args, |_| Ok(())).await
}

/// Executes a Git command and processes its stdout on success using a closure asynchronously.
/// Handles errors, including capturing stderr on failure.
async fn execute_git_fn_async<I, S, P, F, R>(p: P, args: I, process: F) -> Result<R>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    P: AsRef<Path>,
    F: FnOnce(&str) -> Result<R>,
{
    let command_result = Command::new("git")
        .current_dir(p.as_ref())
        .args(args)
        .output()
        .await;

    match command_result {
        Ok(output) => {
            if output.status.success() {
                match str::from_utf8(&output.stdout) {
                    Ok(stdout_str) => process(stdout_str),
                    Err(_) => Err(GitError::Undecodable),
                }
            } else {
                let stdout = str::from_utf8(&output.stdout)
                    .map(|s| s.trim_end().to_owned())
                    .unwrap_or_else(|_| String::from("[stdout: undecodable UTF-8]"));
                let stderr = str::from_utf8(&output.stderr)
                    .map(|s| s.trim_end().to_owned())
                    .unwrap_or_else(|_| String::from("[stderr: undecodable UTF-8]"));
                Err(GitError::GitError { stdout, stderr })
            }
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                Err(GitError::GitNotFound)
            } else {
                eprintln!("Failed to execute async git command: {}", e);
                Err(GitError::Execution)
            }
        }
    }
}