use crate::config::resolve_secret;
use crate::config::Config;
use crate::git::host_from_remote_url;
use crate::gql_queries;
use crate::gql_queries::common::{GraphQLAuthor, GraphQLPageInfo};
use crate::gql_queries::pr_details::{
    GraphQLCheckRunNode, GraphQLCommitConnection, GraphQLCommitNode, GraphQLFileConnection,
    GraphQLFileNode, GraphQLIssueCommentConnection, GraphQLIssueCommentNode, GraphQLPrCommentsPage,
    GraphQLPrCommitsPage, GraphQLPrConnectionPageData, GraphQLPrConnectionVariables,
    GraphQLPrDetailsData, GraphQLPrDetailsVariables, GraphQLPrFilesPage,
    GraphQLPrReviewRequestsPage, GraphQLPrReviewThreadsPage, GraphQLPrReviewsPage,
    GraphQLPrStatusChecksPage, GraphQLPullRequest, GraphQLRequestedReviewer,
    GraphQLReviewConnection, GraphQLReviewNode, GraphQLReviewRequestConnection,
    GraphQLReviewThreadCommentsData, GraphQLReviewThreadCommentsVariables, GraphQLStatusCommit,
};
use crate::gql_queries::prs_to_review::{
    GraphQLPrToReviewNode, GraphQLPrsToReviewData, GraphQLPrsToReviewVariables,
    GraphQLSearchConnection,
};
use crate::gql_queries::review_comments::{
    GraphQLReviewCommentConnection, GraphQLReviewCommentNode, GraphQLReviewCommentsData,
    GraphQLReviewCommentsVariables, GraphQLReviewThreadConnection, GraphQLReviewThreadNode,
};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub login: String,
    #[serde(rename = "type")]
    pub user_type: String,
}

/// A GitHub issue/PR comment (timeline comment)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IssueComment {
    pub body: String,
    pub user: User,
    pub created_at: String,
}

/// A GitHub pull request review comment (diff comment)
#[derive(Clone, Debug, Deserialize, Serialize)]
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

/// A GitHub pull request review thread.
#[derive(Debug, Serialize)]
pub struct ReviewThread {
    pub id: String,
    pub is_resolved: bool,
    pub diff_side: String,
    pub start_diff_side: Option<String>,
    pub comments: Vec<ReviewComment>,
    #[serde(skip)]
    comments_page_info: GraphQLPageInfo,
}

/// Container for all PR comment types
#[derive(Debug, Serialize)]
pub struct PrComments {
    pub issue_comments: Vec<IssueComment>,
    pub review_comments: Vec<ReviewComment>,
}

/// Review-oriented pull request details.
#[derive(Debug, Serialize)]
pub struct PullRequestDetails {
    pub pull_request: Value,
    pub diff: String,
    pub files: Vec<Value>,
    pub issue_comments: Vec<IssueComment>,
    pub review_comments: Vec<ReviewComment>,
    pub review_threads: Vec<ReviewThread>,
    pub reviews: Vec<Value>,
    pub commits: Vec<Value>,
    pub status: Option<Value>,
    pub check_runs: Option<Value>,
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

