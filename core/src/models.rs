//! Provides structured types representing Git data.

use crate::types::{BranchName, CommitHash, GitUrl, Remote, Stash, Tag};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

/// Represents a Git commit.
#[derive(Debug, Clone)]
pub struct Commit {
    /// The commit hash.
    pub hash: CommitHash,
    /// The abbreviated hash.
    pub short_hash: CommitHash,
    /// The commit author's name.
    pub author_name: String,
    /// The commit author's email.
    pub author_email: String,
    /// The commit timestamp (seconds since Unix epoch).
    pub timestamp: u64,
    /// The commit message.
    pub message: String,
    /// Parent commit hashes.
    pub parents: Vec<CommitHash>,
}

impl Commit {
    /// Parses a commit from the output of `git show --format=...`.
    pub(crate) fn from_show_format(output: &str) -> Option<Commit> {
        let mut hash_str = None;
        let mut short_hash_str = None;
        let mut author_name = String::new();
        let mut author_email = String::new();
        let mut timestamp = 0;
        let mut message = String::new();
        let mut parent_hashes_str = String::new();

        for line in output.lines() {
            if hash_str.is_none() && !line.is_empty() {
                hash_str = Some(line.to_string());
            } else if line.starts_with("shortcommit ") {
                short_hash_str = Some(line.trim_start_matches("shortcommit ").to_string());
            } else if line.starts_with("author_name ") {
                author_name = line.trim_start_matches("author_name ").to_string();
            } else if line.starts_with("author_email ") {
                author_email = line.trim_start_matches("author_email ").to_string();
            } else if line.starts_with("timestamp ") {
                timestamp = line.trim_start_matches("timestamp ").parse::<u64>().ok()?;
            } else if !line.starts_with("message ") && parent_hashes_str.is_empty() && hash_str.is_some() && short_hash_str.is_some() {
                parent_hashes_str = line.to_string();
            } else if line.starts_with("message ") {
                message = line.trim_start_matches("message ").to_string();
            }
        }

        let hash = CommitHash::from_str(&hash_str?).ok()?;
        let short_hash = CommitHash::from_str(&short_hash_str?).ok()?;

        let parents = parent_hashes_str
            .split_whitespace()
            .map(CommitHash::from_str)
            .collect::<std::result::Result<Vec<_>, _>>()
            .ok()?;

        Some(Commit {
            hash,
            short_hash,
            author_name,
            author_email,
            timestamp,
            message,
            parents,
        })
    }

    /// Returns the commit date as a SystemTime.
    pub fn date(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp)
    }
}

/// Represents a file status from `git status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    DeletedStaged,
    Renamed,
    Copied,
    UpdatedButUnmerged,
    Untracked,
    Ignored,
}

impl FileStatus {
    /// Parses a file status from a git status porcelain v1/v2 XY code.
    pub(crate) fn from_porcelain_code(index: char, worktree: char) -> FileStatus {
        match (index, worktree) {
            (' ', 'M') => FileStatus::Modified,
            ('M', _)   => FileStatus::Added,
            ('A', _)   => FileStatus::Added,
            ('D', _)   => FileStatus::DeletedStaged,
            ('R', _)   => FileStatus::Renamed,
            ('C', _)   => FileStatus::Copied,
            ('U', _)   => FileStatus::UpdatedButUnmerged,
            (_,   'D') => FileStatus::Deleted,
            ('?', '?') => FileStatus::Untracked,
            ('!', '!') => FileStatus::Ignored,
            _          => FileStatus::Unmodified,
        }
    }
}

/// Represents a file in the repository with its status.
#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: PathBuf,
    pub status: FileStatus,
    pub original_path: Option<PathBuf>,
}

/// Represents a Git tag.
#[derive(Debug, Clone)]
pub struct TagInfo {
    /// The name of the tag.
    pub name: Tag,
    /// The commit hash the tag points to.
    pub target: CommitHash,
    /// Whether the tag is annotated.
    pub annotated: bool,
    /// For annotated tags, the tag message.
    pub message: Option<String>,
}

/// Represents a Git remote.
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    /// The name of the remote.
    pub name: Remote,
    /// The URL of the remote.
    pub url: GitUrl,
    /// The fetch refspec.
    pub fetch: Option<String>,
}

/// Represents a Git branch.
#[derive(Debug, Clone)]
pub struct Branch {
    /// The name of the branch.
    pub name: BranchName,
    /// The commit hash the branch points to.
    pub commit: CommitHash,
    /// Whether the branch is the current HEAD.
    pub is_head: bool,
    /// The upstream branch ref string (e.g., "origin/main").
    pub upstream: Option<String>,
}

