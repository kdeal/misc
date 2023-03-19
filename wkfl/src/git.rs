use std::{fs, path::Path};

use anyhow::{self, bail, Context};

use git2::{
    build::CheckoutBuilder, Branch, BranchType, Error, ErrorCode, Repository, RepositoryState,
    StatusOptions, WorktreeAddOptions,
};
use log::info;

pub fn get_repository() -> Result<Repository, Error> {
    Repository::open_from_env()
}

pub fn uses_worktrees(repo: &Repository) -> bool {
    repo.is_worktree() || repo.is_bare()
}

fn get_default_branch(repo: &Repository) -> anyhow::Result<String> {
    let head_ref = repo.find_reference("refs/remotes/origin/HEAD")?;
    let default_branch_ref = head_ref.symbolic_target().ok_or(anyhow::anyhow!(
        "origin/HEAD doesn't point to branch, can't determine default branch"
    ))?;
    let default_branch_name = default_branch_ref
        .strip_prefix("refs/remotes/origin/")
        .ok_or(anyhow::anyhow!(
            "origin/HEAD doesn't point to a branch in remotes_origin."
        ))?;
    Ok(String::from(default_branch_name))
}

fn create_branch_from_default<'b>(
    repo: &'b Repository,
    branch_name: &String,
) -> anyhow::Result<Branch<'b>> {
    let mut remote = repo
        .find_remote("origin")
        .context("Error getting origin remote. Does 'origin' remote exist?")?;

    remote.connect(git2::Direction::Fetch)?;
    let default_branch_buf = remote.default_branch()?;
    let default_branch_full_name = default_branch_buf
        .as_str()
        .expect("Repo should have default branch");
    let default_branch_name = default_branch_full_name
        .strip_prefix("refs/heads/")
        .expect("default branch should start with refs/heads/");

    remote
        .fetch(&[default_branch_name], None, None)
        .context("Error when fetching default branch from origin remote")?;

    let origin_banch_ref = format!("origin/{}", default_branch_name);
    let master_branch = repo.find_branch(origin_banch_ref.as_str(), BranchType::Remote)?;
    let target = repo.find_commit(
        master_branch
            .get()
            .target()
            .expect("Branch should point to a commit"),
    )?;
    repo.branch(branch_name, &target, false).map_err(|e| {
        let context = if e.code() == ErrorCode::Exists {
            "Branch already exists with this name. Use a different name"
        } else {
            "Failed to create branch"
        };
        anyhow::anyhow!(e).context(context)
    })
}

pub fn determine_repo_root_dir(repo: &Repository) -> &Path {
    if repo.is_bare() {
        // if bare repo assume repo uses a worktree setup, so the path is
        // the .git dir in the base of the repo
        repo.path().parent().expect(".git dir shoud have a parent")
    } else if repo.is_worktree() {
        // repo_path is <base_dir>/.git/worktrees/<worktree_name>/
        repo.path()
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .expect("worktree should be nested in .git dir twice")
    } else {
        repo.workdir()
            .expect("Repo isn't bare, so it should have a workdir")
    }
}

pub fn create_worktree(
    repo: &Repository,
    name: &String,
    branch_name: &String,
) -> anyhow::Result<()> {
    let new_branch = create_branch_from_default(repo, branch_name)?;
    let mut worktree_opts = WorktreeAddOptions::new();
    worktree_opts.reference(Some(new_branch.get()));
    let repo_root = determine_repo_root_dir(repo);
    repo.worktree(name, repo_root.join(name).as_path(), Some(&worktree_opts))?;
    Ok(())
}

pub fn switch_branch(repo: &Repository, branch_name: &String, create: bool) -> anyhow::Result<()> {
    let repo_state = repo.state();
    if repo_state != RepositoryState::Clean {
        anyhow::bail!(
            "Repository in {:?} state. Must be in a clean state to switch branches",
            repo_state
        )
    }
    let branch = if create {
        create_branch_from_default(repo, branch_name)?
    } else {
        repo.find_branch(branch_name, BranchType::Local)?
    };
    repo.set_head(branch.get().name().expect("Branch should have a name"))?;
    // Default is safe checkout
    repo.checkout_head(Some(&mut CheckoutBuilder::new()))?;
    Ok(())
}

pub fn has_changes(repo: &Repository) -> anyhow::Result<bool> {
    let mut status_options = StatusOptions::new();
    status_options.include_ignored(false);
    status_options.include_untracked(true);
    Ok(repo.statuses(Some(&mut status_options))?.len() > 0)
}

pub fn remove_worktree(repo: &Repository, worktree_name: &str) -> anyhow::Result<()> {
    let worktree = repo.find_worktree(worktree_name)?;
    let worktree_repo = Repository::open(worktree.path())?;
    let mut cur_branch = get_current_branch(&worktree_repo)?;
    if has_changes(&worktree_repo)? {
        bail!("Wortree has changes can't delete");
    } else {
        fs::remove_dir_all(worktree.path())?;
    }
    worktree.prune(None)?;
    cur_branch.delete()?;
    Ok(())
}

fn get_current_branch(repo: &Repository) -> anyhow::Result<Branch> {
    if repo.head_detached().unwrap_or(false) {
        bail!("Currently no branch, repo head is detached");
    }

    let head_ref = repo.head()?;
    if !head_ref.is_branch() {
        bail!("Currently no branch, repo head is {:?}", head_ref.kind());
    }
    let branch_name = head_ref
        .shorthand()
        .ok_or(anyhow::anyhow!("Branch name is not utf-8"))?;
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    Ok(branch)
}

pub fn remove_current_branch(repo: &Repository) -> anyhow::Result<()> {
    let mut current_branch = get_current_branch(repo)?;
    let default_branch = get_default_branch(repo)?;
    info!("Switching to the  dafault branch: '{default_branch}'");
    switch_branch(repo, &default_branch, false)?;
    current_branch.delete()?;
    Ok(())
}

pub fn remove_branch(repo: &Repository, branch_name: &str) -> anyhow::Result<()> {
    let mut branch = repo.find_branch(branch_name, BranchType::Local)?;
    if branch.is_head() {
        return remove_current_branch(repo);
    }
    branch.delete()?;
    Ok(())
}
