//! GitLab provider implementation.

use crate::error::GitError;
use crate::types::Result;
use crate::providers::{
    ProviderOperations, ProviderClient, ProviderType,
    PullRequest, PullRequestState, Issue, IssueState, RepositoryInfo,
};

use std::sync::Arc;
use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;

// Regular expression to match GitLab URLs
static GITLAB_URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(?:https?://(?:www\.)?gitlab\.com/|git@gitlab\.com:)([^/]+)/([^/]+?)(?:\.git)?/?$")
        .expect("Invalid GitLab URL regex")
});

/// GitLab provider implementation.
pub struct GitLabProvider {
    api_url: String,
}

impl GitLabProvider {
    /// Creates a new GitLab provider.
    pub fn new() -> Self {
        GitLabProvider {
            api_url: "https://gitlab.com/api/v4".to_string(),
        }
    }

    /// Extracts owner and repository name from a GitLab URL.
    pub fn parse_url(&self, url: &str) -> Result<(String, String)> {
        if let Some(captures) = GITLAB_URL_REGEX.captures(url) {
            let owner = captures.get(1).unwrap().as_str().to_string();
            let repo = captures.get(2).unwrap().as_str().to_string();
            Ok((owner, repo))
        } else {
            Err(GitError::InvalidUrl(url.to_string()))
        }
    }
}

impl ProviderOperations for GitLabProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitLab
    }

    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn matches_url(&self, url: &str) -> bool {
        GITLAB_URL_REGEX.is_match(url)
    }

    fn create_client(&self, auth_token: Option<String>) -> Result<Box<dyn ProviderClient>> {
        Ok(Box::new(GitLabClient {
            provider: Arc::new(self.clone()),
            auth_token,
        }))
    }
}

impl Clone for GitLabProvider {
    fn clone(&self) -> Self {
        GitLabProvider {
            api_url: self.api_url.clone(),
        }
    }
}

/// GitLab client implementation.
pub struct GitLabClient {
    provider: Arc<GitLabProvider>,
    auth_token: Option<String>,
}

impl ProviderClient for GitLabClient {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitLab
    }

    fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    fn get_pull_requests(&self, owner: &str, repo: &str) -> Result<Vec<PullRequest>> {
        // To implement this properly, use an HTTP client to call the GitLab API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for GitLab API".to_string(),
            ));
        }

        // In GitLab, pull requests are called "merge requests"
        // Placeholder - in a real implementation, would call GitLab API
        let prs = vec![
            // Example MRs for testing
            PullRequest {
                id: 1,
                number: 1,
                title: "Example merge request".to_string(),
                description: Some("This is an example MR description".to_string()),
                state: PullRequestState::Open,
                author: "example-user".to_string(),
                created_at: 1617235200, // Example timestamp
                updated_at: 1617235200,
                source_branch: "feature-branch".to_string(),
                target_branch: "main".to_string(),
                url: format!("https://gitlab.com/{}/{}/merge_requests/1", owner, repo),
            },
        ];

        Ok(prs)
    }

    fn get_issues(&self, owner: &str, repo: &str) -> Result<Vec<Issue>> {
        // To implement this properly, use an HTTP client to call the GitLab API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for GitLab API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call GitLab API
        let issues = vec![
            // Example issues for testing
            Issue {
                id: 1,
                number: 1,
                title: "Example issue".to_string(),
                description: Some("This is an example issue description".to_string()),
                state: IssueState::Open,
                author: "example-user".to_string(),
                created_at: 1617235200, // Example timestamp
                updated_at: 1617235200,
                url: format!("https://gitlab.com/{}/{}/issues/1", owner, repo),
            },
        ];

        Ok(issues)
    }

    fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo> {
        // To implement this properly, use an HTTP client to call the GitLab API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for GitLab API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call GitLab API
        let repo_info = RepositoryInfo {
            name: repo.to_string(),
            owner: owner.to_string(),
            description: Some("Repository description".to_string()),
            stars: 0,
            forks: 0,
            created_at: 1617235200, // Example timestamp
            updated_at: 1617235200,
            default_branch: "main".to_string(),
            homepage: None,
            url: format!("https://gitlab.com/{}/{}", owner, repo),
        };

        Ok(repo_info)
    }
}