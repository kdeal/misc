use git2::Repository;
use log::info;

use crate::git;
use crate::utils;

pub fn start_workflow(
    repo: Repository,
    name: &String,
    ticket: &Option<String>,
) -> anyhow::Result<()> {
    let user = utils::get_current_user().ok_or(anyhow::anyhow!("Unable to determine user"))?;
    let branch_name = match ticket {
        Some(ticket_key) => format!("{user}/{ticket_key}_{name}"),
        None => format!("{user}/{name}"),
    };

    if git::uses_worktrees(&repo) {
        info!("Creating worktree named '{name}' on branch '{branch_name}'");
        git::create_worktree(&repo, name, &branch_name)
    } else {
        info!("Creating branch '{branch_name}' and checking it out");
        git::switch_branch(&repo, &branch_name, true)
    }?;

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

pub fn print_repo_debug_info(repo: Repository) -> anyhow::Result<()> {
    info!("worktree: {}", repo.is_worktree());
    info!("bare: {}", repo.is_bare());
    info!("state: {:?}", repo.state());
    info!("path: {:?}", repo.path());
    info!("workdir: {:?}", repo.workdir());
    info!("has_changes: {}", git::has_changes(&repo)?);
    Ok(())
}