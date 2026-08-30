use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{config::Config, shell_actions::ShellAction, Context as WkflContext};

const ADJECTIVES: &[&str] = &[
    "amber", "brave", "calm", "clever", "gentle", "lucky", "merry", "quiet", "swift", "vivid",
];
const NOUNS: &[&str] = &[
    "badger", "falcon", "forest", "harbor", "otter", "river", "sparrow", "summit", "willow", "wolf",
];

#[derive(Serialize)]
struct WorkspacesOutput {
    base_directory: String,
    workspaces: Vec<String>,
}

fn repository(config: &Config, requested: Option<&Path>) -> anyhow::Result<(PathBuf, PathBuf)> {
    let base = config.repositories_directory_path()?;
    let path = match requested {
        Some(path) if path.is_absolute() => path.to_owned(),
        Some(path) => base.join(path),
        None => env::current_dir()?
            .ancestors()
            .find(|path| path.join(".jj").exists())
            .map(Path::to_owned)
            .context("current directory is not inside a Jujutsu repository")?,
    };
    let canonical_base = base
        .canonicalize()
        .context("repositories directory does not exist")?;
    let canonical_path = path.canonicalize().context("repository does not exist")?;
    let relative = canonical_path
        .strip_prefix(&canonical_base)
        .context("repository is outside the configured repositories directory")?
        .to_owned();
    if relative.components().count() < 2 {
        bail!("repository must have both a namespace and a name");
    }
    Ok((canonical_path, relative))
}

fn random_name() -> String {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    format!(
        "{}-{}",
        ADJECTIVES[seed % ADJECTIVES.len()],
        NOUNS[(seed / ADJECTIVES.len()) % NOUNS.len()]
    )
}

pub fn create(context: &mut WkflContext, requested_repo: Option<&Path>) -> anyhow::Result<()> {
    let (repo, relative_repo) = repository(&context.config, requested_repo)?;
    let parent = context
        .config
        .workspaces_directory_path()?
        .join(relative_repo);
    fs::create_dir_all(&parent)?;
    let (name, destination) = (0..100)
        .map(|_| random_name())
        .map(|name| (name.clone(), parent.join(name)))
        .find(|(_, destination)| !destination.exists())
        .context("could not generate an unused workspace name")?;
    let output = Command::new("jj")
        .args(["workspace", "add", "--name", &name])
        .arg(&destination)
        .current_dir(repo)
        .output()
        .context("failed to execute 'jj workspace add' - ensure jj is installed")?;
    if !output.status.success() {
        bail!(
            "jj workspace add failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    println!("{}", destination.display());
    context
        .shell_actions
        .push(ShellAction::Cd { path: destination });
    Ok(())
}

fn workspace_directories(base: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    if !base.exists() {
        return Ok(results);
    }
    let mut pending = vec![(base.to_owned(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            if depth >= 2 && path.join(".jj").exists() {
                results.push(path);
            } else {
                pending.push((path, depth + 1));
            }
        }
    }
    results.sort();
    Ok(results)
}

pub fn list(config: &Config, full_path: bool, json: bool) -> anyhow::Result<()> {
    let base = config.workspaces_directory_path()?;
    let workspaces = workspace_directories(&base)?
        .into_iter()
        .map(|path| {
            if full_path {
                Ok(path.display().to_string())
            } else {
                Ok(path.strip_prefix(&base)?.display().to_string())
            }
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&WorkspacesOutput {
                base_directory: base.display().to_string(),
                workspaces
            })?
        );
    } else {
        for workspace in workspaces {
            println!("{workspace}");
        }
    }
    Ok(())
}

pub fn remove(config: &Config, relative: &Path) -> anyhow::Result<()> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        bail!("workspace must be a relative path below the workspace directory");
    }
    if relative.components().count() < 3 {
        bail!("workspace must be specified as namespace/repository/workspace");
    }
    let name = relative.file_name().context("workspace name is missing")?;
    let repo_relative = relative.parent().context("repository path is missing")?;
    let destination = config.workspaces_directory_path()?.join(relative);
    if !destination.join(".jj").exists() {
        bail!("workspace does not exist: {}", destination.display());
    }
    let output = Command::new("jj")
        .args(["workspace", "forget"])
        .arg(name)
        .current_dir(config.repositories_directory_path()?.join(repo_relative))
        .output()
        .context("failed to execute 'jj workspace forget' - ensure jj is installed")?;
    if !output.status.success() {
        bail!(
            "jj workspace forget failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    fs::remove_dir_all(destination)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn finds_only_workspace_leaf_directories() {
        let root = tempdir().unwrap();
        let workspace = root.path().join("owner/repo/calm-otter");
        fs::create_dir_all(workspace.join(".jj")).unwrap();
        fs::create_dir_all(root.path().join("owner/repo/not-a-workspace")).unwrap();
        assert_eq!(workspace_directories(root.path()).unwrap(), vec![workspace]);
    }

    #[test]
    fn generated_names_have_adjective_noun_format() {
        let name = random_name();
        let (adjective, noun) = name.split_once('-').unwrap();
        assert!(ADJECTIVES.contains(&adjective));
        assert!(NOUNS.contains(&noun));
    }
}