    fn api_get_with_accept(&self, path_segments: &[&str], accept: &str) -> Result<ureq::Response> {
        let url = self.api_url(path_segments, &[])?;
        let resp = self
            .set_headers(ureq::get(url.as_str()))
            .set("Accept", accept)
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

    /// Fetch details useful when reviewing a pull request.
    pub fn get_pull_request_details(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestDetails> {
        let pr_path = ["repos", owner, repo, "pulls", &pr_number.to_string()];
        let diff = self
            .api_get_with_accept(&pr_path, "application/vnd.github.v3.diff")
            .with_context(|| format!("Failed to query GitHub API for PR #{pr_number} diff"))?
            .into_string()
            .with_context(|| "Failed to parse GitHub pull request diff response")?;

        let mut files = Vec::new();
        let mut issue_comments = Vec::new();
        let mut review_threads = Vec::new();
        let mut reviews = Vec::new();
        let mut commits = Vec::new();

        let variables = GraphQLPrDetailsVariables {
            owner,
            name: repo,
            pr_number: pr_number as i64,
            file_cursor: None,
            commit_cursor: None,
            review_cursor: None,
            comment_cursor: None,
            thread_cursor: None,
            review_requests_cursor: None,
            status_check_cursor: None,
        };

        let data: GraphQLPrDetailsData = self
            .graphql_query(gql_queries::pr_details::QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} details")
            })?;
        let repository = data.repository.ok_or_else(|| {
            anyhow!(
                "Repository '{}/{}' not found when fetching PR details",
                owner,
                repo
            )
        })?;
        let mut pr = repository.pull_request.ok_or_else(|| {
            anyhow!(
                "Pull request #{} not found in repository '{}/{}'",
                pr_number,
                owner,
                repo
            )
        })?;

        let mut review_request_page_info = pr.review_requests.page_info.clone();
        while review_request_page_info.has_next_page {
            let Some(cursor) = review_request_page_info.end_cursor.as_deref() else {
                break;
            };
            let page =
                self.get_pull_request_review_requests_page(owner, repo, pr_number, cursor)?;
            pr.review_requests.nodes.extend(page.nodes);
            review_request_page_info = page.page_info;
        }

        let pull_request = pull_request_value(&pr);
        let mut status = None;
        let mut check_runs = None;
        if let Some(mut status_commit) = pr
            .commits_for_status
            .nodes
            .into_iter()
            .flatten()
            .next()
            .map(|node| node.commit)
        {
            let mut status_check_page_info = status_commit
                .status_check_rollup
                .as_ref()
                .map(|rollup| rollup.contexts.page_info.clone());
            while let Some(page_info) = status_check_page_info {
                if !page_info.has_next_page {
                    break;
                }
                let Some(cursor) = page_info.end_cursor.as_deref() else {
                    break;
                };
                let Some(page) =
                    self.get_pull_request_status_checks_page(owner, repo, pr_number, cursor)?
                else {
                    break;
                };
                status_check_page_info = Some(page.page_info.clone());
                if let Some(rollup) = status_commit.status_check_rollup.as_mut() {
                    rollup.contexts.nodes.extend(page.nodes);
                }
            }
            status = status_value(&status_commit);
            check_runs = Some(check_runs_value(&status_commit));
        }

        let mut file_page_info = pr.files.page_info;
        files.extend(pr.files.nodes.into_iter().flatten().map(file_value));

        let mut commit_page_info = pr.commits.page_info;
        commits.extend(pr.commits.nodes.into_iter().flatten().map(commit_value));

        let mut review_page_info = pr.reviews.page_info;
        reviews.extend(pr.reviews.nodes.into_iter().flatten().map(review_value));

        let mut comment_page_info = pr.comments.page_info;
        issue_comments.extend(
            pr.comments
                .nodes
                .into_iter()
                .flatten()
                .map(issue_comment_from_node),
        );

        let mut thread_page_info = pr.review_threads.page_info;
        review_threads.extend(
            pr.review_threads
                .nodes
                .into_iter()
                .map(review_thread_from_node),
        );

        while file_page_info.has_next_page {
            let Some(cursor) = file_page_info.end_cursor.as_deref() else {
                break;
            };
            let page = self.get_pull_request_files_page(owner, repo, pr_number, cursor)?;
            files.extend(page.nodes.into_iter().flatten().map(file_value));
            file_page_info = page.page_info;
        }

        while commit_page_info.has_next_page {
            let Some(cursor) = commit_page_info.end_cursor.as_deref() else {
                break;
            };
            let page = self.get_pull_request_commits_page(owner, repo, pr_number, cursor)?;
            commits.extend(page.nodes.into_iter().flatten().map(commit_value));
            commit_page_info = page.page_info;
        }

        while review_page_info.has_next_page {
            let Some(cursor) = review_page_info.end_cursor.as_deref() else {
                break;
            };
            let page = self.get_pull_request_reviews_page(owner, repo, pr_number, cursor)?;
            reviews.extend(page.nodes.into_iter().flatten().map(review_value));
            review_page_info = page.page_info;
        }

        while comment_page_info.has_next_page {
            let Some(cursor) = comment_page_info.end_cursor.as_deref() else {
                break;
            };
            let page = self.get_pull_request_comments_page(owner, repo, pr_number, cursor)?;
            issue_comments.extend(
                page.nodes
                    .into_iter()
                    .flatten()
                    .map(issue_comment_from_node),
            );
            comment_page_info = page.page_info;
        }

        while thread_page_info.has_next_page {
            let Some(cursor) = thread_page_info.end_cursor.as_deref() else {
                break;
            };
            let page = self.get_pull_request_review_threads_page(owner, repo, pr_number, cursor)?;
            review_threads.extend(page.nodes.into_iter().map(review_thread_from_node));
            thread_page_info = page.page_info;
        }
        self.hydrate_review_thread_comments(&mut review_threads)?;
        let review_comments = flatten_review_threads(&review_threads);

        Ok(PullRequestDetails {
            pull_request,
            diff,
            files,
            issue_comments,
            review_comments,
            review_threads,
            reviews,
            commits,
            status,
            check_runs,
        })
    }

