use serde::{Deserialize, Serialize};

pub mod review_comments;

/// GraphQL request structure
#[derive(Debug, Serialize)]
pub struct GraphQLRequest {
    pub query: String,
    pub variables: serde_json::Value,
}

/// GraphQL response structure
#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// GraphQL error structure
#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}
