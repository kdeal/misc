use crate::config::resolve_secret;
use crate::config::Config;
use crate::git::host_from_remote_url;
use crate::gql_queries;
use crate::gql_queries::prs_to_review::{
    GraphQLPrToReviewNode, GraphQLPrsToReviewData, GraphQLPrsToReviewVariables,
    GraphQLSearchConnection,
};
use crate::gql_queries::review_comments::{
    GraphQLReviewCommentConnection, GraphQLReviewCommentNode, GraphQLReviewCommentsData,
    GraphQLReviewCommentsVariables,
};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;
/// A GitHub pull request minimal representation
#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub number: u64,
    #[serde(default)]
    pub merged_at: Option<String>,
    /// URL of the pull request on GitHub
    pub html_url: String,
}

/// A GitHub user
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub login: String,
    #[serde(rename = "type")]
    pub user_type: String,
}

/// A GitHub issue/PR comment (timeline comment)
#[derive(Debug, Deserialize, Serialize)]
pub struct IssueComment {
    pub body: String,
    pub user: User,
    pub created_at: String,
}

/// A GitHub pull request review comment (diff comment)
#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewComment {
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub path: String,
    pub original_line: Option<u32>,
    /// The first line of the range in the original diff for a multi-line comment
    pub original_start_line: Option<u32>,
    pub diff_hunk: String,
    /// The side of the diff (LEFT for deletions, RIGHT for additions)
    pub side: String,
    /// The side of the first line of the range for a multi-line comment
    pub start_side: Option<String>,
    /// Indicates whether the review thread containing this comment is resolved
    pub is_resolved: Option<bool>,
}

/// Container for all PR comment types
#[derive(Debug, Serialize)]
pub struct PrComments {
    pub issue_comments: Vec<IssueComment>,
    pub review_comments: Vec<ReviewComment>,
}

/// A pull request that is waiting for the authenticated user's review.
#[derive(Debug, Serialize)]
pub struct PrToReview {
    pub repo: String,
    pub repo_url: String,
    pub number: u64,
    pub title: String,
    pub author: User,
    pub state: String,
    pub is_draft: bool,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A GitHub notification subject.
#[derive(Debug, Deserialize, Serialize)]
pub struct NotificationSubject {
    pub title: String,
    pub url: Option<String>,
    pub latest_comment_url: Option<String>,
    #[serde(rename = "type")]
    pub subject_type: String,
}

/// A GitHub notification repository.
#[derive(Debug, Deserialize, Serialize)]
pub struct NotificationRepository {
    pub full_name: String,
    pub html_url: String,
}

/// A GitHub notification for the authenticated user.
#[derive(Debug, Deserialize, Serialize)]
pub struct Notification {
    pub id: String,
    pub unread: bool,
    pub reason: String,
    pub updated_at: String,
    pub last_read_at: Option<String>,
    pub subject: NotificationSubject,
    pub repository: NotificationRepository,
    pub url: String,
    pub subscription_url: String,
}

/// Client for interacting with the GitHub API
pub struct GitHubClient {
    api_base: String,
    graphql_base: String,
    token: String,
}

impl GitHubClient {
    /// Create a new GitHub client
    pub fn new(host: String, token: String) -> Self {
        let (api_base, graphql_base) = if host == "github.com" {
            (
                "https://api.github.com".to_string(),
                "https://api.github.com/graphql".to_string(),
            )
        } else {
            (
                format!("https://{host}/api/v3"),
                format!("https://{host}/api/graphql"),
            )
        };
        GitHubClient {
            api_base,
            graphql_base,
            token,
        }
    }

    fn set_headers(&self, request: ureq::Request) -> ureq::Request {
        request
            .set("Authorization", &format!("Bearer {}", &self.token))
            .set("User-Agent", "wkfl")
            .set("Accept", "application/vnd.github+json")
    }

    /// Make a GET request to the GitHub API
    fn api_get(&self, path_segments: &[&str]) -> Result<ureq::Response> {
        self.api_get_with_query(path_segments, &[])
    }

    fn api_url(&self, path_segments: &[&str], query_pairs: &[(&str, String)]) -> Result<Url> {
        let mut url = Url::parse(&self.api_base)?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("Failed to set URL path segments"))?;
            segments.extend(path_segments);
        }
        if !query_pairs.is_empty() {
            let mut query = url.query_pairs_mut();
            for (key, value) in query_pairs {
                query.append_pair(key, value);
            }
        }