    /// Get all comments for a pull request
    pub fn get_pr_comments(&self, owner: &str, repo: &str, pr_number: u64) -> Result<PrComments> {
        let issue_comments = self.get_issue_comments(owner, repo, pr_number)?;
        let review_comments =
            flatten_review_threads(&self.get_review_threads(owner, repo, pr_number)?);

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
        let mut comments = Vec::new();
        let mut page = 1;

        loop {
            let resp = self
                .api_get_with_query(
                    &[
                        "repos",
                        owner,
                        repo,
                        "issues",
                        &pr_number.to_string(),
                        "comments",
                    ],
                    &[("per_page", "100".to_string()), ("page", page.to_string())],
                )
                .with_context(|| {
                    format!("Failed to query GitHub API for PR #{pr_number} comments")
                })?;
            let has_next_page = link_header_has_next(resp.header("link"));
            let mut page_comments: Vec<IssueComment> = resp
                .into_json()
                .with_context(|| "Failed to parse GitHub issue comments response as JSON")?;
            comments.append(&mut page_comments);

            if !has_next_page {
                break;
            }
            page += 1;
        }

        Ok(comments)
    }

    /// Get review/diff threads for a PR
    fn get_review_threads(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<ReviewThread>> {
        let mut all_threads = Vec::new();
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

            let GraphQLReviewThreadConnection { nodes, page_info } = pull_request.review_threads;
            for thread in nodes {
                let mut thread = review_thread_from_node(thread);
                self.hydrate_review_thread_comments(std::slice::from_mut(&mut thread))?;
                all_threads.push(thread);
            }

            if page_info.has_next_page {
                cursor = page_info.end_cursor;
                if cursor.is_none() {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(all_threads)
    }

    fn get_pull_request_files_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<GraphQLFileConnection> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrFilesPage> = self
            .graphql_query(gql_queries::pr_details::FILES_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} files")
            })?;

        Ok(pull_request_connection_page(data, owner, repo, pr_number, "files")?.files)
    }

