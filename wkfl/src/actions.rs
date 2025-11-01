use anyhow::anyhow;
use log::info;
use std::fs;
use std::io;
use std::io::Write;

use crate::config::get_repo_config;
use crate::config::resolve_secret;
use crate::config::ChatProvider;
use crate::config::Config;
use crate::config::WebChatProvider;
use crate::git::{self, extract_owner_repo_from_url, extract_repo_from_url};
use crate::github::{create_github_client, is_bot_user, IssueComment, PrComments, ReviewComment};
use crate::jira::{create_jira_client, format_jira_date};
use crate::llm;
use crate::llm::anthropic;
use crate::llm::perplexity;
use crate::llm::vertex_ai;
use crate::llm::LlmProvider;
use crate::notes::format_note_path;
use crate::notes::note_template;
use crate::notes::DailyNoteSpecifier;
use crate::notes::NoteSpecifier;
use crate::prompts::basic_prompt;
use crate::prompts::boolean_prompt;
use crate::prompts::select_prompt;
use crate::prompts::Link;
use crate::repositories::get_repositories_in_directory;
use crate::shell_actions::ShellAction;
use crate::utils;
use crate::utils::run_commands;
use crate::Context;

pub fn start_workflow(context: &mut Context) -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    let name = basic_prompt("Name:")?;
    let ticket_str = basic_prompt("Ticket:")?;
    let ticket = if ticket_str.is_empty() {
        None
    } else {
        Some(ticket_str)
    };

    let user = utils::get_current_user().ok_or(anyhow::anyhow!("Unable to determine user"))?;
    let branch_name = match ticket {
        Some(ticket_key) => format!("{user}/{ticket_key}_{name}"),
        None => format!("{user}/{name}"),
    };

    let repo_config = get_repo_config(git::determine_repo_root_dir(&repo))?;
    run_commands(&repo_config.pre_start_commands)?;

    if git::uses_worktrees(&repo) {
        info!("Creating worktree named '{name}' on branch '{branch_name}'");
        let worktree_path = git::create_worktree(&repo, &name, &branch_name)?;
        context.shell_actions.push(ShellAction::Cd {
            path: worktree_path,
        });
    } else {
        info!("Creating branch '{branch_name}' and checking it out");
        git::switch_branch(&repo, &branch_name, true)?;
    };

    run_commands(&repo_config.post_start_commands)?;

    Ok(())
}

pub fn end_workflow() -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    let repo_config = get_repo_config(git::determine_repo_root_dir(&repo))?;
    run_commands(&repo_config.pre_end_commands)?;
    if repo.is_worktree() {
        anyhow::bail!("For worktree based repos call stop from base of repo with name of worktree");
    } else if repo.is_bare() {
        let worktrees = git::get_worktrees(&repo)?;
        let workspace_name = select_prompt("Worktree Name:", &worktrees)?;
        git::remove_worktree(&repo, workspace_name)?;
    } else if git::on_default_branch(&repo)? {
        let branch_name = basic_prompt("Branch Name:")?;
        git::remove_branch(&repo, &branch_name)?;
    } else {
        git::remove_current_branch(&repo)?;
    }
    run_commands(&repo_config.post_end_commands)?;
    Ok(())
}

pub fn list_repositories(config: Config, full_path: bool) -> anyhow::Result<()> {
    let base_repo_path = config.repositories_directory_path()?;
    let repo_paths = get_repositories_in_directory(&base_repo_path)?;
    for repo_path in repo_paths {
        if full_path {
            println!("{}", repo_path.display())
        } else {
            let relative_repo_path = repo_path.strip_prefix(&base_repo_path)?;
            println!("{}", relative_repo_path.display())
        }
    }
    Ok(())
}

pub fn switch_repo(context: &mut Context) -> anyhow::Result<()> {
    let base_repo_path = context.config.repositories_directory_path()?;
    let repo_paths = get_repositories_in_directory(&base_repo_path)?;
    let repo_paths_strs: Vec<String> = repo_paths
        .iter()
        .map(|path| {
            path.strip_prefix(&base_repo_path)
                .expect("All paths should be subpaths of the base_repo_path")
                .to_string_lossy()
                .to_string()
        })
        .collect();
    let repo_name = select_prompt("Repo:", &repo_paths_strs)?;
    let repo_path = base_repo_path.join(repo_name);
    context
        .shell_actions
        .push(ShellAction::Cd { path: repo_path });
    Ok(())
}

