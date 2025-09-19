use crate::config::resolve_secret;
use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::Deserialize;
use url::Url;

/// A Jira issue minimal representation
#[derive(Debug, Deserialize)]
pub struct Issue {
    #[allow(dead_code)]
    pub id: String,
    pub key: String,
    pub fields: IssueFields,
}

/// Jira issue fields
#[derive(Debug, Deserialize)]
pub struct IssueFields {
    pub summary: String,
    pub description: Option<Document>,
    pub status: Status,
    pub assignee: Option<User>,
    pub reporter: Option<User>,
    pub created: String,
    pub updated: String,
    pub priority: Option<Priority>,
    pub issuetype: IssueType,
    pub project: Project,
    pub comment: CommentBlock,
}

#[derive(Debug, Deserialize)]
pub struct CommentBlock {
    pub comments: Vec<Comment>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub key: String,
}

/// A Jira user
#[derive(Debug, Deserialize)]
pub struct User {
    #[serde(rename = "accountId")]
    #[allow(dead_code)]
    pub account_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress")]
    #[allow(dead_code)]
    pub email_address: Option<String>,
}

/// Jira issue status
#[derive(Debug, Deserialize)]
pub struct Status {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    #[serde(rename = "statusCategory")]
    #[allow(dead_code)]
    pub status_category: StatusCategory,
}

/// Jira status category
#[derive(Debug, Deserialize)]
pub struct StatusCategory {
    #[allow(dead_code)]
    pub key: String,
    #[allow(dead_code)]
    pub name: String,
}

/// Jira issue priority
#[derive(Debug, Deserialize)]
pub struct Priority {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
}

/// Jira issue type
#[derive(Debug, Deserialize)]
pub struct IssueType {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub subtask: bool,
}

/// A Jira comment
#[derive(Debug, Deserialize)]
pub struct Comment {
    #[allow(dead_code)]
    pub id: String,
    pub body: Document,
    pub author: User,
    pub created: String,
    #[allow(dead_code)]
    pub updated: String,
}

/// Jira comment body (Atlassian Document Format)
#[derive(Debug, Deserialize)]
pub struct Document {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: String,
    #[allow(dead_code)]
    pub version: u32,
    pub content: Vec<serde_json::Value>,
}

impl Document {
    /// Extract plain text from ADF (Atlassian Document Format)
    pub fn to_plain_text(&self) -> String {
        extract_text_from_adf(&self.content)
    }
}

/// Extract text from ADF content using iterative approach to avoid stack overflow
fn extract_text_from_adf(content: &[serde_json::Value]) -> String {
    let mut text = String::new();
    let mut stack = Vec::new();

    // Initialize stack with root content items
    for item in content.iter().rev() {
        stack.push((item, false)); // (item, should_add_newline_after)
    }

    while let Some((item, add_newline)) = stack.pop() {
        if add_newline {
            text.push('\n');
            continue;
        }

        if let Some(obj) = item.as_object() {
            if let Some(item_type) = obj.get("type").and_then(|t| t.as_str()) {
                match item_type {
                    "text" => {
                        if let Some(text_content) = obj.get("text").and_then(|t| t.as_str()) {
                            text.push_str(text_content);
                        }
                    }
                    "paragraph" | "blockquote" | "codeBlock" => {
                        if let Some(child_content) = obj.get("content").and_then(|c| c.as_array()) {
                            // Add a marker to add newline after processing children
                            stack.push((item, true));
                            // Add children in reverse order so they're processed in correct order
                            for child in child_content.iter().rev() {
                                stack.push((child, false));
                            }
                        }
                    }
                    "hardBreak" => text.push('\n'),
                    _ => {
                        // For other node types, just process their content if it exists
                        if let Some(child_content) = obj.get("content").and_then(|c| c.as_array()) {
                            for child in child_content.iter().rev() {
                                stack.push((child, false));
                            }
                        }
                    }
                }
            }
        }
    }

    text
}

/// Client for interacting with the Jira API
pub struct JiraClient {
    api_base: String,
    email: String,
    api_token: String,
}

impl JiraClient {
    /// Create a new Jira client
    pub fn new(instance_url: String, email: String, api_token: String) -> Self {
        let api_base = format!("{}/rest/api/3", instance_url.trim_end_matches('/'));
        JiraClient {
            api_base,
            email,
            api_token,
        }
    }