        Ok(url)
    }

    /// Make a GET request to the GitHub API with query parameters.
    fn api_get_with_query(
        &self,
        path_segments: &[&str],
        query_pairs: &[(&str, String)],
    ) -> Result<ureq::Response> {
        let url = self.api_url(path_segments, query_pairs)?;
        let resp = self
            .set_headers(ureq::get(url.as_str()))
            .call()
            .with_context(|| {
                format!(
                    "Failed to query GitHub API at path: {}",
                    path_segments.join("/")
                )
            })?;
        Ok(resp)
    }

    /// Make a PATCH request to the GitHub API without a request body.
    fn api_patch_empty(&self, path_segments: &[&str]) -> Result<ureq::Response> {
        let url = self.api_url(path_segments, &[])?;
        let resp = self
            .set_headers(ureq::patch(url.as_str()))
            .call()
            .with_context(|| {
                format!(
                    "Failed to patch GitHub API at path: {}",
                    path_segments.join("/")
                )
            })?;
        Ok(resp)
    }

    /// Make a DELETE request to the GitHub API.
    fn api_delete(&self, path_segments: &[&str]) -> Result<ureq::Response> {
        let url = self.api_url(path_segments, &[])?;
        let resp = self
            .set_headers(ureq::delete(url.as_str()))
            .call()
            .with_context(|| {
                format!(
                    "Failed to delete GitHub API at path: {}",
                    path_segments.join("/")
                )
            })?;
        Ok(resp)
    }

    /// List notifications for the authenticated user.
    pub fn get_notifications(&self, since: Option<&str>, all: bool) -> Result<Vec<Notification>> {
        let mut notifications = Vec::new();
        let mut page = 1;

        loop {
            let mut query_pairs = vec![
                ("all", all.to_string()),
                ("per_page", "100".to_string()),
                ("page", page.to_string()),
            ];
            if let Some(since) = since {
                query_pairs.push(("since", since.to_string()));
            }

            let resp = self
                .api_get_with_query(&["notifications"], &query_pairs)
                .with_context(|| "Failed to query GitHub API for notifications")?;
            let has_next_page = link_header_has_next(resp.header("link"));
            let page_notifications: Vec<Notification> = resp
                .into_json()
                .with_context(|| "Failed to parse GitHub notifications response as JSON")?;
            notifications.extend(page_notifications);

            if !has_next_page {
                break;
            }
            page += 1;
        }

        Ok(notifications)
    }

    /// Mark a notification thread as read.
    pub fn mark_notification_thread_read(&self, thread_id: &str) -> Result<()> {
        self.api_patch_empty(&["notifications", "threads", thread_id])
            .with_context(|| {
                format!("Failed to mark GitHub notification thread '{thread_id}' as read")
            })?;
        Ok(())
    }

    /// Mark a notification thread as done.
    pub fn mark_notification_thread_done(&self, thread_id: &str) -> Result<()> {
        self.api_delete(&["notifications", "threads", thread_id])
            .with_context(|| {
                format!("Failed to mark GitHub notification thread '{thread_id}' as done")
            })?;
        Ok(())
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

    /// List open pull requests where the authenticated user has a pending review request.
    pub fn get_prs_to_review(&self, include_teams: bool) -> Result<Vec<PrToReview>> {
        let mut pull_requests = Vec::new();
        let mut cursor: Option<String> = None;
        let review_request_filter = if include_teams {
            "review-requested:@me"
        } else {
            "user-review-requested:@me"
        };
        let query = format!("is:pr is:open archived:false {review_request_filter}");

        loop {
            let variables = GraphQLPrsToReviewVariables {
                query: &query,
                cursor: cursor.as_deref(),
            };

            let data: GraphQLPrsToReviewData = self
                .graphql_query(gql_queries::prs_to_review::QUERY, &variables)
                .with_context(|| "Failed to query GitHub GraphQL API for PRs to review")?;

            let GraphQLSearchConnection { nodes, page_info } = data.search;
            pull_requests.extend(nodes.into_iter().flatten().map(PrToReview::from));

            if page_info.has_next_page {
                cursor = page_info.end_cursor;
                if cursor.is_none() {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(pull_requests)
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
        let mut all_comments = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let variables = GraphQLReviewCommentsVariables {
                owner,
                name: repo,
                pr_number: pr_number as i64,
                cursor: cursor.as_deref(),
            };

            let data: GraphQLReviewCommentsData = self
                .graphql_query(gql_queries::review_comments::QUERY, &variables)
                .with_context(|| {
                    format!(
                        "Failed to query GitHub GraphQL API for PR #{pr_number} review comments"
                    )
                })?;

            let repository = data.repository.ok_or_else(|| {
                anyhow!(
                    "Repository '{}/{}' not found when fetching review comments",
                    owner,
                    repo
                )
            })?;
            let pull_request = repository.pull_request.ok_or_else(|| {
                anyhow!(
                    "Pull request #{} not found in repository '{}/{}'",
                    pr_number,
                    owner,
                    repo
                )
            })?;

            let GraphQLReviewCommentConnection { nodes, page_info } = pull_request.review_comments;
            all_comments.extend(nodes.into_iter().map(ReviewComment::from));

            if page_info.has_next_page {
                cursor = page_info.end_cursor;
                if cursor.is_none() {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(all_comments)
    }

    fn graphql_query<T, V>(&self, query: &str, variables: &V) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        V: ?Sized + Serialize,
    {
        let response = self
            .set_headers(ureq::post(&self.graphql_base))
            .send_json(json!({ "query": query, "variables": variables }))
            .with_context(|| "Failed to execute GitHub GraphQL request")?;

        let parsed: gql_queries::common::GraphQLResponse<T> = response
            .into_json()
            .with_context(|| "Failed to parse GitHub GraphQL response as JSON")?;

        if let Some(errors) = parsed.errors {
            let messages = errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!("GitHub GraphQL API returned errors: {messages}"));
        }

        parsed
            .data
            .ok_or_else(|| anyhow!("GitHub GraphQL response missing data"))
    }
}

fn link_header_has_next(link_header: Option<&str>) -> bool {
    link_header
        .map(|header| {
            header
                .split(',')
                .any(|link| link.split(';').any(|part| part.trim() == "rel=\"next\""))
        })
        .unwrap_or(false)
}

impl From<GraphQLReviewCommentNode> for ReviewComment {
    fn from(node: GraphQLReviewCommentNode) -> Self {
        let GraphQLReviewCommentNode {
            body,
            author,
            created_at,
            path,
            original_line,
            original_start_line,
            diff_hunk,
            side,
            start_side,
            pull_request_review_thread,
        } = node;

        let user = author
            .map(|author| User {
                login: author.login,
                user_type: author.typename,
            })
            .unwrap_or_else(|| User {
                login: "unknown".to_string(),
                user_type: "Unknown".to_string(),
            });

        ReviewComment {
            body,
            user,
            created_at,
            path,
            original_line,
            original_start_line,
            diff_hunk,
            side,
            start_side,
            is_resolved: pull_request_review_thread.map(|thread| thread.is_resolved),
        }
    }
}

impl From<GraphQLPrToReviewNode> for PrToReview {
    fn from(node: GraphQLPrToReviewNode) -> Self {
        let author = node
            .author
            .map(|author| User {
                login: author.login,
                user_type: author.typename,
            })
            .unwrap_or_else(|| User {
                login: "unknown".to_string(),
                user_type: "Unknown".to_string(),
            });

        PrToReview {
            repo: node.repository.name_with_owner,
            repo_url: node.repository.url,
            number: node.number,
            title: node.title,
            author,
            state: node.state,
            is_draft: node.is_draft,
            url: node.url,
            created_at: node.created_at,
            updated_at: node.updated_at,
        }
    }
}

pub fn create_github_client(remote_url: &str, config: &Config) -> anyhow::Result<GitHubClient> {
    let host = host_from_remote_url(remote_url)?;
    create_github_client_for_host(&host, config)
}

pub fn create_github_client_for_host(host: &str, config: &Config) -> anyhow::Result<GitHubClient> {
    let github_token_raw = config.github_tokens.get(host).ok_or_else(|| {
        anyhow!(
            "GitHub token not configured for host '{}'. Add it to your config file.",
            host
        )
    })?;
    let github_token = resolve_secret(github_token_raw)
        .with_context(|| format!("Failed to resolve GitHub token for host '{host}'"))?;

    Ok(GitHubClient::new(host.to_string(), github_token))
}

pub fn is_bot_user(user_login: &str, user_type: &str) -> bool {
    user_type == "Bot" || user_login.starts_with("service") || user_login.ends_with("[bot]")
}
