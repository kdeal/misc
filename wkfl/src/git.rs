use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{self, bail};
use url::Url;

use git2::{
    build::CheckoutBuilder, Branch, BranchType, Error, ErrorCode, Repository, RepositoryState,
    StatusOptions, WorktreeAddOptions,
};
use log::{info, warn};

pub fn get_repository() -> Result<Repository, Error> {
    Repository::open_from_env()
}

pub fn uses_worktrees(repo: &Repository) -> bool {
    repo.is_worktree() || repo.is_bare()
}

pub fn get_default_branch(repo: &Repository) -> anyhow::Result<String> {
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
    branch_name: &str,
) -> anyhow::Result<Branch<'b>> {
    let default_branch_name = get_default_branch(repo)?;

    // Shell out to git for fetch because libgit2 doesn't take into account .ssh/config
    info!("Fetching {} from origin...", &default_branch_name);
    let fetch_output = Command::new("git")
        .args(["fetch", "origin", &default_branch_name])
        .output()?;
    if !fetch_output.status.success() {
        warn!(
            "Fetching {} failed. Output: {}",
            default_branch_name,
            String::from_utf8_lossy(&fetch_output.stderr),
        );
    }

    let origin_banch_ref = format!("origin/{}", &default_branch_name);
    let default_branch = repo.find_branch(origin_banch_ref.as_str(), BranchType::Remote)?;
    let target = repo.find_commit(
        default_branch
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
    name: &str,
    branch_name: &str,
) -> anyhow::Result<PathBuf> {
    let new_branch = create_branch_from_default(repo, branch_name)?;
    let mut worktree_opts = WorktreeAddOptions::new();
    worktree_opts.reference(Some(new_branch.get()));
    let repo_root = determine_repo_root_dir(repo);
    let worktree_path = repo_root.join(name);
    repo.worktree(name, &worktree_path, Some(&worktree_opts))?;
    Ok(worktree_path)
}

pub fn switch_branch(repo: &Repository, branch_name: &str, create: bool) -> anyhow::Result<()> {
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
    Ok(!repo.statuses(Some(&mut status_options))?.is_empty())
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

pub fn on_default_branch(repo: &Repository) -> anyhow::Result<bool> {
    let current_branch = get_current_branch(repo)?;
    let default_branch = get_default_branch(repo)?;
    Ok(current_branch.name()?.unwrap_or("") == default_branch)
}

fn get_current_branch(repo: &Repository) -> anyhow::Result<Branch<'_>> {
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

pub fn get_worktrees(repo: &Repository) -> anyhow::Result<Vec<String>> {
    Ok(repo
        .worktrees()?
        .into_iter()
        .flatten()
        .map(|s| s.to_string())
        .collect())
}

pub fn clone_repo(repo_url: &str, repo_path: &Path) -> anyhow::Result<()> {
    info!("Cloing {} into {}...", repo_url, repo_path.display());
    // Shell out to git for clone because libgit2 doesn't take into account .ssh/config
    let clone_output = Command::new("git")
        .args(["clone", repo_url, &repo_path.to_string_lossy()])
        .output()?;
    if !clone_output.status.success() {
        anyhow::bail!(
            "Failed to clone {}, output: {}",
            repo_url,
            String::from_utf8_lossy(&clone_output.stderr)
        );
    }
    Ok(())
}

pub fn get_current_commit_sha(repo: &Repository) -> anyhow::Result<String> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

pub fn get_default_remote_url(repo: &Repository) -> anyhow::Result<String> {
    let remote = repo.find_remote("origin")?;
    let url = remote
        .url()
        .ok_or_else(|| anyhow::anyhow!("Remote 'origin' has no URL"))?;
    Ok(url.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Right,
    Left,
    Context,
}

impl DiffSide {
    /// Returns the symbol character used in unified diff format for this side.
    ///
    /// - `Right` (added lines) → `'+'`
    /// - `Left` (deleted lines) → `'-'`
    /// - `Context` (unchanged lines) → `' '` (space)
    pub fn to_symbol(self) -> char {
        match self {
            DiffSide::Right => '+',
            DiffSide::Left => '-',
            DiffSide::Context => ' ',
        }
    }
}

impl FromStr for DiffSide {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RIGHT" => Ok(DiffSide::Right),
            "LEFT" => Ok(DiffSide::Left),
            "CONTEXT" => Ok(DiffSide::Context),
            _ => anyhow::bail!(
                "Invalid DiffSide: '{}'. Valid values are: RIGHT, LEFT, CONTEXT",
                s
            ),
        }
    }
}

/// Represents a single line in a diff hunk with its associated metadata.
///
/// # Line Number Handling
/// - For `Left` and `Right` sides: `line` contains the line number, `left_line` is `None`
/// - For `Context` lines: `line` contains the right-side line number, `left_line` contains `Some(left_line_number)`
///
/// This design avoids duplicating context lines while preserving both line numbers.
#[allow(dead_code)]
pub struct DiffLine<'a> {
    /// Which side of the diff this line belongs to
    pub side: DiffSide,
    /// Line number (right-side for context lines, actual side for added/deleted lines)
    pub line: u32,
    /// Left-side line number for context lines, None for added/deleted lines
    pub left_line: Option<u32>,
    /// The actual content of the line (without the +, -, or space prefix)
    pub content: &'a str,
}