    /// Make a GET request to the Jira API with optional query parameters
    fn api_get_with_query(
        &self,
        path_segments: &[&str],
        query_params: Option<&[(&str, &str)]>,
    ) -> Result<ureq::Response> {
        let mut url = Url::parse(&self.api_base)?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("Failed to set URL path segments"))?;
            segments.extend(path_segments);
        }

        if let Some(params) = query_params {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in params {
                query_pairs.append_pair(key, value);
            }
        }

        let auth_header = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(format!("{}:{}", self.email, self.api_token))
        );

        let resp = ureq::get(url.as_str())
            .set("Authorization", &auth_header)
            .set("Accept", "application/json")
            .call()
            .with_context(|| {
                format!(
                    "Failed to query Jira API at path: {}",
                    path_segments.join("/")
                )
            })?;
        Ok(resp)
    }

    /// Make a GET request to the Jira API
    fn api_get(&self, path_segments: &[&str]) -> Result<ureq::Response> {
        self.api_get_with_query(path_segments, None)
    }

    /// Get a Jira issue by key
    pub fn get_issue(&self, issue_key: &str) -> Result<Issue> {
        let resp = self
            .api_get(&["issue", issue_key])
            .with_context(|| format!("Failed to query Jira API for issue '{issue_key}'. Check that the issue key exists and you have permission to view it"))?;

        let issue: Issue = resp
            .into_json()
            .with_context(|| "Failed to parse Jira issue response as JSON")?;
        Ok(issue)
    }

    /// Search for issues using JQL
    pub fn search_issues(&self, jql: &str, max_results: Option<u32>) -> Result<Vec<Issue>> {
        let query_params = [
            ("jql", jql),
            ("maxResults", &max_results.unwrap_or(50).to_string()),
        ];

        let resp = self
            .api_get_with_query(&["search"], Some(&query_params))
            .with_context(|| format!("Failed to search Jira issues with JQL: {jql}"))?;

        #[derive(Deserialize)]
        struct SearchResponse {
            issues: Vec<Issue>,
        }

        let search_response: SearchResponse = resp
            .into_json()
            .with_context(|| "Failed to parse Jira search response as JSON")?;

        Ok(search_response.issues)
    }
}

pub fn create_jira_client(config: &Config) -> anyhow::Result<JiraClient> {
    let jira_config = config.jira.as_ref().ok_or_else(|| {
        anyhow!("Jira configuration not found. Add jira section to your config file.")
    })?;

    let instance_url = jira_config.instance_url.clone();
    let email = jira_config.email.clone();
    let api_token_raw = &jira_config.api_token;
    let api_token =
        resolve_secret(api_token_raw).with_context(|| "Failed to resolve Jira API token")?;

    Ok(JiraClient::new(instance_url, email, api_token))
}

pub fn format_jira_date(date_str: &str) -> String {
    if let Some((date_part, rest)) = date_str.split_once('T') {
        let time_no_tz = rest.split(['+', '-', 'Z']).next().unwrap_or(rest);
        let time_clean = time_no_tz.split('.').next().unwrap_or(time_no_tz);
        format!("{} {}", date_part, time_clean)
    } else {
        date_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_jira_date_with_timezone_plus() {
        let input = "2023-10-15T14:30:25.123+0200";
        let expected = "2023-10-15 14:30:25";
        assert_eq!(format_jira_date(input), expected);
    }

    #[test]
    fn test_format_jira_date_with_timezone_minus() {
        let input = "2023-10-15T14:30:25.456-0500";
        let expected = "2023-10-15 14:30:25";
        assert_eq!(format_jira_date(input), expected);
    }

    #[test]
    fn test_format_jira_date_with_z_timezone() {
        let input = "2023-10-15T14:30:25.789Z";
        let expected = "2023-10-15 14:30:25";
        assert_eq!(format_jira_date(input), expected);
    }

    #[test]
    fn test_format_jira_date_without_milliseconds() {
        let input = "2023-10-15T14:30:25+0000";
        let expected = "2023-10-15 14:30:25";
        assert_eq!(format_jira_date(input), expected);
    }

    #[test]
    fn test_format_jira_date_without_timezone() {
        let input = "2023-10-15T14:30:25";
        let expected = "2023-10-15 14:30:25";
        assert_eq!(format_jira_date(input), expected);
    }

    #[test]
    fn test_format_jira_date_without_t_separator() {
        let input = "2023-10-15 14:30:25";
        let expected = "2023-10-15 14:30:25";
        assert_eq!(format_jira_date(input), expected);
    }

    #[test]
    fn test_format_jira_date_simple_date() {
        let input = "2023-10-15";
        let expected = "2023-10-15";
        assert_eq!(format_jira_date(input), expected);
    }
}
