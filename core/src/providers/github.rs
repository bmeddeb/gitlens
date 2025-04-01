//! GitHub provider implementation.

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

// Regular expression to match GitHub URLs
static GITHUB_URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(?:https?://(?:www\.)?github\.com/|git@github\.com:)([^/]+)/([^/]+?)(?:\.git)?/?$")
        .expect("Invalid GitHub URL regex")
});

/// GitHub provider implementation.
pub struct GitHubProvider {
    api_url: String,
}

impl GitHubProvider {
    /// Creates a new GitHub provider.
    pub fn new() -> Self {
        GitHubProvider {
            api_url: "https://api.github.com".to_string(),
        }
    }

    /// Extracts owner and repository name from a GitHub URL.
    pub fn parse_url(&self, url: &str) -> Result<(String, String)> {
        if let Some(captures) = GITHUB_URL_REGEX.captures(url) {
            let owner = captures.get(1).unwrap().as_str().to_string();
            let repo = captures.get(2).unwrap().as_str().to_string();
            Ok((owner, repo))
        } else {
            Err(GitError::InvalidUrl(url.to_string()))
        }
    }
}

impl ProviderOperations for GitHubProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitHub
    }

    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn matches_url(&self, url: &str) -> bool {
        GITHUB_URL_REGEX.is_match(url)
    }

    fn create_client(&self, auth_token: Option<String>) -> Result<Box<dyn ProviderClient>> {
        Ok(Box::new(GitHubClient {
            provider: Arc::new(self.clone()),
            auth_token,
        }))
    }
}

impl Clone for GitHubProvider {
    fn clone(&self) -> Self {
        GitHubProvider {
            api_url: self.api_url.clone(),
        }
    }
}

/// GitHub client implementation.
pub struct GitHubClient {
    provider: Arc<GitHubProvider>,
    auth_token: Option<String>,
}

impl ProviderClient for GitHubClient {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitHub
    }

    fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    fn get_pull_requests(&self, owner: &str, repo: &str) -> Result<Vec<PullRequest>> {
        // To implement this properly, use an HTTP client to call the GitHub API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for GitHub API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call GitHub API
        let prs = vec![
            // Example PRs for testing
            PullRequest {
                id: 1,
                number: 1,
                title: "Example pull request".to_string(),
                description: Some("This is an example PR description".to_string()),
                state: PullRequestState::Open,
                author: "example-user".to_string(),
                created_at: 1617235200, // Example timestamp
                updated_at: 1617235200,
                source_branch: "feature-branch".to_string(),
                target_branch: "main".to_string(),
                url: format!("https://github.com/{}/{}/pull/1", owner, repo),
            },
        ];

        Ok(prs)
    }

    fn get_issues(&self, owner: &str, repo: &str) -> Result<Vec<Issue>> {
        // To implement this properly, use an HTTP client to call the GitHub API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for GitHub API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call GitHub API
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
                url: format!("https://github.com/{}/{}/issues/1", owner, repo),
            },
        ];

        Ok(issues)
    }

    fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo> {
        // To implement this properly, use an HTTP client to call the GitHub API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for GitHub API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call GitHub API
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
            url: format!("https://github.com/{}/{}", owner, repo),
        };

        Ok(repo_info)
    }
}