/// Parses a unified diff hunk into structured diff lines.
///
/// Takes a diff hunk in unified format (starting with `@@` header) and returns
/// a vector of `DiffLine` structs representing each line in the diff.
///
/// # Format
/// The input should be in unified diff format:
/// ```text
/// @@ -old_start,old_count +new_start,new_count @@
/// -deleted line
/// +added line
///  context line
/// ```
///
/// # Returns
/// - `Ok(Vec<DiffLine>)` - Successfully parsed diff lines
/// - `Err(anyhow::Error)` - Invalid diff format or parsing error
///
/// # Errors
/// - Empty input
/// - Missing or malformed `@@` header
/// - Invalid line numbers in header
/// - Unknown line prefixes (not `+`, `-`, or ` `)
///
/// # Example
/// ```rust
/// let hunk = "@@ -1,3 +1,4 @@\n old line\n-deleted\n+added\n new line";
/// let lines = parse_diff_hunk(hunk)?;
/// ```
pub fn parse_diff_hunk(diff_hunk: &str) -> anyhow::Result<Vec<DiffLine<'_>>> {
    let mut result = Vec::new();
    let lines: Vec<&str> = diff_hunk.lines().collect();

    if lines.is_empty() {
        bail!("Diff hunk is empty");
    }

    // Parse the @@ header to get starting line numbers
    let header = lines[0];
    if !header.starts_with("@@") {
        bail!("Invalid diff hunk: missing @@ header");
    }

    // Extract line numbers from header like "@@ -0,0 +1,494 @@"
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        bail!("Invalid diff hunk header: insufficient parts");
    }

    let left_info = parts[1].trim_start_matches('-');
    let right_info = parts[2].trim_start_matches('+');

    let left_start = left_info
        .split(',')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid left side info in diff header"))?
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("Invalid left line number in diff header"))?;

    let right_start = right_info
        .split(',')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid right side info in diff header"))?
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("Invalid right line number in diff header"))?;

    let mut left_line = left_start;
    let mut right_line = right_start;

    // Process each line after the header
    for line in &lines[1..] {
        if line.is_empty() {
            bail!("Invalid diff line: too short");
        }

        let first_char = line.chars().next().unwrap_or(' ');
        let content = &line[1..]; // Remove the +, -, or space prefix

        match first_char {
            '+' => {
                result.push(DiffLine {
                    side: DiffSide::Right,
                    line: right_line,
                    left_line: None,
                    content,
                });
                right_line += 1;
            }
            '-' => {
                result.push(DiffLine {
                    side: DiffSide::Left,
                    line: left_line,
                    left_line: None,
                    content,
                });
                left_line += 1;
            }
            ' ' => {
                // Context line - store right line in 'line' and left line in 'left_line'
                result.push(DiffLine {
                    side: DiffSide::Context,
                    line: right_line,
                    left_line: Some(left_line),
                    content,
                });
                left_line += 1;
                right_line += 1;
            }
            _ => {
                bail!("Invalid diff line: unknown prefix '{}'", first_char);
            }
        }
    }

    Ok(result)
}

pub fn extract_repo_from_url(repo_url_str: &str) -> anyhow::Result<String> {
    let repo_path = if repo_url_str.starts_with("git@") {
        // Handle SSH URLs: git@github.com:owner/repo.git
        let (_, path) = repo_url_str.split_once(':').ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid SSH URL format: expected 'git@<host>:<path>' but got '{}'",
                repo_url_str
            )
        })?;
        path.to_string()
    } else {
        // Handle HTTPS URLs: https://github.com/owner/repo.git
        let repo_url = Url::parse(repo_url_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse URL '{}': {}", repo_url_str, e))?;

        let path = repo_url.path();
        if path.is_empty() || path == "/" {
            return Err(anyhow::anyhow!(
                "URL '{}' does not contain a repository path",
                repo_url_str
            ));
        }

        // Remove leading slash from path
        path.strip_prefix('/').unwrap_or(path).to_string()
    };

    // Remove .git suffix if present and ensure non-empty result
    let cleaned_path = repo_path.strip_suffix(".git").unwrap_or(&repo_path);
    if cleaned_path.is_empty() {
        return Err(anyhow::anyhow!(
            "Invalid repository path extracted from '{}'",
            repo_url_str
        ));
    }

    Ok(cleaned_path.to_string())
}

/// Parse owner and repository name from a remote URL
pub fn extract_owner_repo_from_url(remote_url: &str) -> anyhow::Result<(String, String)> {
    let owner_repo = extract_repo_from_url(remote_url)?;
    let parts = owner_repo.split_once('/');
    let (owner, repo) = parts.ok_or(anyhow::anyhow!(
        "Unable to parse owner and repo from '{}'",
        owner_repo
    ))?;
    Ok((owner.to_string(), repo.to_string()))
}

/// Determine the host name from a remote URL.
pub fn host_from_remote_url(remote_url: &str) -> anyhow::Result<String> {
    let host = if remote_url.starts_with("git@") {
        let after_at = remote_url
            .split_once('@')
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid git SSH URL format: missing '@' in '{}'",
                    remote_url
                )
            })?
            .1;

        after_at
            .split_once(':')
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid git SSH URL format: missing ':' after host in '{}'",
                    remote_url
                )
            })?
            .0
            .to_string()
    } else {
        let parsed = Url::parse(remote_url)?;
        parsed
            .host_str()
            .ok_or(anyhow::anyhow!(
                "Failed to parse host from '{}'",
                remote_url
            ))?
            .to_string()
    };

    Ok(host)
}