pub fn clone_repo(context: &mut Context) -> anyhow::Result<()> {
    let repo_url = basic_prompt("Clone Url:")?;
    let repo = extract_repo_from_url(&repo_url)?;

    let repo_path = context.config.repositories_directory_path()?.join(repo);
    fs::create_dir_all(&repo_path)?;

    let use_worktrees = boolean_prompt("Use worktrees?", false)?;
    if use_worktrees {
        anyhow::bail!("Cloning and using worktrees is unsupported");
    }
    git::clone_repo(&repo_url, &repo_path)?;
    context
        .shell_actions
        .push(ShellAction::Cd { path: repo_path });
    Ok(())
}

/// List all local branches and delete those whose pull request has been merged
pub fn prune_merged_branches(config: &Config) -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    let remote_url = git::get_default_remote_url(&repo)?;

    let (owner, repo_name) = extract_owner_repo_from_url(&remote_url)?;
    let gh_client = create_github_client(&remote_url, config)?;

    // Determine default branch name to avoid deleting it
    let default_branch = git::get_default_branch(&repo)?;
    let branches = repo.branches(Some(git2::BranchType::Local))?;
    let mut branches_to_delete: Vec<String> = vec![];
    for branch_info in branches {
        let (branch, _) = branch_info?;
        let branch_name = branch
            .name()?
            .ok_or(anyhow::anyhow!("Branch name not valid UTF-8"))?;

        if branch.is_head() {
            continue;
        }

        println!("Branch: {branch_name}");
        // Skip the default branch to prevent accidental deletion
        if branch_name == default_branch {
            println!("  Default branch '{branch_name}', skipping");
            continue;
        }
        // Get head commit SHA of this branch
        let reference = branch.get();
        let oid = reference
            .target()
            .ok_or(anyhow::anyhow!("Branch should point to a commit"))?;
        let sha = oid.to_string();
        // Query GitHub for pull requests associated with this commit
        let prs = match gh_client.get_pull_requests_for_commit(&owner, &repo_name, &sha) {
            Ok(prs) => prs,
            Err(e) => {
                println!("  Failed to query GitHub API: {e}");
                continue;
            }
        };
        if prs.is_empty() {
            println!("  No pull request found");
            continue;
        }
        // Check if any PR is merged
        if let Some(pr) = prs.iter().find(|pr| pr.merged_at.is_some()) {
            // Use HTML URL from GitHub response for link
            let pr_text = format!("#{}", pr.number);
            let pr_link = Link::new(&pr_text, &pr.html_url);
            println!("  Pull request {pr_link} merged, deleting branch");
        } else {
            // First PR not merged
            let pr0 = &prs[0];
            let pr_text = format!("#{}", pr0.number);
            let pr_link = Link::new(&pr_text, &pr0.html_url);
            println!("  Pull request {pr_link} not merged");
            continue;
        }
        branches_to_delete.push(branch_name.to_string())
    }
    for branch_name in branches_to_delete {
        git::remove_branch(&repo, &branch_name)?;
    }
    Ok(())
}

pub fn print_repo_debug_info() -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    info!("worktree: {}", repo.is_worktree());
    info!("bare: {}", repo.is_bare());
    info!("state: {:?}", repo.state());
    info!("path: {:?}", repo.path());
    info!("workdir: {:?}", repo.workdir());
    if !repo.is_bare() {
        info!("has_changes: {}", git::has_changes(&repo)?);
    } else {
        info!("has_changes: n/a");
    }
    info!("worktrees: {:?}", git::get_worktrees(&repo)?);
    Ok(())
}

pub fn confirm(prompt: &str, default: bool) -> anyhow::Result<()> {
    if !boolean_prompt(prompt, default)? {
        std::process::exit(1);
    }
    Ok(())
}

pub fn select(prompt: &str) -> anyhow::Result<()> {
    let options: Vec<String> = io::stdin()
        .lines()
        .map_while(Result::ok)
        .filter(|s| !s.is_empty())
        .collect();
    let result = select_prompt(prompt, &options)?;
    println!("{result}");
    Ok(())
}

