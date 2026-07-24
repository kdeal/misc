use super::common::{GraphQLAuthor, GraphQLPageInfo};
use serde::{Deserialize, Serialize};

pub const QUERY: &str = include_str!("pr_details.graphql");
pub const FILES_QUERY: &str = include_str!("pr_details_files.graphql");
pub const COMMITS_QUERY: &str = include_str!("pr_details_commits.graphql");
pub const REVIEWS_QUERY: &str = include_str!("pr_details_reviews.graphql");
pub const COMMENTS_QUERY: &str = include_str!("pr_details_comments.graphql");
pub const REVIEW_THREADS_QUERY: &str = include_str!("pr_details_review_threads.graphql");
pub const REVIEW_REQUESTS_QUERY: &str = include_str!("pr_details_review_requests.graphql");
pub const REVIEW_THREAD_COMMENTS_QUERY: &str =
    include_str!("pr_details_review_thread_comments.graphql");
pub const STATUS_CHECKS_QUERY: &str = include_str!("pr_details_status_checks.graphql");

#[derive(Debug, Serialize)]
pub struct GraphQLPrDetailsVariables<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    #[serde(rename = "prNumber")]
    pub pr_number: i64,
    #[serde(rename = "fileCursor")]
    pub file_cursor: Option<&'a str>,
    #[serde(rename = "commitCursor")]
    pub commit_cursor: Option<&'a str>,
    #[serde(rename = "reviewCursor")]
    pub review_cursor: Option<&'a str>,
    #[serde(rename = "commentCursor")]
    pub comment_cursor: Option<&'a str>,
    #[serde(rename = "threadCursor")]
    pub thread_cursor: Option<&'a str>,
    #[serde(rename = "reviewRequestsCursor")]
    pub review_requests_cursor: Option<&'a str>,
    #[serde(rename = "statusCheckCursor")]
    pub status_check_cursor: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct GraphQLPrConnectionVariables<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    #[serde(rename = "prNumber")]
    pub pr_number: i64,
    pub cursor: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrDetailsData {
    pub repository: Option<GraphQLRepository>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrConnectionPageData<T> {
    pub repository: Option<GraphQLConnectionPageRepository<T>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLConnectionPageRepository<T> {
    #[serde(rename = "pullRequest")]
    pub pull_request: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrFilesPage {
    pub files: GraphQLFileConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrCommitsPage {
    pub commits: GraphQLCommitConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrReviewsPage {
    pub reviews: GraphQLReviewConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrCommentsPage {
    pub comments: GraphQLIssueCommentConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrReviewThreadsPage {
    #[serde(rename = "reviewThreads")]
    pub review_threads: GraphQLReviewThreadConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrReviewRequestsPage {
    #[serde(rename = "reviewRequests")]
    pub review_requests: GraphQLReviewRequestConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrStatusChecksPage {
    #[serde(rename = "commitsForStatus")]
    pub commits_for_status: GraphQLStatusCommitConnection,
}

#[derive(Debug, Serialize)]
pub struct GraphQLReviewThreadCommentsVariables<'a> {
    #[serde(rename = "threadId")]
    pub thread_id: &'a str,
    pub cursor: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewThreadCommentsData {
    pub node: Option<GraphQLReviewThreadCommentsNode>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewThreadCommentsNode {
    pub comments: super::review_comments::GraphQLReviewCommentConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLRepository {
    #[serde(rename = "pullRequest")]
    pub pull_request: Option<GraphQLPullRequest>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "mergedAt")]
    pub merged_at: Option<String>,
    pub additions: u64,
    pub deletions: u64,
    #[serde(rename = "changedFiles")]
    pub changed_files: u64,
    pub author: Option<GraphQLAuthor>,
    #[serde(rename = "baseRefName")]
    pub base_ref_name: String,
    #[serde(rename = "baseRefOid")]
    pub base_ref_oid: String,
    #[serde(rename = "headRefName")]
    pub head_ref_name: String,
    #[serde(rename = "headRefOid")]
    pub head_ref_oid: String,
    #[serde(rename = "baseRepository")]
    pub base_repository: GraphQLPrRepository,
    #[serde(rename = "headRepository")]
    pub head_repository: Option<GraphQLPrRepository>,
    #[serde(rename = "reviewRequests")]
    pub review_requests: GraphQLReviewRequestConnection,
    pub files: GraphQLFileConnection,
    pub commits: GraphQLCommitConnection,
    pub reviews: GraphQLReviewConnection,
    pub comments: GraphQLIssueCommentConnection,
    #[serde(rename = "reviewThreads")]
    pub review_threads: GraphQLReviewThreadConnection,
    #[serde(rename = "commitsForStatus")]
    pub commits_for_status: GraphQLStatusCommitConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrRepository {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewRequestConnection {
    pub nodes: Vec<Option<GraphQLReviewRequestNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewRequestNode {
    #[serde(rename = "requestedReviewer")]
    pub requested_reviewer: Option<GraphQLRequestedReviewer>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLRequestedReviewer {
    #[serde(rename = "__typename")]
    pub typename: String,
    pub login: Option<String>,
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLFileConnection {
    pub nodes: Vec<Option<GraphQLFileNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLFileNode {
    pub path: String,
    pub additions: u64,
    pub deletions: u64,
    #[serde(rename = "changeType")]
    pub change_type: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLCommitConnection {
    pub nodes: Vec<Option<GraphQLCommitNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLCommitNode {
    pub commit: GraphQLCommit,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLCommit {
    pub oid: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewConnection {
    pub nodes: Vec<Option<GraphQLReviewNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewNode {
    pub author: Option<GraphQLAuthor>,
    pub state: String,
    #[serde(rename = "submittedAt")]
    pub submitted_at: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLIssueCommentConnection {
    pub nodes: Vec<Option<GraphQLIssueCommentNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLIssueCommentNode {
    pub body: String,
    pub author: Option<GraphQLAuthor>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewThreadConnection {
    pub nodes: Vec<super::review_comments::GraphQLReviewThreadNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLStatusCommitConnection {
    pub nodes: Vec<Option<GraphQLStatusCommitNode>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLStatusCommitNode {
    pub commit: GraphQLStatusCommit,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLStatusCommit {
    pub oid: String,
    pub status: Option<GraphQLStatus>,
    #[serde(rename = "statusCheckRollup")]
    pub status_check_rollup: Option<GraphQLStatusCheckRollup>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLStatus {
    pub state: String,
    pub contexts: Vec<GraphQLStatusContext>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLStatusContext {
    pub context: String,
    pub state: String,
    pub description: Option<String>,
    #[serde(rename = "targetUrl")]
    pub target_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLStatusCheckRollup {
    pub contexts: GraphQLCheckRunConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLCheckRunConnection {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub nodes: Vec<Option<GraphQLCheckRunNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLCheckRunNode {
    #[serde(rename = "__typename")]
    pub typename: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    #[serde(rename = "detailsUrl")]
    pub details_url: Option<String>,
    pub context: Option<String>,
    pub state: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "targetUrl")]
    pub target_url: Option<String>,
}
