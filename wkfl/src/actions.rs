use anyhow::anyhow;
use log::info;
use std::fs;
use std::io;
use url::Url;

use crate::config;
use crate::config::get_repo_config;
use crate::config::Config;
use crate::git;
use crate::git::determine_repo_root_dir;
use crate::llm::perplexity;
use crate::notes::format_note_path;
use crate::notes::note_template;
use crate::notes::DailyNoteSpecifier;
use crate::notes::NoteSpecifier;
use crate::prompts::basic_prompt;
use crate::prompts::boolean_prompt;
use crate::prompts::select_prompt;
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

    let repo_config = get_repo_config(determine_repo_root_dir(&repo))?;
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
    let repo_config = get_repo_config(determine_repo_root_dir(&repo))?;
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

pub fn list_repositories(config: Config) -> anyhow::Result<()> {
    let base_repo_path = config.repositories_directory_path()?;
    let repo_paths = get_repositories_in_directory(&base_repo_path)?;
    for repo_path in repo_paths {
        let relative_repo_path = repo_path.strip_prefix(&base_repo_path)?;
        println!("{}", relative_repo_path.display())
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

fn extract_repo_from_url(repo_url_str: &str) -> anyhow::Result<String> {
    // This isn't perfect, but should be good enough for me and doesn't
    // require writing a regex
    if repo_url_str.starts_with("git@") {
        let (_, repo) = repo_url_str.split_once(':').ok_or(anyhow::anyhow!(
            "Repo url that start with git@ must be in the form 'git@<host>:<repo>'"
        ))?;
        return Ok(repo.to_string());
    }

    let repo_url = Url::parse(repo_url_str)?;
    let repo = repo_url.path();
    if repo.starts_with('/') {
        Ok(repo
            .strip_prefix('/')
            .expect("Checked that it starts with '/'")
            .to_string())
    } else {
        Ok(repo.to_string())
    }
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
    println!("{}", result);
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
    info!("config: {:?}", config);
}

pub fn run_perplexity_query(maybe_query: Option<String>, config: Config) -> anyhow::Result<()> {
    let query = match maybe_query {
        Some(query) => query,
        None => basic_prompt("Query:")?,
    };
    let api_key = config
        .perplexit_api_key
        .ok_or(anyhow!("Missing perplexit_api_key in config"))?;
    let client = perplexity::PerplexityClient::new(api_key);
    let result = client.create_chat_completion(perplexity::ChatCompletionRequest {
        model: "llama-3.1-sonar-large-128k-online".to_string(),
        messages: vec![perplexity::Message {
            role: "user".to_string(),
            content: query,
        }],
        ..perplexity::ChatCompletionRequest::default()
    })?;
    let mut citation_text = String::new();
    if let Some(citations) = result.citations {
        citation_text.push('\n');
        citation_text.push_str(
            &citations
                .iter()
                .enumerate()
                .map(|(i, citation)| format!("[{}] = {}", i, citation))
                .collect::<Vec<String>>()
                .join("\n"),
        );
    }
    println!("{}{}", citation_text, result.choices[0].message.content);
    Ok(())
}
