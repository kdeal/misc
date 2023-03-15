use std::path::Path;

use anyhow::{self, Context};

use git2::{
    build::CheckoutBuilder, Branch, BranchType, Error, ErrorCode, Repository, RepositoryState,
    WorktreeAddOptions,
};
use log::info;

pub fn get_repository() -> Result<Repository, Error> {
    Repository::open_from_env()
}

pub fn uses_worktrees(repo: &Repository) -> bool {
    repo.is_worktree() || repo.is_bare()
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

pub fn switch_branch(repo: &Repository, branch_name: &String) -> anyhow::Result<()> {
    let repo_state = repo.state();
    if repo_state != RepositoryState::Clean {
        anyhow::bail!(
            "Repository in {:?} state. Must be in a clean state to create new branch",
            repo_state
        )
    }
    let new_branch = create_branch_from_default(repo, branch_name)?;
    info!("branch name: {:?}", new_branch.name()?);
    repo.set_head(
        new_branch
            .get()
            .name()
            .expect("Newly created branch should have a name"),
    )?;
    // Default is safe checkout
    repo.checkout_head(Some(&mut CheckoutBuilder::new()))?;
    Ok(())
}
