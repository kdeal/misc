use crate::config::resolve_secret;
use crate::config::Config;
use crate::git::host_from_remote_url;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use url::Url;
/// A GitHub pull request minimal representation
#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    #[serde(default)]
    pub merged_at: Option<String>,
    /// URL of the pull request on GitHub
    pub html_url: String,
}

/// A GitHub user
#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
    #[serde(rename = "type")]
    pub user_type: String,
}

/// A GitHub issue/PR comment (timeline comment)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

/// A GitHub pull request review comment (diff comment)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReviewComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
    pub path: String,
    pub line: Option<u32>,
    pub original_line: Option<u32>,
    /// The first line of the range in the original diff for a multi-line comment
    pub original_start_line: Option<u32>,
    pub diff_hunk: String,
    #[serde(default)]
    pub in_reply_to_id: Option<u64>,
    /// The side of the diff (LEFT for deletions, RIGHT for additions)
    pub side: String,
    /// The first line of the range for a multi-line comment
    pub start_line: Option<u32>,
    /// The side of the first line of the range for a multi-line comment  
    pub start_side: Option<String>,
    /// The SHA of the commit needing a comment
    pub commit_id: String,
    /// The SHA of the original commit (for multi-commit PRs)
    pub original_commit_id: String,
}

/// A GitHub pull request review
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Review {
    pub id: u64,
    pub state: String, // "APPROVED", "CHANGES_REQUESTED", "COMMENTED", "DISMISSED"
}

/// Container for all PR comment types
#[derive(Debug)]
pub struct PrComments {
    pub issue_comments: Vec<IssueComment>,
    pub review_comments: Vec<ReviewComment>,
}

/// Client for interacting with the GitHub API
pub struct GitHubClient {
    api_base: String,
    token: String,
}

impl GitHubClient {
    /// Create a new GitHub client
    pub fn new(host: String, token: String) -> Self {
        let api_base = if host == "github.com" {
            "https://api.github.com".to_string()
        } else {
            format!("https://{host}/api/v3")
        };
        GitHubClient { api_base, token }
    }

    /// Make a GET request to the GitHub API
    fn api_get(&self, path_segments: &[&str]) -> Result<ureq::Response> {
        let mut url = Url::parse(&self.api_base)?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("Failed to set URL path segments"))?;
            segments.extend(path_segments);
        }
        let resp = ureq::get(url.as_str())
            .set("Authorization", &format!("Bearer {}", &self.token))
            .set("User-Agent", "wkfl")
            .set("Accept", "application/vnd.github+json")
            .call()
            .with_context(|| {
                format!(
                    "Failed to query GitHub API at path: {}",
                    path_segments.join("/")
                )
            })?;
        Ok(resp)
    }

    /// List pull requests associated with a specific commit SHA
    pub fn get_pull_requests_for_commit(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> Result<Vec<PullRequest>> {
        let resp = self
            .api_get(&["repos", owner, repo, "commits", commit_sha, "pulls"])
            .with_context(|| format!("Failed to query GitHub API for commit '{commit_sha}'"))?;
        let prs: Vec<PullRequest> = resp
            .into_json()
            .with_context(|| "Failed to parse GitHub PRs response as JSON")?;
        Ok(prs)
    }

    /// Get all comments for a pull request
    pub fn get_pr_comments(&self, owner: &str, repo: &str, pr_number: u64) -> Result<PrComments> {
        let issue_comments = self.get_issue_comments(owner, repo, pr_number)?;
        let review_comments = self.get_review_comments(owner, repo, pr_number)?;

        Ok(PrComments {
            issue_comments,
            review_comments,
        })
    }

    /// Get issue/timeline comments for a PR
    fn get_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<IssueComment>> {
        let resp = self
            .api_get(&[
                "repos",
                owner,
                repo,
                "issues",
                &pr_number.to_string(),
                "comments",
            ])
            .with_context(|| format!("Failed to query GitHub API for PR #{pr_number} comments"))?;

        let comments: Vec<IssueComment> = resp
            .into_json()
            .with_context(|| "Failed to parse GitHub issue comments response as JSON")?;
        Ok(comments)
    }

    /// Get review/diff comments for a PR
    fn get_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<ReviewComment>> {
        let resp = self
            .api_get(&[
                "repos",
                owner,
                repo,
                "pulls",
                &pr_number.to_string(),
                "comments",
            ])
            .with_context(|| {
                format!("Failed to query GitHub API for PR #{pr_number} review comments")
            })?;

        let comments: Vec<ReviewComment> = resp
            .into_json()
            .with_context(|| "Failed to parse GitHub review comments response as JSON")?;
        Ok(comments)
    }
}

pub fn create_github_client(remote_url: &str, config: &Config) -> anyhow::Result<GitHubClient> {
    let host = host_from_remote_url(remote_url)?;
    let github_token_raw = config.github_tokens.get(&host).ok_or_else(|| {
        anyhow!(
            "GitHub token not configured for host '{}'. Add it to your config file.",
            host
        )
    })?;
    let github_token = resolve_secret(github_token_raw)
        .with_context(|| format!("Failed to resolve GitHub token for host '{host}'"))?;

    Ok(GitHubClient::new(host, github_token))
}

pub fn is_bot_user(user_login: &str, user_type: &str) -> bool {
    user_type == "Bot" || user_login.starts_with("service") || user_login.ends_with("[bot]")
}
