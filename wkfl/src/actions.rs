use git2::Repository;
use log::{info, warn};

use crate::config::Config;
use crate::git;
use crate::prompts::basic_prompt;
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
        context.shell_actions.push(crate::shell_actions::ShellAction::Cd { path: worktree_path });
    } else {
        info!("Creating branch '{branch_name}' and checking it out");
        git::switch_branch(&repo, &branch_name, true)?;
    };

    Ok(())
}

pub fn end_workflow(repo: Repository, name: &Option<String>) -> anyhow::Result<()> {
    if repo.is_worktree() {
        anyhow::bail!("For worktree based repos call stop from base of repo with name of worktree");
    } else if repo.is_bare() {
        let workspace_name = name.clone().ok_or(anyhow::anyhow!(
            "Must specify a name in worktree based repo"
        ))?;
        git::remove_worktree(&repo, &workspace_name)?;
    } else {
        match name {
            Some(branch_name) => git::remove_branch(&repo, branch_name)?,
            None => git::remove_current_branch(&repo)?,
        }
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
    let repo_name = basic_prompt("Repo name:")?;
    let base_repo_path = context.config.repositories_directory_path()?;
    let repo_paths = get_repositories_in_directory(&base_repo_path)?;
    let repo_match = repo_paths
        .into_iter()
        .find(|repo_path| repo_path.ends_with(&repo_name));
    match repo_match {
        Some(repo_path) => context
            .shell_actions
            .push(crate::shell_actions::ShellAction::Cd { path: repo_path }),
        None => warn!("Unable to find repo named: {}", repo_name),
    }
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