pub fn open_daily_note(
    daily_note_to_open: DailyNoteSpecifier,
    context: &mut Context,
) -> anyhow::Result<()> {
    open_note(
        NoteSpecifier::Daily {
            day: daily_note_to_open,
        },
        context,
    )
}

pub fn open_topic_note(maybe_name: Option<String>, context: &mut Context) -> anyhow::Result<()> {
    let name = match maybe_name {
        Some(name) => name,
        None => basic_prompt("Topic Name:")?,
    };
    open_note(NoteSpecifier::Topic { name }, context)
}

pub fn open_person_note(maybe_who: Option<String>, context: &mut Context) -> anyhow::Result<()> {
    let who = match maybe_who {
        Some(who) => who,
        None => basic_prompt("Who:")?,
    };
    open_note(NoteSpecifier::Person { who }, context)
}

fn open_note(note_to_open: NoteSpecifier, context: &mut Context) -> anyhow::Result<()> {
    let notes_subpath = format_note_path(&note_to_open);
    let mut notes_file = context.config.notes_directory_path()?;
    notes_file.push(notes_subpath);
    fs::create_dir_all(notes_file.parent().unwrap())?;

    if !notes_file.exists() {
        let template = note_template(&note_to_open);
        fs::write(&notes_file, template)?;
    }

    context
        .shell_actions
        .push(ShellAction::EditFile { path: notes_file });
    Ok(())
}

pub fn print_config(config: Config) {
    info!("config: {config:?}");
}

pub fn run_perplexity_query(maybe_query: Option<String>, config: Config) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let client = perplexity::PerplexityClient::from_config(config)?;
    let result = client.create_chat_completion(perplexity::PerplexityRequest {
        messages: vec![llm::Message {
            role: llm::Role::User,
            content: query,
        }],
        ..perplexity::PerplexityRequest::default()
    })?;
    let mut citation_text = String::new();
    if let Some(citations) = result.citations {
        citation_text.push('\n');
        citation_text.push_str(
            &citations
                .iter()
                .enumerate()
                .map(|(i, citation)| format!("[{i}] = {citation}"))
                .collect::<Vec<String>>()
                .join("\n"),
        );
    }
    println!("{}{}", citation_text, result.choices[0].message.content);
    Ok(())
}

pub fn stream_perplexity_query(maybe_query: Option<String>, config: Config) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let client = perplexity::PerplexityClient::from_config(config)?;
    let result = client.stream_chat_completion(perplexity::PerplexityRequest {
        messages: vec![llm::Message {
            role: llm::Role::User,
            content: query,
        }],
        stream: Some(true),
        ..perplexity::PerplexityRequest::default()
    })?;
    let mut citation_text = String::new();
    for partial_result in result {
        let part = partial_result?;
        if citation_text.is_empty() {
            if let Some(citations) = part.citations {
                citation_text.push('\n');
                citation_text.push_str(
                    &citations
                        .iter()
                        .enumerate()
                        .map(|(i, citation)| format!("[{i}] = {citation}"))
                        .collect::<Vec<String>>()
                        .join("\n"),
                );
            }
        }
        print!("{}", part.choices[0].delta.content);
        // This is a nice to have, so ignore any errors it returns
        std::io::stdout().flush().unwrap_or_default();
    }
    println!("{citation_text}");
    Ok(())
}

pub fn run_anthropic_query(maybe_query: Option<String>, config: Config) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let api_key_raw = config
        .anthropic_api_key
        .ok_or(anyhow!("Missing anthropic_api_key in config"))?;
    let api_key = resolve_secret(&api_key_raw)?;
    let client = anthropic::AnthropicClient::new(api_key);
    let result = client.create_chat_completion(anthropic::AnthropicRequest {
        messages: vec![llm::Message {
            role: llm::Role::User,
            content: query,
        }],
        max_tokens: 1024,
        ..anthropic::AnthropicRequest::default()
    })?;
    let content = result
        .content
        .into_iter()
        .next()
        .expect("It should always return some content");

    if let anthropic::ContentBlock::Text { text } = content {
        println!("{text}");
    }

    Ok(())
}