    fn get_pull_request_commits_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<GraphQLCommitConnection> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrCommitsPage> = self
            .graphql_query(gql_queries::pr_details::COMMITS_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} commits")
            })?;

        Ok(pull_request_connection_page(data, owner, repo, pr_number, "commits")?.commits)
    }

    fn get_pull_request_reviews_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<GraphQLReviewConnection> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrReviewsPage> = self
            .graphql_query(gql_queries::pr_details::REVIEWS_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} reviews")
            })?;

        Ok(pull_request_connection_page(data, owner, repo, pr_number, "reviews")?.reviews)
    }

    fn get_pull_request_comments_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<GraphQLIssueCommentConnection> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrCommentsPage> = self
            .graphql_query(gql_queries::pr_details::COMMENTS_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} comments")
            })?;

        Ok(pull_request_connection_page(data, owner, repo, pr_number, "comments")?.comments)
    }

    fn get_pull_request_review_threads_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<crate::gql_queries::pr_details::GraphQLReviewThreadConnection> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrReviewThreadsPage> = self
            .graphql_query(gql_queries::pr_details::REVIEW_THREADS_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} review threads")
            })?;

        Ok(
            pull_request_connection_page(data, owner, repo, pr_number, "review threads")?
                .review_threads,
        )
    }

    fn get_pull_request_review_requests_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<GraphQLReviewRequestConnection> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrReviewRequestsPage> = self
            .graphql_query(gql_queries::pr_details::REVIEW_REQUESTS_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} review requests")
            })?;

        Ok(
            pull_request_connection_page(data, owner, repo, pr_number, "review requests")?
                .review_requests,
        )
    }

    fn get_pull_request_status_checks_page(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        cursor: &str,
    ) -> Result<Option<crate::gql_queries::pr_details::GraphQLCheckRunConnection>> {
        let variables = pr_connection_variables(owner, repo, pr_number, cursor);
        let data: GraphQLPrConnectionPageData<GraphQLPrStatusChecksPage> = self
            .graphql_query(gql_queries::pr_details::STATUS_CHECKS_QUERY, &variables)
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for PR #{pr_number} status checks")
            })?;

        let page = pull_request_connection_page(data, owner, repo, pr_number, "status checks")?;
        Ok(page
            .commits_for_status
            .nodes
            .into_iter()
            .flatten()
            .next()
            .and_then(|node| node.commit.status_check_rollup)
            .map(|rollup| rollup.contexts))
    }

    fn get_review_thread_comments_page(
        &self,
        thread_id: &str,
        cursor: &str,
    ) -> Result<crate::gql_queries::review_comments::GraphQLReviewCommentConnection> {
        let variables = GraphQLReviewThreadCommentsVariables {
            thread_id,
            cursor: Some(cursor),
        };
        let data: GraphQLReviewThreadCommentsData = self
            .graphql_query(
                gql_queries::pr_details::REVIEW_THREAD_COMMENTS_QUERY,
                &variables,
            )
            .with_context(|| {
                format!("Failed to query GitHub GraphQL API for review thread {thread_id} comments")
            })?;

        Ok(data
            .node
            .ok_or_else(|| anyhow!("Review thread '{thread_id}' not found"))?
            .comments)
    }

    fn hydrate_review_thread_comments(&self, threads: &mut [ReviewThread]) -> Result<()> {
        for thread in threads {
            while thread.comments_page_info.has_next_page {
                let Some(cursor) = thread.comments_page_info.end_cursor.as_deref() else {
                    break;
                };
                let page = self.get_review_thread_comments_page(&thread.id, cursor)?;
                thread
                    .comments
                    .extend(page.nodes.into_iter().map(|comment| {
                        review_comment_from_node(
                            comment,
                            &thread.diff_side,
                            &thread.start_diff_side,
                            thread.is_resolved,
                        )
                    }));
                thread.comments_page_info = page.page_info;
            }
        }

        Ok(())
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

fn pr_connection_variables<'a>(
    owner: &'a str,
    repo: &'a str,
    pr_number: u64,
    cursor: &'a str,
) -> GraphQLPrConnectionVariables<'a> {
    GraphQLPrConnectionVariables {
        owner,
        name: repo,
        pr_number: pr_number as i64,
        cursor: Some(cursor),
    }
}

fn pull_request_connection_page<T>(
    data: GraphQLPrConnectionPageData<T>,
    owner: &str,
    repo: &str,
    pr_number: u64,
    connection_name: &str,
) -> Result<T> {
    let repository = data.repository.ok_or_else(|| {
        anyhow!(
            "Repository '{}/{}' not found when fetching PR {}",
            owner,
            repo,
            connection_name
        )
    })?;

    repository.pull_request.ok_or_else(|| {
        anyhow!(
            "Pull request #{} not found in repository '{}/{}' when fetching {}",
            pr_number,
            owner,
            repo,
            connection_name
        )
    })
}

