use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use ureq;
use url::Url;
/// A GitHub pull request minimal representation
#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub number: i64,
    #[serde(default)]
    pub merged_at: Option<String>,
    /// URL of the pull request on GitHub
    pub html_url: String,
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

    /// List pull requests associated with a specific commit SHA
    pub fn get_pull_requests_for_commit(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> Result<Vec<PullRequest>> {
        let mut url = Url::parse(&self.api_base)?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("Failed to set URL path segments"))?;
            segments.extend(&["repos", owner, repo, "commits", commit_sha, "pulls"]);
        }
        let resp = ureq::get(url.as_str())
            .set("Authorization", &format!("Bearer {}", &self.token))
            .set("User-Agent", "wkfl")
            .set("Accept", "application/vnd.github+json")
            .call()
            .with_context(|| format!("Failed to query GitHub API for commit '{commit_sha}'"))?;
        let prs: Vec<PullRequest> = resp
            .into_json()
            .with_context(|| "Failed to parse GitHub PRs response as JSON")?;
        Ok(prs)
    }
}