pub fn stream_anthropic_query(maybe_query: Option<String>, config: Config) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let api_key_raw = config
        .anthropic_api_key
        .ok_or(anyhow!("Missing anthropic_api_key in config"))?;
    let api_key = resolve_secret(&api_key_raw)?;
    let client = anthropic::AnthropicClient::new(api_key);
    let stream = client.stream_chat_completion(anthropic::AnthropicRequest {
        messages: vec![llm::Message {
            role: llm::Role::User,
            content: query,
        }],
        max_tokens: 1024,
        stream: Some(true),
        ..anthropic::AnthropicRequest::default()
    })?;

    for event in stream {
        let event = event?;

        match event {
            anthropic::StreamEvent::ContentBlockStart { content_block, .. } => {
                match content_block {
                    anthropic::ContentBlock::Text { text } => {
                        print!("{text}");
                        // Flush stdout to see incremental updates
                        std::io::stdout().flush().unwrap_or_default();
                    }
                    anthropic::ContentBlock::Thinking { thinking } => {
                        // Optionally print thinking output
                        print!("\n[Thinking] {thinking}");
                        std::io::stdout().flush().unwrap_or_default();
                    }
                    _ => {} // Ignore other delta types
                }
            }
            anthropic::StreamEvent::ContentBlockDelta { delta, .. } => {
                match delta {
                    anthropic::ContentDelta::TextDelta { text } => {
                        print!("{text}");
                        // Flush stdout to see incremental updates
                        std::io::stdout().flush().unwrap_or_default();
                    }
                    anthropic::ContentDelta::ThinkingDelta { thinking } => {
                        // Optionally print thinking output
                        print!("\n[Thinking] {thinking}");
                        std::io::stdout().flush().unwrap_or_default();
                    }
                    _ => {} // Ignore other delta types
                }
            }
            anthropic::StreamEvent::Error { error } => {
                eprintln!("Error: {} - {}", error.error_type, error.message);
            }
            _ => {} // Ignore other event types
        }
    }

    println!(); // Add a newline at the end
    Ok(())
}

fn print_grounding_chunks(grounding_chunks: &[vertex_ai::GroundingChunk]) {
    if grounding_chunks.is_empty() {
        return;
    }
    println!(); // Add empty line before citations
    grounding_chunks
        .iter()
        .enumerate()
        .for_each(|(i, grounding_chunk)| {
            println!(
                "[{}] = {}",
                i,
                Link::new(&grounding_chunk.web.title, &grounding_chunk.web.uri)
            );
        });
}

pub fn stream_vertex_ai_query(
    maybe_query: Option<String>,
    enable_search: bool,
    config: Config,
) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let client = vertex_ai::VertexAiClient::from_config(config)?;
    let mut request = vertex_ai::VertexAiRequest {
        contents: vec![vertex_ai::Content {
            role: Some(vertex_ai::Role::User),
            parts: vec![vertex_ai::Part { text: query }],
        }],
        ..vertex_ai::VertexAiRequest::default()
    };
    if enable_search {
        request.tools = Some(vec![vertex_ai::GoogleSearchTool::default()]);
    }

    let stream = client.stream_chat_completion(request, vertex_ai::VertexAiModel::default())?;

    let mut last_grounding_chunks = Vec::new();

    for event_result in stream {
        let event = event_result?;

        let candidate = &event.candidates[0];

        print!("{}", candidate.content.parts[0].text);
        // Flush stdout to see incremental updates
        std::io::stdout().flush().unwrap_or_default();

        if let Some(grounding_metadata) = &candidate.grounding_metadata {
            if !grounding_metadata.grounding_chunks.is_empty() {
                last_grounding_chunks = grounding_metadata.grounding_chunks.clone();
            }
        }
    }

    println!();

    // Process citations from the saved grounding chunks
    print_grounding_chunks(&last_grounding_chunks);

    Ok(())
}

