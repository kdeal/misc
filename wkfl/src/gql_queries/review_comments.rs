use serde::Deserialize;

/// The query string for fetching review comments
pub const QUERY: &str = include_str!("review_comments.graphql");

/// Root data structure for review comments query response
#[derive(Debug, Deserialize)]
pub struct Data {
    pub repository: Repository,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    #[serde(rename = "pullRequest")]
    pub pull_request: PullRequest,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    #[serde(rename = "reviewThreads")]
    pub review_threads: ReviewThreadConnection,
}

#[derive(Debug, Deserialize)]
pub struct ReviewThreadConnection {
    pub nodes: Vec<ReviewThread>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewThread {
    pub comments: ReviewCommentConnection,
}

#[derive(Debug, Deserialize)]
pub struct ReviewCommentConnection {
    pub nodes: Vec<ReviewComment>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewComment {
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub path: String,
    #[serde(rename = "originalLine")]
    pub original_line: Option<u32>,
    #[serde(rename = "originalStartLine")]
    pub original_start_line: Option<u32>,
    #[serde(rename = "diffHunk")]
    pub diff_hunk: String,
    pub side: String,
    #[serde(rename = "startSide")]
    pub start_side: Option<String>,
    pub author: Author,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub login: String,
    #[serde(rename = "__typename")]
    pub typename: String,
}
