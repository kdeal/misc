use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GraphQLPageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GraphQLAuthor {
    pub login: String,
    #[serde(rename = "__typename")]
    pub typename: String,
}