fn review_comment_from_node(
    node: GraphQLReviewCommentNode,
    diff_side: &str,
    start_diff_side: &Option<String>,
    is_resolved: bool,
) -> ReviewComment {
    let GraphQLReviewCommentNode {
        body,
        author,
        created_at,
        path,
        original_line,
        original_start_line,
        diff_hunk,
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
        side: diff_side.to_string(),
        start_side: start_diff_side.clone(),
        is_resolved: Some(is_resolved),
    }
}

fn review_thread_from_node(node: GraphQLReviewThreadNode) -> ReviewThread {
    let GraphQLReviewThreadNode {
        id,
        is_resolved,
        diff_side,
        start_diff_side,
        comments,
    } = node;
    let GraphQLReviewCommentConnection { nodes, page_info } = comments;

    ReviewThread {
        id,
        is_resolved,
        diff_side: diff_side.clone(),
        start_diff_side: start_diff_side.clone(),
        comments_page_info: page_info,
        comments: nodes
            .into_iter()
            .map(|comment| {
                review_comment_from_node(comment, &diff_side, &start_diff_side, is_resolved)
            })
            .collect(),
    }
}

fn flatten_review_threads(threads: &[ReviewThread]) -> Vec<ReviewComment> {
    threads
        .iter()
        .flat_map(|thread| {
            thread.comments.iter().map(|comment| ReviewComment {
                body: comment.body.clone(),
                user: User {
                    login: comment.user.login.clone(),
                    user_type: comment.user.user_type.clone(),
                },
                created_at: comment.created_at.clone(),
                path: comment.path.clone(),
                original_line: comment.original_line,
                original_start_line: comment.original_start_line,
                diff_hunk: comment.diff_hunk.clone(),
                side: comment.side.clone(),
                start_side: comment.start_side.clone(),
                is_resolved: comment.is_resolved,
            })
        })
        .collect()
}

fn graphql_author_to_user(author: Option<GraphQLAuthor>) -> User {
    author
        .map(|author| User {
            login: author.login,
            user_type: author.typename,
        })
        .unwrap_or_else(|| User {
            login: "unknown".to_string(),
            user_type: "Unknown".to_string(),
        })
}

fn pull_request_value(pr: &GraphQLPullRequest) -> Value {
    let requested_reviewers: Vec<Value> = pr
        .review_requests
        .nodes
        .iter()
        .filter_map(|node| node.as_ref())
        .filter_map(|node| node.requested_reviewer.as_ref())
        .filter(|reviewer| reviewer.typename == "User")
        .filter_map(requested_user_value)
        .collect();
    let requested_teams: Vec<Value> = pr
        .review_requests
        .nodes
        .iter()
        .filter_map(|node| node.as_ref())
        .filter_map(|node| node.requested_reviewer.as_ref())
        .filter(|reviewer| reviewer.typename == "Team")
        .filter_map(requested_team_value)
        .collect();
    let head_repo = pr.head_repository.as_ref().unwrap_or(&pr.base_repository);

    json!({
        "number": pr.number,
        "title": pr.title,
        "html_url": pr.url,
        "state": pr.state.to_lowercase(),
        "body": pr.body,
        "created_at": pr.created_at,
        "updated_at": pr.updated_at,
        "merged_at": pr.merged_at,
        "additions": pr.additions,
        "deletions": pr.deletions,
        "changed_files": pr.changed_files,
        "user": graphql_author_to_user(pr.author.clone()),
        "base": {
            "label": format!("{}:{}", pr.base_repository.name_with_owner, pr.base_ref_name),
            "sha": pr.base_ref_oid,
        },
        "head": {
            "label": format!("{}:{}", head_repo.name_with_owner, pr.head_ref_name),
            "sha": pr.head_ref_oid,
        },
        "requested_reviewers": requested_reviewers,
        "requested_teams": requested_teams,
    })
}

