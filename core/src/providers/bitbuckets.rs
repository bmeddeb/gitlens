//! Bitbucket provider implementation.

use crate::error::GitError;
use crate::types::Result;
use crate::providers::{
    ProviderOperations, ProviderClient, ProviderType,
    PullRequest, PullRequestState, Issue, IssueState, RepositoryInfo,
};

use std::sync::Arc;
use regex::Regex;
use once_cell::sync::Lazy;

// Regular expression to match Bitbucket URLs
static BITBUCKET_URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(?:https?://(?:www\.)?bitbucket\.org/|git@bitbucket\.org:)([^/]+)/([^/]+?)(?:\.git)?/?$")
        .expect("Invalid Bitbucket URL regex")
});

/// Bitbucket provider implementation.
pub struct BitbucketProvider {
    api_url: String,
}

impl BitbucketProvider {
    /// Creates a new Bitbucket provider.
    pub fn new() -> Self {
        BitbucketProvider {
            api_url: "https://api.bitbucket.org/2.0".to_string(),
        }
    }

    /// Extracts owner and repository name from a Bitbucket URL.
    pub fn parse_url(&self, url: &str) -> Result<(String, String)> {
        if let Some(captures) = BITBUCKET_URL_REGEX.captures(url) {
            let owner = captures.get(1).unwrap().as_str().to_string();
            let repo = captures.get(2).unwrap().as_str().to_string();
            Ok((owner, repo))
        } else {
            Err(GitError::InvalidUrl(url.to_string()))
        }
    }
}

impl ProviderOperations for BitbucketProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Bitbucket
    }

    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn matches_url(&self, url: &str) -> bool {
        BITBUCKET_URL_REGEX.is_match(url)
    }

    fn create_client(&self, auth_token: Option<String>) -> Result<Box<dyn ProviderClient>> {
        Ok(Box::new(BitbucketClient {
            provider: Arc::new(self.clone()),
            auth_token,
        }))
    }
}

impl Clone for BitbucketProvider {
    fn clone(&self) -> Self {
        BitbucketProvider {
            api_url: self.api_url.clone(),
        }
    }
}

/// Bitbucket client implementation.
pub struct BitbucketClient {
    provider: Arc<BitbucketProvider>,
    auth_token: Option<String>,
}

impl ProviderClient for BitbucketClient {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Bitbucket
    }

    fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    fn get_pull_requests(&self, owner: &str, repo: &str) -> Result<Vec<PullRequest>> {
        // To implement this properly, use an HTTP client to call the Bitbucket API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for Bitbucket API".to_string(),
            ));
        }

        // In Bitbucket, pull requests are also called "pull requests"
        // Placeholder - in a real implementation, would call Bitbucket API
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
                url: format!("https://bitbucket.org/{}/{}/pull-requests/1", owner, repo),
            },
        ];

        Ok(prs)
    }

    fn get_issues(&self, owner: &str, repo: &str) -> Result<Vec<Issue>> {
        // To implement this properly, use an HTTP client to call the Bitbucket API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for Bitbucket API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call Bitbucket API
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
                url: format!("https://bitbucket.org/{}/{}/issues/1", owner, repo),
            },
        ];

        Ok(issues)
    }

    fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo> {
        // To implement this properly, use an HTTP client to call the Bitbucket API
        // For now, return a placeholder with error if not authenticated
        if !self.is_authenticated() {
            return Err(GitError::AnalysisError(
                "Authentication required for Bitbucket API".to_string(),
            ));
        }

        // Placeholder - in a real implementation, would call Bitbucket API
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
            url: format!("https://bitbucket.org/{}/{}", owner, repo),
        };

        Ok(repo_info)
    }
}