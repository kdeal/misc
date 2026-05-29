use super::common::{GraphQLAuthor, GraphQLPageInfo};
use serde::{Deserialize, Serialize};

pub const QUERY: &str = include_str!("review_comments.graphql");

#[derive(Debug, Serialize)]
pub struct GraphQLReviewCommentsVariables<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    #[serde(rename = "prNumber")]
    pub pr_number: i64,
    pub cursor: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewCommentsData {
    pub repository: Option<GraphQLRepository>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLRepository {
    #[serde(rename = "pullRequest")]
    pub pull_request: Option<GraphQLPullRequest>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPullRequest {
    #[serde(rename = "reviewThreads")]
    pub review_threads: GraphQLReviewThreadConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewThreadConnection {
    pub nodes: Vec<GraphQLReviewThreadNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewThreadNode {
    pub id: String,
    #[serde(rename = "isResolved")]
    pub is_resolved: bool,
    #[serde(rename = "diffSide")]
    pub diff_side: String,
    #[serde(rename = "startDiffSide")]
    pub start_diff_side: Option<String>,
    pub comments: GraphQLReviewCommentConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewCommentConnection {
    pub nodes: Vec<GraphQLReviewCommentNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewCommentNode {
    pub body: String,
    pub author: Option<GraphQLAuthor>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub path: String,
    #[serde(rename = "originalLine")]
    pub original_line: Option<u32>,
    #[serde(rename = "originalStartLine")]
    pub original_start_line: Option<u32>,
    #[serde(rename = "diffHunk")]
    pub diff_hunk: String,
}
