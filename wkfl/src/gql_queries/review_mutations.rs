use serde::{Deserialize, Serialize};

pub const ADD_REVIEW_THREAD_MUTATION: &str = include_str!("review_mutations.graphql");

#[derive(Debug, Serialize)]
pub struct GraphQLAddReviewThreadVariables<'a> {
    pub input: GraphQLAddReviewThreadInput<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLAddReviewThreadInput<'a> {
    pub body: &'a str,
    pub path: &'a str,
    pub line: u32,
    pub side: &'a str,
    pub pull_request_review_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_side: Option<&'a str>,
    pub subject_type: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLAddReviewThreadData {
    #[serde(rename = "addPullRequestReviewThread")]
    pub add_pull_request_review_thread: Option<GraphQLAddReviewThreadPayload>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLAddReviewThreadPayload {
    pub thread: Option<GraphQLReviewThread>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLReviewThread {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::{GraphQLAddReviewThreadInput, GraphQLAddReviewThreadVariables};
    use serde_json::json;

    #[test]
    fn add_review_thread_variables_use_graphql_field_names() {
        let variables = GraphQLAddReviewThreadVariables {
            input: GraphQLAddReviewThreadInput {
                body: "Please simplify this block",
                path: "src/main.rs",
                line: 24,
                side: "RIGHT",
                pull_request_review_id: "PRR_node_id",
                start_line: Some(20),
                start_side: Some("RIGHT"),
                subject_type: "LINE",
            },
        };

        assert_eq!(
            serde_json::to_value(variables).unwrap(),
            json!({
                "input": {
                    "body": "Please simplify this block",
                    "path": "src/main.rs",
                    "line": 24,
                    "side": "RIGHT",
                    "pullRequestReviewId": "PRR_node_id",
                    "startLine": 20,
                    "startSide": "RIGHT",
                    "subjectType": "LINE"
                }
            })
        );
    }
}