pub fn run_vertex_ai_query(
    maybe_query: Option<String>,
    enable_search: bool,
    config: Config,
) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let client = vertex_ai::VertexAiClient::from_config(config)?;
    let mut request = vertex_ai::VertexAiRequest {
        contents: vec![vertex_ai::Content {
            role: Some(vertex_ai::Role::User),
            parts: vec![vertex_ai::Part { text: query }],
        }],
        ..vertex_ai::VertexAiRequest::default()
    };
    if enable_search {
        request.tools = Some(vec![vertex_ai::GoogleSearchTool::default()]);
    }
    let result = client.create_chat_completion(request, vertex_ai::VertexAiModel::default())?;
    let candidate = &result.candidates[0];
    if let Some(grounding_metadata) = &candidate.grounding_metadata {
        print_grounding_chunks(&grounding_metadata.grounding_chunks);
    }
    println!("{}", candidate.content.parts[0].text);
    Ok(())
}

fn number_to_superscript(number: &u8) -> String {
    const SUPERSCRIPT_DIGITS: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
    number
        .to_string()
        .chars()
        .map(|c| SUPERSCRIPT_DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

fn format_citation_indices(indices: &[u8]) -> String {
    indices
        .iter()
        .map(number_to_superscript)
        .collect::<Vec<String>>()
        .join("˒")
}

pub fn run_web_chat(
    maybe_query: Option<String>,
    model_type: llm::ModelType,
    model_provider: Option<WebChatProvider>,
    config: Config,
) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let client_provider = match model_provider {
        Some(provider) => provider,
        None => config
            .get_web_chat_provider()
            .expect("No provider configured that supports web chat"),
    };
    let client = client_provider.create_client(config)?;
    let result =
        client.create_grounded_chat_completion(llm::GroundedChatRequest { query, model_type })?;

    let mut last_end = 0;
    for support in result.citations.supports.iter() {
        let str_to_print = result.message.content[last_end..support.end_index].to_string();
        print!(
            "{}{}",
            str_to_print,
            format_citation_indices(&support.source_indices)
        );
        last_end = support.end_index;
    }
    if last_end != result.message.content.len() {
        let str_to_print = result.message.content[last_end..].to_string();
        print!("{str_to_print}");
    }
    println!("\n");

    for citation in result.citations.sources.iter() {
        print!(" {:}", Link::new(&citation.title, &citation.uri));
    }
    println!();

    Ok(())
}

pub fn run_chat(
    maybe_query: Option<String>,
    model_type: llm::ModelType,
    model_provider: Option<ChatProvider>,
    config: Config,
) -> anyhow::Result<()> {
    let query = llm::get_query(maybe_query)?;
    let client_provider = match model_provider {
        Some(provider) => provider,
        None => config
            .get_chat_provider()
            .expect("No provider configured that supports web chat"),
    };
    let client = client_provider.create_client(config)?;
    let result = client.create_message(llm::ChatRequest { query, model_type })?;

    println!("{}", result.message.content);
    Ok(())
}

pub fn run_test_commands(_context: &mut Context, list: bool) -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    let repo_root = git::determine_repo_root_dir(&repo);
    let repo_config = get_repo_config(repo_root)?;

    if repo_config.test_commands.is_empty() {
        println!("No test commands configured in repository config");
        return Ok(());
    }

    if list {
        for command in &repo_config.test_commands {
            println!("{command}");
        }
        return Ok(());
    }

    utils::run_commands_with_output(&repo_config.test_commands)?;
    Ok(())
}

pub fn run_fmt_commands(_context: &mut Context, list: bool) -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    let repo_root = git::determine_repo_root_dir(&repo);
    let repo_config = get_repo_config(repo_root)?;

    if repo_config.fmt_commands.is_empty() {
        println!("No fmt commands configured in repository config");
        return Ok(());
    }

    if list {
        for command in &repo_config.fmt_commands {
            println!("{command}");
        }
        return Ok(());
    }

    utils::run_commands_with_output(&repo_config.fmt_commands)?;
    Ok(())
}

pub fn run_build_commands(_context: &mut Context, list: bool) -> anyhow::Result<()> {
    let repo = git::get_repository()?;
    let repo_root = git::determine_repo_root_dir(&repo);
    let repo_config = get_repo_config(repo_root)?;

    if repo_config.build_commands.is_empty() {
        println!("No build commands configured in repository config");
        return Ok(());
    }

    if list {
        for command in &repo_config.build_commands {
            println!("{command}");
        }
        return Ok(());
    }

    utils::run_commands_with_output(&repo_config.build_commands)?;
    Ok(())
}

