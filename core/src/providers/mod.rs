//! Providers for different source control hosting platforms.
//!
//! This module contains specialized functionality for interacting with
//! different source control providers like GitHub, GitLab, Bitbucket, etc.

pub mod github;
pub mod gitlab;
pub mod bitbucket;
pub mod generic;

use crate::error::GitError;
use crate::types::Result;

/// Trait for provider-specific operations.
pub trait ProviderOperations {
    /// Gets the provider type.
    fn provider_type(&self) -> ProviderType;

    /// Gets the API URL for the provider.
    fn api_url(&self) -> &str;

    /// Checks if a repository URL belongs to this provider.
    fn matches_url(&self, url: &str) -> bool;

    /// Creates a client for the provider with optional authentication.
    fn create_client(&self, auth_token: Option<String>) -> Result<Box<dyn ProviderClient>>;
}

/// Enum representing different provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    GitHub,
    GitLab,
    Bitbucket,
    Generic,
}

/// Trait for provider-specific client operations.
pub trait ProviderClient {
    /// Gets the provider type.
    fn provider_type(&self) -> ProviderType;

    /// Checks if the client is authenticated.
    fn is_authenticated(&self) -> bool;

    /// Gets pull/merge requests for a repository.
    fn get_pull_requests(&self, owner: &str, repo: &str) -> Result<Vec<PullRequest>>;

    /// Gets issues for a repository.
    fn get_issues(&self, owner: &str, repo: &str) -> Result<Vec<Issue>>;

    /// Gets repository metadata.
    fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo>;
}

/// Factory function to create a provider based on a repository URL.
pub fn provider_for_url(url: &str) -> Result<Box<dyn ProviderOperations>> {
    if github::GitHubProvider::new().matches_url(url) {
        Ok(Box::new(github::GitHubProvider::new()))
    } else if gitlab::GitLabProvider::new().matches_url(url) {
        Ok(Box::new(gitlab::GitLabProvider::new()))
    } else if bitbucket::BitbucketProvider::new().matches_url(url) {
        Ok(Box::new(bitbucket::BitbucketProvider::new()))
    } else {
        Ok(Box::new(generic::GenericProvider::new()))
    }
}

/// Pull/Merge request information.
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: PullRequestState,
    pub author: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub source_branch: String,
    pub target_branch: String,
    pub url: String,
}

/// Pull/Merge request state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

/// Issue information.
#[derive(Debug, Clone)]
pub struct Issue {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: IssueState,
    pub author: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub url: String,
}

/// Issue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

/// Repository information.
#[derive(Debug, Clone)]
pub struct RepositoryInfo {
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub default_branch: String,
    pub homepage: Option<String>,
    pub url: String,
}