fn requested_user_value(reviewer: &GraphQLRequestedReviewer) -> Option<Value> {
    Some(json!({
        "login": reviewer.login.as_ref()?,
        "type": reviewer.typename,
    }))
}

fn requested_team_value(reviewer: &GraphQLRequestedReviewer) -> Option<Value> {
    Some(json!({
        "name": reviewer.name.as_ref()?,
        "slug": reviewer.slug.as_ref()?,
    }))
}

fn file_value(file: GraphQLFileNode) -> Value {
    json!({
        "filename": file.path,
        "status": file.change_type.to_lowercase(),
        "additions": file.additions,
        "deletions": file.deletions,
    })
}

fn commit_value(commit: GraphQLCommitNode) -> Value {
    json!({
        "sha": commit.commit.oid,
        "commit": {
            "message": commit.commit.message,
        },
    })
}

fn review_value(review: GraphQLReviewNode) -> Value {
    json!({
        "user": graphql_author_to_user(review.author),
        "state": review.state,
        "submitted_at": review.submitted_at,
        "body": review.body,
    })
}

fn issue_comment_from_node(comment: GraphQLIssueCommentNode) -> IssueComment {
    IssueComment {
        body: comment.body,
        user: graphql_author_to_user(comment.author),
        created_at: comment.created_at,
    }
}

fn status_value(commit: &GraphQLStatusCommit) -> Option<Value> {
    commit.status.as_ref().map(|status| {
        let contexts: Vec<Value> = status.contexts.iter().map(status_context_value).collect();
        json!({
            "sha": commit.oid,
            "state": status.state.to_lowercase(),
            "total_count": status.contexts.len(),
            "contexts": contexts,
        })
    })
}

fn check_runs_value(commit: &GraphQLStatusCommit) -> Value {
    let Some(rollup) = &commit.status_check_rollup else {
        return json!({
            "total_count": 0,
            "check_runs": [],
        });
    };

    let check_runs: Vec<Value> = commit
        .status_check_rollup
        .as_ref()
        .map(|rollup| &rollup.contexts)
        .into_iter()
        .flat_map(|contexts| contexts.nodes.iter())
        .filter_map(|node| node.as_ref())
        .filter(|node| node.typename == "CheckRun")
        .map(check_run_value)
        .collect();
    let status_contexts: Vec<Value> = commit
        .status_check_rollup
        .as_ref()
        .map(|rollup| &rollup.contexts)
        .into_iter()
        .flat_map(|contexts| contexts.nodes.iter())
        .filter_map(|node| node.as_ref())
        .filter(|node| node.typename == "StatusContext")
        .map(status_context_rollup_value)
        .collect();

    json!({
        "total_count": rollup.contexts.total_count,
        "check_runs": check_runs,
        "status_contexts": status_contexts,
    })
}

fn check_run_value(run: &GraphQLCheckRunNode) -> Value {
    json!({
        "name": run.name.as_deref().unwrap_or("(unnamed)"),
        "status": run.status.as_ref().map(|status| status.to_lowercase()).unwrap_or_else(|| "unknown".to_string()),
        "conclusion": run.conclusion.as_ref().map(|conclusion| conclusion.to_lowercase()),
        "details_url": run.details_url,
    })
}

fn status_context_value(context: &crate::gql_queries::pr_details::GraphQLStatusContext) -> Value {
    json!({
        "context": context.context,
        "state": context.state.to_lowercase(),
        "description": context.description,
        "target_url": context.target_url,
    })
}

fn status_context_rollup_value(context: &GraphQLCheckRunNode) -> Value {
    json!({
        "context": context.context.as_deref().unwrap_or("(unnamed)"),
        "state": context.state.as_ref().map(|state| state.to_lowercase()).unwrap_or_else(|| "unknown".to_string()),
        "description": context.description,
        "target_url": context.target_url,
    })
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