pub fn get_pull_request_for_commit(
    commit_sha: Option<String>,
    config: &Config,
) -> anyhow::Result<()> {
    let repo = git::get_repository()?;

    let sha = if let Some(sha) = commit_sha {
        sha
    } else {
        git::get_current_commit_sha(&repo)?
    };

    let remote_url = git::get_default_remote_url(&repo)?;
    let (owner, repo_name) = extract_owner_repo_from_url(&remote_url)?;

    let github_client = create_github_client(&remote_url, config)?;
    let pull_requests = github_client.get_pull_requests_for_commit(&owner, &repo_name, &sha)?;

    if pull_requests.is_empty() {
        println!("No pull request found for commit {sha}");
    } else {
        for pr in pull_requests {
            let status = if pr.merged_at.is_some() {
                "merged"
            } else {
                "open"
            };
            println!("PR #{} ({}): {}", pr.number, status, pr.html_url);
        }
    }

    Ok(())
}

pub fn get_pr_comments(
    pr_number: Option<u64>,
    filter_timeline: bool,
    filter_bots: bool,
    filter_diff: bool,
    config: &Config,
) -> anyhow::Result<()> {
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

    let comments = github_client.get_pr_comments(&owner, &repo_name, pr_num)?;

    print_comments_markdown(&comments, filter_timeline, filter_bots, filter_diff)?;

    Ok(())
}

