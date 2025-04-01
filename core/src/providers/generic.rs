//! Generic provider implementation for other Git hosting services.

use crate::error::GitError;
use crate::types::Result;
use crate::providers::{
    ProviderOperations, ProviderClient, ProviderType,
    PullRequest, PullRequestState, Issue, IssueState, RepositoryInfo,
};

use std::sync::Arc;

/// Generic provider implementation.
pub struct GenericProvider {
    api_url: String,
}

impl GenericProvider {
    /// Creates a new generic provider.
    pub fn new() -> Self {
        GenericProvider {
            api_url: "".to_string(), // No API URL for generic provider
        }
    }
}

impl ProviderOperations for GenericProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Generic
    }

    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn matches_url(&self, _url: &str) -> bool {
        // Generic provider is a fallback, so it matches any URL
        true
    }

    fn create_client(&self, auth_token: Option<String>) -> Result<Box<dyn ProviderClient>> {
        Ok(Box::new(GenericClient {
            provider: Arc::new(self.clone()),
            auth_token,
        }))
    }
}

impl Clone for GenericProvider {
    fn clone(&self) -> Self {
        GenericProvider {
            api_url: self.api_url.clone(),
        }
    }
}

/// Generic client implementation.
pub struct GenericClient {
    provider: Arc<GenericProvider>,
    auth_token: Option<String>,
}

impl ProviderClient for GenericClient {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Generic
    }

    fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    fn get_pull_requests(&self, _owner: &str, _repo: &str) -> Result<Vec<PullRequest>> {
        // Generic provider doesn't support pull requests
        Err(GitError::AnalysisError(
            "Pull requests are not supported for generic Git providers".to_string(),
        ))
    }

    fn get_issues(&self, _owner: &str, _repo: &str) -> Result<Vec<Issue>> {
        // Generic provider doesn't support issues
        Err(GitError::AnalysisError(
            "Issues are not supported for generic Git providers".to_string(),
        ))
    }

    fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo> {
        // Generic provider has limited repository info
        let repo_info = RepositoryInfo {
            name: repo.to_string(),
            owner: owner.to_string(),
            description: None,
            stars: 0,
            forks: 0,
            created_at: 0, // Unknown
            updated_at: 0, // Unknown
            default_branch: "main".to_string(), // Assume default
            homepage: None,
            url: "".to_string(), // Unknown URL
        };

        Ok(repo_info)
    }
}