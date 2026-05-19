use super::common::{GraphQLAuthor, GraphQLPageInfo};
use serde::{Deserialize, Serialize};

pub const QUERY: &str = include_str!("prs_to_review.graphql");

#[derive(Debug, Serialize)]
pub struct GraphQLPrsToReviewVariables<'a> {
    pub query: &'a str,
    pub cursor: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrsToReviewData {
    pub search: GraphQLSearchConnection,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLSearchConnection {
    pub nodes: Vec<Option<GraphQLPrToReviewNode>>,
    #[serde(rename = "pageInfo")]
    pub page_info: GraphQLPageInfo,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLPrToReviewNode {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub author: Option<GraphQLAuthor>,
    pub repository: GraphQLRepository,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLRepository {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
    pub url: String,
}