fn format_context_lines(diff_lines: &[&git::DiffLine]) -> String {
    if diff_lines.is_empty() {
        return String::new();
    }

    // Calculate max line number for formatting
    let max_line = diff_lines.iter().map(|d| d.line).max().unwrap_or(0);

    let width = max_line.to_string().len();

    diff_lines
        .iter()
        .map(|diff_line| {
            format!(
                "{:width$} | {} {}",
                diff_line.line,
                diff_line.side.to_symbol(),
                diff_line.content,
                width = width
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn extract_code_context_fallback(
    parsed_diff: Vec<git::DiffLine>,
    start_line: Option<u32>,
    end_line_pos: u32,
) -> String {
    let range_size = std::cmp::max(
        start_line
            .unwrap_or(end_line_pos)
            .saturating_sub(end_line_pos),
        4,
    );
    let first_line = parsed_diff.len().saturating_sub(range_size as usize);

    let context_lines: Vec<&git::DiffLine> =
        parsed_diff[first_line..parsed_diff.len()].iter().collect();

    format_context_lines(&context_lines)
}

fn extract_code_context(
    diff_hunk: &str,
    start_line: Option<u32>,
    start_side: &Option<String>,
    line: Option<u32>,
    side: &str,
) -> anyhow::Result<String> {
    if line.is_none() {
        return Ok(String::new());
    }

    let end_line_pos = line.unwrap();
    let end_line_side = side.parse::<git::DiffSide>()?;
    let parsed_diff = git::parse_diff_hunk(diff_hunk)?;
    let last_line = parsed_diff.iter().last().map(|d| d.line).unwrap_or(0);
    if end_line_pos > last_line {
        return Ok(extract_code_context_fallback(
            parsed_diff,
            start_line,
            end_line_pos,
        ));
    }

    let start_line_pos = start_line.unwrap_or(0);
    let start_line_side = start_side
        .as_ref()
        .map(|s| s.parse::<git::DiffSide>())
        .unwrap_or(Ok(git::DiffSide::Context))?;

    let size = if start_line_pos == 0 {
        4
    } else {
        end_line_pos - start_line_pos
    };

    let mut context_lines = Vec::with_capacity(size as usize);
    let mut in_context = false;
    for diff_line in parsed_diff.iter().rev() {
        if diff_line.line == end_line_pos && diff_line.side == end_line_side {
            in_context = true;
        }
        if in_context {
            context_lines.push(diff_line);
            if diff_line.line == start_line_pos && diff_line.side == start_line_side {
                break;
            }
            if start_line_pos == 0 && context_lines.len() >= 4 {
                break;
            }
        }
    }

    context_lines.reverse();
    Ok(format_context_lines(&context_lines))
}

fn print_comments_markdown(
    comments: &PrComments,
    filter_timeline: bool,
    filter_bots: bool,
    filter_diff: bool,
) -> anyhow::Result<()> {
    println!("# PR Comments\n");

    // Print timeline comments
    if !filter_timeline {
        let mut timeline_comments: Vec<&IssueComment> = comments.issue_comments.iter().collect();

        if filter_bots {
            timeline_comments.retain(|c| !is_bot_user(&c.user.login, &c.user.user_type));
        }

        if !timeline_comments.is_empty() {
            println!("## Timeline Comments\n");
            for comment in timeline_comments {
                println!("### @{} - {}", comment.user.login, comment.created_at);
                println!("{}\n", comment.body);
                println!("---\n");
            }
        }
    }

    // Print diff comments
    if !filter_diff {
        let mut diff_comments: Vec<&ReviewComment> = comments.review_comments.iter().collect();

        if filter_bots {
            diff_comments.retain(|c| !is_bot_user(&c.user.login, &c.user.user_type));
        }

        if !diff_comments.is_empty() {
            println!("## Review Comments\n");
            for comment in diff_comments {
                let resolved_marker = match comment.is_resolved {
                    Some(true) => " [resolved]",
                    Some(false) => " [unresolved]",
                    None => "",
                };
                println!(
                    "### @{} - {} ({}:{}){}",
                    comment.user.login,
                    comment.created_at,
                    comment.path,
                    comment.original_line.unwrap_or(0),
                    resolved_marker
                );

                println!("\n**Code Context:**");
                println!("```");
                println!(
                    "{}",
                    extract_code_context(
                        &comment.diff_hunk,
                        comment.original_start_line,
                        &comment.start_side,
                        comment.original_line,
                        &comment.side,
                    )?
                );
                println!("```\n");

                println!("**Comment:**");
                println!("{}\n", comment.body);
                println!("---\n");
            }
        }
    }

    Ok(())
}

pub fn get_jira_issue(issue_key: &str, config: &Config) -> anyhow::Result<()> {
    let client = create_jira_client(config)?;
    let issue = client.get_issue(issue_key)?;

    println!("Issue: {}", issue.key);
    println!("Summary: {}", issue.fields.summary);
    println!("Status: {}", issue.fields.status.name);
    println!(
        "Project: {} ({})",
        issue.fields.project.name, issue.fields.project.key
    );
    println!("Type: {}", issue.fields.issuetype.name);

    if let Some(assignee) = &issue.fields.assignee {
        println!("Assignee: {}", assignee.display_name);
    } else {
        println!("Assignee: Unassigned");
    }

    if let Some(reporter) = &issue.fields.reporter {
        println!("Reporter: {}", reporter.display_name);
    }

    if let Some(priority) = &issue.fields.priority {
        println!("Priority: {}", priority.name);
    }

    println!("Created: {}", format_jira_date(&issue.fields.created));
    println!("Updated: {}", format_jira_date(&issue.fields.updated));

    if let Some(description) = &issue.fields.description {
        println!("\nDescription:");
        println!("{}", description.to_markdown());
    }

    if !issue.fields.comment.comments.is_empty() {
        println!("\nComments ({}):", issue.fields.comment.comments.len());
        for comment in &issue.fields.comment.comments {
            println!(
                "\n--- {} ({}) ---",
                comment.author.display_name,
                format_jira_date(&comment.created)
            );
            let comment_text = comment.body.to_markdown();
            if !comment_text.trim().is_empty() {
                println!("{}", comment_text.trim());
            } else {
                println!("(No text content)");
            }
        }
    }

    Ok(())
}

fn display_jira_issues(issues: &[crate::jira::Issue]) {
    if issues.is_empty() {
        println!("No issues found matching the query.");
        return;
    }

    println!("Found {} issue(s):\n", issues.len());
    println!(
        "{:<15} {:<50} {:<15} {:<20}",
        "Key", "Summary", "Status", "Assignee"
    );
    println!("{}", "-".repeat(100));

    for issue in issues {
        let assignee = issue
            .fields
            .assignee
            .as_ref()
            .map(|a| a.display_name.as_str())
            .unwrap_or("Unassigned");

        let summary = if issue.fields.summary.chars().count() > 50 {
            let truncated: String = issue.fields.summary.chars().take(49).collect();
            format!("{}…", truncated)
        } else {
            issue.fields.summary.clone()
        };

        println!(
            "{:<15} {:<50} {:<15} {:<20}",
            issue.key, summary, issue.fields.status.name, assignee
        );
    }
}

pub fn search_jira_issues(
    jql: &str,
    max_results: Option<u32>,
    config: &Config,
) -> anyhow::Result<()> {
    let client = create_jira_client(config)?;
    let issues = client.search_issues(jql, max_results)?;

    display_jira_issues(&issues);
    Ok(())
}

pub fn search_jira_issues_by_filter(
    filter_id: Option<String>,
    max_results: Option<u32>,
    config: &Config,
) -> anyhow::Result<()> {
    let client = create_jira_client(config)?;

    let filter = match filter_id {
        Some(id) => {
            info!("Using filter ID: {}", id);
            // Use the provided filter ID directly
            client.get_filter(&id)?
        }
        None => {
            info!("Loading favourite filters for interactive selection");
            // Show interactive selection of favorite filters
            let filters = client.get_favourite_filters()?;

            if filters.is_empty() {
                println!(
                    "No favourite filters found. You can add filters to your favourites in Jira."
                );
                return Ok(());
            }

            info!("Found {} favourite filters", filters.len());

            let filter_options: Vec<String> = filters.iter().map(|f| f.display_name()).collect();

            let selected = select_prompt("Select a filter:", &filter_options)?;

            // Find the selected filter by matching the display name
            filters
                .into_iter()
                .find(|f| f.display_name() == selected)
                .ok_or_else(|| anyhow::anyhow!("Selected filter not found"))?
        }
    };

    info!(
        "Executing search with filter: {} (ID: {})",
        filter.name, filter.id
    );
    println!("Using filter: {} ({})", filter.name, filter.jql);
    println!();

    // Search for issues using the filter's JQL
    let issues = client.search_issues(&filter.jql, max_results)?;
    display_jira_issues(&issues);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::extract_owner_repo_from_url;
    use super::extract_repo_from_url;
    use crate::git::host_from_remote_url;

    #[test]
    fn test_host_from_remote_url_github_com() {
        let url = "git@github.com:owner/repo.git";
        let host = host_from_remote_url(url).unwrap();
        assert_eq!(host, "github.com");
    }

    #[test]
    fn test_host_from_remote_url_enterprise() {
        let url = "git@github.example.com:owner/repo.git";
        let host = host_from_remote_url(url).unwrap();
        assert_eq!(host, "github.example.com");
    }

    #[test]
    fn test_host_from_remote_url_https_url() {
        let url = "https://github.com/owner/repo.git";
        let host = host_from_remote_url(url).unwrap();
        assert_eq!(host, "github.com");
    }

    #[test]
    fn test_host_from_remote_url_http_url_enterprise() {
        let url = "http://github.example.com/owner/repo";
        let host = host_from_remote_url(url).unwrap();
        assert_eq!(host, "github.example.com");
    }

    #[test]
    fn test_extract_repo_from_url_git_ssh() {
        let url = "git@github.com:owner/repo.git";
        let repo = extract_repo_from_url(url).unwrap();
        assert_eq!(repo, "owner/repo");
    }

    #[test]
    fn test_extract_repo_from_url_https() {
        let url = "https://github.com/owner/repo.git";
        let repo = extract_repo_from_url(url).unwrap();
        assert_eq!(repo, "owner/repo");
    }

    #[test]
    fn test_extract_owner_repo_basic() {
        let url = "git@github.com:alice/project.git";
        let (owner, repo) = extract_owner_repo_from_url(url).unwrap();
        assert_eq!(owner, "alice");
        assert_eq!(repo, "project");
    }

    #[test]
    fn test_extract_owner_repo_http() {
        let url = "https://github.com/bob/tool";
        let (owner, repo) = extract_owner_repo_from_url(url).unwrap();
        assert_eq!(owner, "bob");
        assert_eq!(repo, "tool");
    }

    #[test]
    fn test_extract_owner_repo_error() {
        // Missing slash
        let url = "https://github.com/onlyowner";
        assert!(extract_owner_repo_from_url(url).is_err());
    }
}