/// Represents the result of a `git status` command.
#[derive(Debug, Clone)]
pub struct StatusResult {
    /// The current branch name, if on a branch.
    pub branch: Option<BranchName>,
    /// The files in the repository with their status.
    pub files: Vec<StatusEntry>,
    /// Whether the repository is in a merge state.
    pub merging: bool,
    /// Whether the repository is in a rebase state.
    pub rebasing: bool,
    /// Whether the repository is in a cherry-pick state.
    pub cherry_picking: bool,
    /// Whether the working directory is clean (no changes, excluding untracked/ignored).
    pub is_clean: bool,
}

/// Represents a line of blame information.
#[derive(Debug, Clone)]
pub struct BlameLine {
    /// The commit hash.
    pub hash: CommitHash,
    /// The author's name.
    pub author: String,
    /// The line number in the original file.
    pub original_line: usize,
    /// The line number in the final file.
    pub final_line: usize,
    /// The timestamp (seconds since Unix epoch).
    pub timestamp: u64,
    /// The line content.
    pub content: String,
}

/// Represents the result of a `git diff` command.
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub files: Vec<DiffFile>,
}

/// Represents a file in a diff.
#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub hunks: Vec<DiffHunk>,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub is_binary: bool,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
}

/// Represents a hunk in a diff.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub lines: Vec<DiffLine>,
}

/// Represents a line in a diff hunk.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
}

/// Represents the type of a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
}

/// Represents a stash entry.
#[derive(Debug, Clone)]
pub struct StashEntry {
    /// The stash reference.
    pub reference: Stash,
    /// The branch the stash was created from.
    pub branch: Option<String>,
    /// The commit message of the stash.
    pub message: String,
}

/// Represents a worktree.
#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    /// The commit hash the worktree is at.
    pub head: CommitHash,
    /// The branch the worktree is on, if any.
    pub branch: Option<String>,
    pub is_main: bool,
    pub is_bare: bool,
    pub is_prunable: bool,
}

/// Represents a Git reference (branch, tag, etc.).
#[derive(Debug, Clone)]
pub struct Reference {
    /// The name of the reference.
    pub name: String,
    /// The type of the reference.
    pub ref_type: ReferenceType,
    /// The commit hash the reference points to.
    pub target: CommitHash,
}

/// Represents the type of a Git reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    LocalBranch,
    RemoteBranch,
    Tag,
    Note,
    Other,
}

/// Represents author contribution statistics.
#[derive(Debug, Clone)]
pub struct AuthorStats {
    /// Number of commits by this author.
    pub commits: usize,
    /// Number of lines added.
    pub added_lines: usize,
    /// Number of lines removed.
    pub removed_lines: usize,
    /// Number of files changed.
    pub files_changed: usize,
    /// Timestamp of first commit.
    pub first_commit: u64,
    /// Timestamp of last commit.
    pub last_commit: u64,
}

/// Represents contribution statistics for a repository.
#[derive(Debug, Clone)]
pub struct ContributionStats {
    /// Statistics by author.
    pub by_author: HashMap<String, AuthorStats>,
    /// Total number of commits analyzed.
    pub total_commits: usize,
    /// Total unique authors.
    pub total_authors: usize,
    /// Total lines added.
    pub total_added: usize,
    /// Total lines removed.
    pub total_removed: usize,
    /// Total files changed.
    pub total_files_changed: usize,
}

/// Represents a file's change frequency metrics.
#[derive(Debug, Clone)]
pub struct FileChangeFrequency {
    /// Path to the file.
    pub path: PathBuf,
    /// Number of times the file has been modified.
    pub change_count: usize,
    /// Last time the file was modified (timestamp).
    pub last_modified: u64,
    /// Authors who have modified this file.
    pub authors: Vec<String>,
    /// Who has made the most changes (primary owner).
    pub primary_owner: Option<String>,
}

/// Represents code ownership for a repository.
#[derive(Debug, Clone)]
pub struct CodeOwnership {
    /// Ownership by file.
    pub files: HashMap<PathBuf, HashMap<String, usize>>,
    /// Ownership by directory.
    pub directories: HashMap<PathBuf, HashMap<String, usize>>,
}

/// Represents metrics for branch divergence.
#[derive(Debug, Clone)]
pub struct BranchDivergence {
    /// Source branch.
    pub source: BranchName,
    /// Target branch.
    pub target: BranchName,
    /// Common ancestor commit.
    pub merge_base: CommitHash,
    /// Commits in source not in target.
    pub ahead_count: usize,
    /// Commits in target not in source.
    pub behind_count: usize,
    /// Files that differ between branches.
    pub differing_files: usize,
}