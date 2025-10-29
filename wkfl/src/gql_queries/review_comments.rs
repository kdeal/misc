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
    #[serde(rename = "reviewComments")]
    pub review_comments: GraphQLReviewCommentConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewCommentConnection {
    pub nodes: Vec<GraphQLReviewCommentNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
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
    pub side: String,
    #[serde(rename = "startSide")]
    pub start_side: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLAuthor {
    pub login: String,
    #[serde(rename = "__typename")]
    pub typename: String,
}
