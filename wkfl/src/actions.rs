use git2::Repository;
use log::info;
use std::fs;

use crate::config::Config;
use crate::git;
use crate::prompts::basic_prompt;
use crate::prompts::boolean_prompt;
use crate::prompts::select_prompt;
use crate::repositories::get_repositories_in_directory;
use crate::utils;
use crate::Context;

pub fn start_workflow(
    repo: Repository,
    name: &String,
    ticket: &Option<String>,
    context: &mut Context,
) -> anyhow::Result<()> {
    let user = utils::get_current_user().ok_or(anyhow::anyhow!("Unable to determine user"))?;
    let branch_name = match ticket {
        Some(ticket_key) => format!("{user}/{ticket_key}_{name}"),
        None => format!("{user}/{name}"),
    };

    if git::uses_worktrees(&repo) {
        info!("Creating worktree named '{name}' on branch '{branch_name}'");
        let worktree_path = git::create_worktree(&repo, name, &branch_name)?;
        context
            .shell_actions
            .push(crate::shell_actions::ShellAction::Cd {
                path: worktree_path,
            });
    } else {
        info!("Creating branch '{branch_name}' and checking it out");
        git::switch_branch(&repo, &branch_name, true)?;
    };

    Ok(())
}

pub fn end_workflow(repo: Repository) -> anyhow::Result<()> {
    if repo.is_worktree() {
        anyhow::bail!("For worktree based repos call stop from base of repo with name of worktree");
    } else if repo.is_bare() {
        let workspace_name = basic_prompt("Workspace Name:")?;
        git::remove_worktree(&repo, &workspace_name)?;
    } else if git::on_default_branch(&repo)? {
        let branch_name = basic_prompt("Branch Name:")?;
        git::remove_branch(&repo, &branch_name)?;
    } else {
        git::remove_current_branch(&repo)?;
    }
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
    let repo_paths_strs = repo_paths
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
        .push(crate::shell_actions::ShellAction::Cd { path: repo_path });
    Ok(())
}

pub fn print_repo_debug_info(repo: Repository) -> anyhow::Result<()> {
    info!("worktree: {}", repo.is_worktree());
    info!("bare: {}", repo.is_bare());
    info!("state: {:?}", repo.state());
    info!("path: {:?}", repo.path());
    info!("workdir: {:?}", repo.workdir());
    info!("has_changes: {}", git::has_changes(&repo)?);
    Ok(())
}

pub fn confirm(prompt: &str, default: bool) -> anyhow::Result<()> {
    if !boolean_prompt(prompt, default)? {
        std::process::exit(1);
    }
    Ok(())
}
