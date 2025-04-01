//! Provides the core Repository implementation for GitLens.

use crate::error::GitError;
use crate::types::{BranchName, CommitHash, GitUrl, Remote, Result};
use crate::models::*;

use std::env;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::{self, FromStr};

/// Options for cloning a repository.
#[derive(Debug, Clone)]
pub struct CloneOptions {
    /// Clone with --depth=1 for a shallow clone
    pub shallow: bool,
    /// Clone with --filter=blob:none to exclude file content
    pub no_blob_content: bool,
    /// Clone with --single-branch for a specific branch only
    pub single_branch: Option<BranchName>,
    /// Clone with --bare for a repository without a working tree
    pub bare: bool,
    /// Clone with --mirror for a complete mirror
    pub mirror: bool,
}

impl Default for CloneOptions {
    fn default() -> Self {
        CloneOptions {
            shallow: false,
            no_blob_content: false,
            single_branch: None,
            bare: false,
            mirror: false,
        }
    }
}

/// Represents a local Git repository for read-only analysis.
#[derive(Debug, Clone)]
pub struct Repository {
    pub(crate) location: PathBuf,
}

impl Repository {
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
    pub fn open<P: AsRef<Path>>(p: P) -> Result<Repository> {
        let repo = Repository {
            location: PathBuf::from(p.as_ref()),
        };

        // Verify this is actually a git repository
        match repo.cmd_out(["rev-parse", "--is-inside-work-tree"]) {
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

    /// Clones a remote Git repository into a specified local path.
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
    pub fn clone<P: AsRef<Path>>(url: GitUrl, p: P, options: Option<CloneOptions>) -> Result<Repository> {
        let p_ref = p.as_ref();
        let cwd = env::current_dir().map_err(|_| GitError::WorkingDirectoryInaccessible)?;

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

        execute_git(cwd, args)?;

        Self::open(p_ref)
    }

    /// Lists the names of all local branches.
    ///
    /// Equivalent to `git branch --format='%(refname:short)'`.
    ///
    /// # Returns
    /// A `Vec<BranchName>` containing the branch names.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn list_branches(&self) -> Result<Vec<BranchName>> {
        execute_git_fn(
            &self.location,
            &["branch", "--list", "--format=%(refname:short)"],
            |output| {
                output
                    .lines()
                    .map(|line| BranchName::from_str(line.trim()))
                    .collect::<Result<Vec<BranchName>>>()
            },
        )
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
    pub fn list_tracked(&self) -> Result<Vec<String>> {
        execute_git_fn(&self.location, &["ls-files"], |output| {
            Ok(output.lines().map(|line| line.to_owned()).collect())
        })
    }

    /// Gets the URL configured for a specific remote.
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
    pub fn show_remote_uri(&self, remote_name: &Remote) -> Result<GitUrl> {
        execute_git_fn(
            &self.location,
            &[
                "config",
                "--get",
                &format!("remote.{}.url", remote_name),
            ],
            |output| GitUrl::from_str(output.trim()),
        )
    }

    /// Lists the names of all configured remotes.
    ///
    /// Equivalent to `git remote`.
    ///
    /// # Returns
    /// A `Vec<Remote>` containing the remote names.
    ///
    /// # Errors
    /// Returns `GitError::NoRemoteRepositorySet` if no remotes are configured.
    /// Returns `GitError` (including `GitNotFound`).
    pub fn list_remotes(&self) -> Result<Vec<Remote>> {
        execute_git_fn(&self.location, &["remote"], |output| {
            let remote_names: Vec<&str> = output.lines().map(|line| line.trim()).collect();
            if remote_names.is_empty() {
                let config_check = self.cmd_out(["config", "--get-regexp", r"^remote\..*\.url"]);
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
        })
    }

    /// Obtains the commit hash (SHA-1) of the current `HEAD`.
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
    pub fn get_hash(&self, short: bool) -> Result<CommitHash> {
        let args: &[&str] = if short {
            &["rev-parse", "--short", "HEAD"]
        } else {
            &["rev-parse", "HEAD"]
        };
        execute_git_fn(
            &self.location,
            args,
            |output| CommitHash::from_str(output.trim()),
        )
    }

    /// Executes an arbitrary Git command and returns its standard output.
    ///
    /// # Arguments
    /// * `args` - An iterator yielding command-line arguments for Git.
    ///
    /// # Returns
    /// A `Vec<String>` where each element is a line from the command's standard output.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn cmd_out<I, S>(&self, args: I) -> Result<Vec<String>>
    where
        I: IntoIterator<Item=S>,
        S: AsRef<OsStr>,
    {
        execute_git_fn(&self.location, args, |output| {
            Ok(output.lines().map(|line| line.to_owned()).collect())
        })
    }

    /// Gets detailed information about a commit.
    ///
    /// # Arguments
    /// * `commit_ref` - The commit reference (hash, branch name, etc.). If `None`, uses HEAD.
    ///
    /// # Returns
    /// A `Commit` struct with commit details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn get_commit(&self, commit_ref: Option<&str>) -> Result<Commit> {
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

        execute_git_fn(&self.location, args, |output| {
            Commit::from_show_format(output).ok_or_else(|| GitError::GitError {
                stdout: output.to_string(),
                stderr: "Failed to parse commit information".to_string(),
            })
        })
    }

    /// Gets the current status of the repository.
    ///
    /// # Returns
    /// A `StatusResult` struct with status details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn status(&self) -> Result<StatusResult> {
        let porcelain_output = execute_git_fn(
            &self.location,
            &["status", "--porcelain=v2", "--branch"],
            |output| Ok(output.to_string())
        )?;

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

        // Check for special states
        let git_dir = self.location.join(".git");
        if std::path::Path::new(&git_dir.join("MERGE_HEAD")).exists() { merging = true; }
        if std::path::Path::new(&git_dir.join("rebase-apply")).exists() || std::path::Path::new(&git_dir.join("rebase-merge")).exists() { rebasing = true; }
        if std::path::Path::new(&git_dir.join("CHERRY_PICK_HEAD")).exists() { cherry_picking = true; }

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

    /// Lists branches with detailed information.
    ///
    /// # Returns
    /// A vector of `Branch` structs with branch details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn list_branches_info(&self) -> Result<Vec<Branch>> {
        execute_git_fn(
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
        )
    }

    /// Gets a list of commits in the repository history.
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
    pub fn get_commit_history(
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
        let output = self.cmd_out(&args)?;

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

    /// Gets blame information for a file.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root.
    ///
    /// # Returns
    /// A vector of `BlameLine` structs with line-by-line blame information.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn blame<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<BlameLine>> {
        let output = execute_git_fn(
            &self.location,
            &[
                "blame",
                "--porcelain",
                file_path.as_ref().to_str().ok_or_else(|| {
                    GitError::PathEncodingError(PathBuf::from(file_path.as_ref()))
                })?,
            ],
            |output| Ok(output.to_string()),
        )?;

        let mut blame_lines = Vec::new();
        let mut current_hash: Option<CommitHash> = None;
        let mut current_author = String::new();
        let mut current_timestamp = 0;
        let mut current_line_no = 0;
        let mut final_line_no = 0;

        for line in output.lines() {
            if line.starts_with('\t') {
                // Content line
                if let (Some(hash), line_no, fin_line_no) =
                    (current_hash.clone(), current_line_no, final_line_no) {
                    blame_lines.push(BlameLine {
                        hash,
                        author: current_author.clone(),
                        original_line: line_no,
                        final_line: fin_line_no,
                        timestamp: current_timestamp,
                        content: line[1..].to_string(),
                    });
                }
            } else if line.contains(' ') {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    match parts[0] {
                        "author" => current_author = parts[1].to_string(),
                        "author-time" => {
                            current_timestamp = parts[1].parse().unwrap_or(0);
                        }
                        "original-line" => {
                            current_line_no = parts[1].parse().unwrap_or(0);
                        }
                        "final-line" => {
                            final_line_no = parts[1].parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            } else if line.len() >= 40 {
                // Commit hash
                if let Ok(hash) = CommitHash::from_str(&line[0..40]) {
                    current_hash = Some(hash);
                }
            }
        }

        Ok(blame_lines)
    }

    /// Gets information about tags in the repository.
    ///
    /// # Returns
    /// A vector of `TagInfo` structs with tag details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn list_tags(&self) -> Result<Vec<TagInfo>> {
        let output = self.cmd_out([
            "tag",
            "--list",
            "--format=%(refname:short) %(objecttype) %(objectname) %(*objectname)",
        ])?;

        let mut tags = Vec::new();

        for line in output {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name_str = parts[0];
                let obj_type = parts[1];
                let obj_name = parts[2];

                let target_hash_str = if obj_type == "tag" && parts.len() >= 4 {
                    // For annotated tags, use the target commit hash
                    parts[3]
                } else {
                    // For lightweight tags, use the direct hash
                    obj_name
                };

                if let (Ok(name), Ok(target)) = (
                    crate::types::Tag::from_str(name_str),
                    CommitHash::from_str(target_hash_str),
                ) {
                    // Get message for annotated tags
                    let message = if obj_type == "tag" {
                        match self.cmd_out(["tag", "-n1", "--format=%(contents)", name_str]) {
                            Ok(msg) if !msg.is_empty() => Some(msg.join("\n")),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    tags.push(TagInfo {
                        name,
                        target,
                        annotated: obj_type == "tag",
                        message,
                    });
                }
            }
        }

        Ok(tags)
    }

    /// Finds the common ancestor (merge base) of two commits or branches.
    ///
    /// # Arguments
    /// * `commit1` - First commit reference
    /// * `commit2` - Second commit reference
    ///
    /// # Returns
    /// The common ancestor commit hash.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn find_merge_base(&self, commit1: &str, commit2: &str) -> Result<CommitHash> {
        execute_git_fn(
            &self.location,
            &["merge-base", commit1, commit2],
            |output| CommitHash::from_str(output.trim()),
        )
    }

    /// Calculates the divergence metrics between two branches.
    ///
    /// # Arguments
    /// * `source` - Source branch
    /// * `target` - Target branch
    ///
    /// # Returns
    /// A `BranchDivergence` struct with divergence metrics.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn calculate_branch_divergence(
        &self,
        source: &BranchName,
        target: &BranchName,
    ) -> Result<BranchDivergence> {
        // Find common ancestor
        let merge_base = self.find_merge_base(source.as_ref(), target.as_ref())?;

        // Count commits ahead (source not in target)
        let ahead_output = self.cmd_out([
            "rev-list",
            "--count",
            &format!("{}..{}", target.as_ref(), source.as_ref()),
        ])?;
        let ahead_count = ahead_output
            .first()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        // Count commits behind (target not in source)
        let behind_output = self.cmd_out([
            "rev-list",
            "--count",
            &format!("{}..{}", source.as_ref(), target.as_ref()),
        ])?;
        let behind_count = behind_output
            .first()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        // Count differing files
        let diff_output = self.cmd_out([
            "diff",
            "--name-only",
            source.as_ref(),
            target.as_ref(),
        ])?;
        let differing_files = diff_output.len();

        Ok(BranchDivergence {
            source: source.clone(),
            target: target.clone(),
            merge_base,
            ahead_count,
            behind_count,
            differing_files,
        })
    }

    /// Gets information about remotes in the repository.
    ///
    /// # Returns
    /// A vector of `RemoteInfo` structs with remote details.
    ///
    /// # Errors
    /// Returns `GitError` (including `GitNotFound`).
    pub fn get_remotes_info(&self) -> Result<Vec<RemoteInfo>> {
        let remotes = self.list_remotes()?;
        let mut remote_infos = Vec::with_capacity(remotes.len());

        for remote in remotes {
            let url = self.show_remote_uri(&remote)?;

            // Get fetch refspec
            let fetch_output = self.cmd_out([
                "config",
                "--get",
                &format!("remote.{}.fetch", remote),
            ]);

            let fetch = match fetch_output {
                Ok(lines) if !lines.is_empty() => Some(lines.join("\n")),
                _ => None,
            };

            remote_infos.push(RemoteInfo {
                name: remote,
                url,
                fetch,
            });
        }

        Ok(remote_infos)
    }
}