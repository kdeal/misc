use crate::config::{get_config, get_repo_config, Config};
use crate::git;
use crate::git::extract_owner_repo_from_url;
use crate::github::create_github_client;
use crate::github::PrComments;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

pub const JSONRPC_VERSION: &str = "2.0";
pub const LATEST_PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCError {
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: ErrorObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request(JSONRPCRequest),
    Response(JSONRPCResponse),
    Error(JSONRPCError),
    Notification(JSONRPCNotification),
}

// MCP-specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,
    #[serde(rename = "outputSchema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<ToolOutputSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<TextContent>,
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(rename = "structuredContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
}

pub struct McpServer {
    server_info: Implementation,
    capabilities: ServerCapabilities,
    tools: Vec<Tool>,
}

impl McpServer {
    pub fn new() -> Self {
        let server_info = Implementation {
            name: "wkfl".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("WKFL MCP Server".to_string()),
        };

        let capabilities = ServerCapabilities {
            tools: ToolsCapability {
                list_changed: Some(false),
            },
            resources: None,
            prompts: None,
            logging: None,
            experimental: None,
        };

        let tools = vec![
            Tool {
                name: "get_test_commands".to_string(),
                title: Some("Get Test Commands".to_string()),
                description: Some(
                    "Get test commands configured in the repository's .wkfl.toml config"
                        .to_string(),
                ),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "repo_path".to_string(),
                            json!({
                                "type": "string",
                                "description": "Path to the repository root directory"
                            }),
                        );
                        props
                    }),
                    required: Some(vec!["repo_path".to_string()]),
                },
                output_schema: Some(ToolOutputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "commands".to_string(),
                            json!({
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "List of test commands"
                            }),
                        );
                        props.insert(
                            "error".to_string(),
                            json!({
                                "type": "string",
                                "description": "Error message if command retrieval failed"
                            }),
                        );
                        props
                    }),
                    required: None,
                }),
            },
            Tool {
                name: "get_fmt_commands".to_string(),
                title: Some("Get Format Commands".to_string()),
                description: Some(
                    "Get format commands configured in the repository's .wkfl.toml config"
                        .to_string(),
                ),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "repo_path".to_string(),
                            json!({
                                "type": "string",
                                "description": "Path to the repository root directory"
                            }),
                        );
                        props
                    }),
                    required: Some(vec!["repo_path".to_string()]),
                },
                output_schema: Some(ToolOutputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "commands".to_string(),
                            json!({
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "List of format commands"
                            }),
                        );
                        props.insert(
                            "error".to_string(),
                            json!({
                                "type": "string",
                                "description": "Error message if command retrieval failed"
                            }),
                        );
                        props
                    }),
                    required: None,
                }),
            },
            Tool {
                name: "get_build_commands".to_string(),
                title: Some("Get Build Commands".to_string()),
                description: Some(
                    "Get build commands configured in the repository's .wkfl.toml config"
                        .to_string(),
                ),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "repo_path".to_string(),
                            json!({
                                "type": "string",
                                "description": "Path to the repository root directory"
                            }),
                        );
                        props
                    }),
                    required: Some(vec!["repo_path".to_string()]),
                },
                output_schema: Some(ToolOutputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "commands".to_string(),
                            json!({
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "List of build commands"
                            }),
                        );
                        props.insert(
                            "error".to_string(),
                            json!({
                                "type": "string",
                                "description": "Error message if command retrieval failed"
                            }),
                        );
                        props
                    }),
                    required: None,
                }),
            },
            Tool {
                name: "get_pr_comments".to_string(),
                title: Some("Get PR Comments".to_string()),
                description: Some(
                    "Get comments from a GitHub pull request in structured JSON format".to_string(),
                ),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "repo_path".to_string(),
                            json!({
                                "type": "string",
                                "description": "Path to the repository root directory"
                            }),
                        );
                        props.insert(
                            "pr_number".to_string(),
                            json!({
                                "type": "integer",
                                "description": "PR number (optional, will auto-detect from current commit if not provided)"
                            }),
                        );
                        props
                    }),
                    required: Some(vec!["repo_path".to_string()]),
                },
                output_schema: Some(ToolOutputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert(
                            "issue_comments".to_string(),
                            json!({
                                "type": "array",
                                "description": "Timeline comments on the PR"
                            }),
                        );
                        props.insert(
                            "review_comments".to_string(),
                            json!({
                                "type": "array",
                                "description": "Review comments on specific lines of code"
                            }),
                        );
                        props.insert(
                            "error".to_string(),
                            json!({
                                "type": "string",
                                "description": "Error message if retrieval failed"
                            }),
                        );
                        props
                    }),
                    required: None,
                }),
            },
        ];

        Self {
            server_info,
            capabilities,
            tools,
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let mut stdout = std::io::stdout();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let message: JSONRPCMessage = match serde_json::from_str(&line) {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("Failed to parse JSON-RPC message: {e}");
                    continue;
                }
            };

            let response = self.handle_message(message);
            if let Some(response) = response {
                let response_json = serde_json::to_string(&response)?;
                writeln!(stdout, "{response_json}")?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    fn handle_message(&self, message: JSONRPCMessage) -> Option<JSONRPCMessage> {
        match message {
            JSONRPCMessage::Request(req) => Some(self.handle_request(req)),
            JSONRPCMessage::Notification(notif) => {
                self.handle_notification(notif);
                None
            }
            _ => None,
        }
    }

    fn handle_request(&self, req: JSONRPCRequest) -> JSONRPCMessage {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "tools/list" => self.handle_list_tools(req),
            "tools/call" => self.handle_call_tool(req),
            "ping" => self.handle_ping(req),
            _ => {
                let error = JSONRPCError {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id: req.id,
                    error: ErrorObject {
                        code: -32601,
                        message: "Method not found".to_string(),
                        data: None,
                    },
                };
                JSONRPCMessage::Error(error)
            }
        }
    }

    fn handle_notification(&self, _notif: JSONRPCNotification) {
        // Handle notifications if needed
    }

    fn handle_initialize(&self, req: JSONRPCRequest) -> JSONRPCMessage {
        let result = json!({
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": self.capabilities,
            "serverInfo": self.server_info,
            "instructions": "This is a wkfl MCP server that provides tools for retrieving test, format, and build commands from repository configuration."
        });

        let response = JSONRPCResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: req.id,
            result,
        };

        JSONRPCMessage::Response(response)
    }

    fn handle_list_tools(&self, req: JSONRPCRequest) -> JSONRPCMessage {
        let result = json!({
            "tools": self.tools
        });

        let response = JSONRPCResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: req.id,
            result,
        };

        JSONRPCMessage::Response(response)
    }

    fn handle_call_tool(&self, req: JSONRPCRequest) -> JSONRPCMessage {
        let params = req.params.unwrap_or(json!({}));
        let tool_name = params["name"].as_str().unwrap_or("");
        let arguments = &params["arguments"];

        let result = match tool_name {
            "get_test_commands" => self.get_test_commands(arguments),
            "get_fmt_commands" => self.get_fmt_commands(arguments),
            "get_build_commands" => self.get_build_commands(arguments),
            "get_pr_comments" => self.get_pr_comments(arguments),
            _ => CallToolResult {
                content: vec![TextContent {
                    content_type: "text".to_string(),
                    text: format!("Unknown tool: {tool_name}"),
                }],
                is_error: Some(true),
                structured_content: Some(json!({
                    "error": format!("Unknown tool: {}", tool_name)
                })),
            },
        };

        let response = JSONRPCResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: req.id,
            result: serde_json::to_value(result).unwrap(),
        };

        JSONRPCMessage::Response(response)
    }

    fn handle_ping(&self, req: JSONRPCRequest) -> JSONRPCMessage {
        let response = JSONRPCResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: req.id,
            result: json!({}),
        };

        JSONRPCMessage::Response(response)
    }

    fn create_error_result(error_msg: &str) -> CallToolResult {
        CallToolResult {
            content: vec![TextContent {
                content_type: "text".to_string(),
                text: error_msg.to_string(),
            }],
            is_error: Some(true),
            structured_content: Some(json!({
                "error": error_msg
            })),
        }
    }

    fn create_success_result(commands: Vec<String>, command_type: &str) -> CallToolResult {
        let message = if commands.is_empty() {
            format!("No {command_type} commands configured in .wkfl.toml")
        } else {
            format!("{command_type} commands retrieved successfully")
        };

        CallToolResult {
            content: vec![TextContent {
                content_type: "text".to_string(),
                text: if commands.is_empty() {
                    message.clone()
                } else {
                    format!("{}\n{}", message, commands.join("\n"))
                },
            }],
            is_error: Some(false),
            structured_content: Some(json!({
                "commands": commands
            })),
        }
    }

    fn extract_repo_path(args: &Value) -> Result<PathBuf, CallToolResult> {
        match args["repo_path"].as_str() {
            Some(path) => Ok(PathBuf::from(path)),
            None => Err(Self::create_error_result(
                "Error: repo_path parameter is required",
            )),
        }
    }

    fn get_commands_from_config<F>(
        &self,
        args: &Value,
        command_type: &str,
        extract_commands: F,
    ) -> CallToolResult
    where
        F: Fn(&crate::config::RepoConfig) -> Vec<String>,
    {
        let repo_path = match Self::extract_repo_path(args) {
            Ok(path) => path,
            Err(error_result) => return error_result,
        };

        match get_repo_config(&repo_path) {
            Ok(config) => {
                let commands = extract_commands(&config);
                Self::create_success_result(commands, command_type)
            }
            Err(e) => Self::create_error_result(&format!("Failed to load repository config: {e}")),
        }
    }

    fn get_test_commands(&self, args: &Value) -> CallToolResult {
        self.get_commands_from_config(args, "test", |config| config.test_commands.clone())
    }

    fn get_fmt_commands(&self, args: &Value) -> CallToolResult {
        self.get_commands_from_config(args, "format", |config| config.fmt_commands.clone())
    }

    fn get_build_commands(&self, args: &Value) -> CallToolResult {
        self.get_commands_from_config(args, "build", |config| config.build_commands.clone())
    }

    fn get_pr_comments(&self, args: &Value) -> CallToolResult {
        let repo_path = match Self::extract_repo_path(args) {
            Ok(path) => path,
            Err(error_result) => return error_result,
        };

        // Extract parameters
        let pr_number = args["pr_number"].as_u64();

        // Create config from repo path - we need this for GitHub tokens
        let config = match get_config() {
            Ok(config) => config,
            Err(e) => return Self::create_error_result(&format!("Failed to load config: {e}")),
        };

        match self.get_pr_comments_impl(&repo_path, pr_number, &config) {
            Ok(comments) => Self::create_pr_comments_result(comments),
            Err(e) => Self::create_error_result(&format!("Failed to get PR comments: {e}")),
        }
    }

    fn get_pr_comments_impl(
        &self,
        repo_path: &PathBuf,
        pr_number: Option<u64>,
        config: &Config,
    ) -> anyhow::Result<PrComments> {
        // Set current directory to repo path
        std::env::set_current_dir(repo_path)?;

        let repo = git::get_repository()?;
        let remote_url = git::get_default_remote_url(&repo)?;
        let (owner, repo_name) = extract_owner_repo_from_url(&remote_url)?;
        let github_client = create_github_client(&remote_url, config)?;

        let pr_num = if let Some(num) = pr_number {
            num
        } else {
            // Find PR for current commit
            let sha = git::get_current_commit_sha(&repo)?;
            let prs = github_client.get_pull_requests_for_commit(&owner, &repo_name, &sha)?;

            if prs.is_empty() {
                anyhow::bail!("No pull request found for current commit {}", sha);
            }

            prs[0].number
        };

        github_client.get_pr_comments(&owner, &repo_name, pr_num)
    }

    fn create_pr_comments_result(comments: PrComments) -> CallToolResult {
        let issue_comments_json: Vec<Value> = comments
            .issue_comments
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "user": {
                        "login": c.user.login,
                        "user_type": c.user.user_type
                    },
                    "body": c.body,
                    "created_at": c.created_at,
                    "updated_at": c.updated_at
                })
            })
            .collect();

        let review_comments_json: Vec<Value> = comments
            .review_comments
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "user": {
                        "login": c.user.login,
                        "user_type": c.user.user_type
                    },
                    "body": c.body,
                    "path": c.path,
                    "line": c.line,
                    "original_line": c.original_line,
                    "side": c.side,
                    "start_side": c.start_side,
                    "diff_hunk": c.diff_hunk,
                    "created_at": c.created_at,
                    "updated_at": c.updated_at
                })
            })
            .collect();

        let total_comments = issue_comments_json.len() + review_comments_json.len();
        let message = format!("Retrieved {total_comments} PR comments successfully");

        CallToolResult {
            content: vec![TextContent {
                content_type: "text".to_string(),
                text: message,
            }],
            is_error: Some(false),
            structured_content: Some(json!({
                "issue_comments": issue_comments_json,
                "review_comments": review_comments_json
            })),
        }
    }
}
