//! Defines the error types used throughout the GitLens library.
use thiserror::Error;

/// Represents errors that can occur during Git operations.
#[derive(Debug, Error)]
pub enum GitError {
    /// Failed to access the current working directory, e.g., due to permissions.
    #[error("Unable to access current working directory")]
    WorkingDirectoryInaccessible,

    /// Failed to execute the external 'git' process, e.g., 'git' not found in PATH.
    #[error("Unable to execute git process")]
    Execution,

    /// The output (stdout or stderr) from the 'git' process was not valid UTF-8.
    #[error("Unable to decode error from git executable")]
    Undecodable,

    /// The provided string is not a valid Git URL according to the library's criteria.
    #[error("git URL is invalid: {0}")]
    InvalidUrl(String),

    /// The provided string is not a valid Git reference name (e.g., branch name).
    #[error("Ref name is invalid: {0}")]
    InvalidRefName(String),

    /// The 'git' command executed successfully but reported an error.
    /// Contains the captured stdout and stderr from the failed command.
    #[error("git failed with the following stdout: {stdout} stderr: {stderr}")]
    GitError { stdout: String, stderr: String },

    /// Attempted an operation requiring a remote (e.g., list remotes) but none were configured.
    #[error("No Git remote repository is available")]
    NoRemoteRepositorySet,

    /// The provided path could not be converted to a UTF-8 string, which was required
    /// for constructing the git command arguments in this specific context.
    #[error("Path contains non-UTF8 characters and cannot be used as a string argument: {0:?}")]
    PathEncodingError(std::path::PathBuf),

    /// The provided commit hash is invalid.
    #[error("Commit hash is invalid: {0}")]
    InvalidCommitHash(String),

    /// The provided remote name is invalid.
    #[error("Remote name is invalid: {0}")]
    InvalidRemoteName(String),

    /// The provided stash reference is invalid.
    #[error("Stash reference is invalid: {0}")]
    InvalidStashRef(String),

    /// The 'git' executable was not found in the system's PATH.
    #[error("'git' command not found. Please ensure Git is installed and that its executable is included in your system's PATH environment variable.")]
    GitNotFound,

    /// Repository at the specified path is not a valid Git repository.
    #[error("Not a valid Git repository at {0}")]
    NotAGitRepository(String),

    /// Invalid analysis parameters provided.
    #[error("Invalid analysis parameters: {0}")]
    InvalidAnalysisParameters(String),

    /// Error that occurs during repository analysis operations.
    #[error("Analysis operation failed: {0}")]
    AnalysisError(String),

    /// Error accessing or reading from the file system.
    #[error("File system error: {0}")]
    FileSystemError(String),

    /// Write operation attempted on a read-only repository.
    #[error("Write operation not permitted on read-only repository: {0}")]
    WriteOperationNotPermitted(String),
}