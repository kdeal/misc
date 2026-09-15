use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;

const ADJECTIVES: &[&str] = &[
    "amber", "brave", "calm", "clever", "gentle", "lucky", "merry", "quiet", "swift", "vivid",
];
const NOUNS: &[&str] = &[
    "badger", "falcon", "forest", "harbor", "otter", "river", "sparrow", "summit", "willow", "wolf",
];

pub fn current_repository(config: &Config, current_dir: &Path) -> anyhow::Result<Option<PathBuf>> {
    if !current_dir
        .ancestors()
        .any(|path| path.join(".jj").exists())
    {
        return Ok(None);
    }
    Ok(Some(repository_from(config, None, current_dir)?.0))
}

pub fn navigation_choices(
    config: &Config,
    repo: &Path,
    current_dir: &Path,
) -> anyhow::Result<Vec<(String, PathBuf)>> {
    let (base, _) = repository(config, Some(repo))?;
    let current_dir = current_dir.canonicalize()?;
    let mut choices = vec![("base".to_owned(), base)];
    for workspace in list(config, Some(repo))? {
        if !current_dir.starts_with(workspace.canonicalize()?) {
            let name = workspace.file_name().context("workspace name is missing")?;
            let name = name.to_string_lossy().into_owned();
            choices.push((name, workspace));
        }
    }
    Ok(choices)
}

fn repository(config: &Config, requested: Option<&Path>) -> anyhow::Result<(PathBuf, PathBuf)> {
    repository_from(config, requested, &env::current_dir()?)
}

fn repository_from(
    config: &Config,
    requested: Option<&Path>,
    current_dir: &Path,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let base = config.repositories_directory_path()?;
    let canonical_base = base
        .canonicalize()
        .context("repositories directory does not exist")?;
    let path = match requested {
        Some(path) if path.is_absolute() => path.to_owned(),
        Some(path) => base.join(path),
        None => current_dir
            .ancestors()
            .find(|path| path.join(".jj").exists())
            .map(Path::to_owned)
            .context("current directory is not inside a Jujutsu repository")?,
    };
    let canonical_path = path.canonicalize().context("repository does not exist")?;

    let relative = if let Ok(relative) = canonical_path.strip_prefix(&canonical_base) {
        relative.to_owned()
    } else if requested.is_none() {
        let workspace_base = config
            .workspaces_directory_path()?
            .canonicalize()
            .context("workspaces directory does not exist")?;
        canonical_path
            .strip_prefix(workspace_base)
            .context(
                "current repository is outside the configured repositories and workspaces directories",
            )?
            .parent()
            .context("workspace path does not include a repository")?
            .to_owned()
    } else {
        bail!("repository is outside the configured repositories directory");
    };
    if relative.components().count() < 2 {
        bail!("repository must have both a namespace and a name");
    }

    let repository = canonical_base.join(&relative);
    let repository = repository
        .canonicalize()
        .context("repository does not exist")?;
    Ok((repository, relative))
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

pub fn create(
    config: &Config,
    requested_repo: Option<&Path>,
    requested_name: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let (repo, relative_repo) = repository(config, requested_repo)?;
    let parent = config.workspaces_directory_path()?.join(relative_repo);
    fs::create_dir_all(&parent)?;
    let (name, destination) = if let Some(name) = requested_name {
        let mut components = Path::new(name).components();
        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
            bail!("workspace name must be a single path component");
        }
        let destination = parent.join(name);
        if destination.exists() {
            bail!("workspace already exists: {}", destination.display());
        }
        (name.to_owned(), destination)
    } else {
        (0..100)
            .map(|_| random_name())
            .map(|name| (name.clone(), parent.join(name)))
            .find(|(_, destination)| !destination.exists())
            .context("could not generate an unused workspace name")?
    };
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
    Ok(destination)
}

pub fn list(config: &Config, requested_repo: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let (_, relative_repo) = repository(config, requested_repo)?;
    let base = config.workspaces_directory_path()?.join(relative_repo);
    let mut results = Vec::new();
    if !base.exists() {
        return Ok(results);
    }

    for entry in fs::read_dir(base)? {
        let path = entry?.path();
        if path.is_dir() && path.join(".jj").exists() {
            results.push(path);
        }
    }
    results.sort();
    Ok(results)
}

pub fn remove(config: &Config, relative: &Path) -> anyhow::Result<PathBuf> {
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
    let canonical_destination = destination
        .canonicalize()
        .context("workspace does not exist")?;
    let source_repository = config.repositories_directory_path()?.join(repo_relative);
    let output = Command::new("jj")
        .arg("status")
        .current_dir(&destination)
        .output()
        .context("failed to execute 'jj status' - ensure jj is installed")?;
    if !output.status.success() {
        bail!(
            "jj status failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let output = Command::new("jj")
        .args(["workspace", "forget"])
        .arg(name)
        .current_dir(source_repository)
        .output()
        .context("failed to execute 'jj workspace forget' - ensure jj is installed")?;
    if !output.status.success() {
        bail!(
            "jj workspace forget failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    fs::remove_dir_all(destination)?;
    Ok(canonical_destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn config(root: &Path) -> Config {
        let repositories = root.join("repos");
        let workspaces = root.join("workspaces");
        fs::create_dir_all(&repositories).unwrap();
        fs::create_dir_all(&workspaces).unwrap();
        serde_json::from_value(json!({
            "repositories_directory": repositories,
            "workspaces_directory": workspaces,
        }))
        .unwrap()
    }

    fn create_repository(config: &Config, relative: &str) -> PathBuf {
        let repository = config.repositories_directory_path().unwrap().join(relative);
        fs::create_dir_all(repository.join(".jj")).unwrap();
        repository
    }

    #[test]
    fn lists_only_workspaces_for_requested_repository() {
        let root = tempdir().unwrap();
        let config = config(root.path());
        create_repository(&config, "owner/repo");
        create_repository(&config, "other/repo");

        let workspace = config
            .workspaces_directory_path()
            .unwrap()
            .join("owner/repo/calm-otter");
        fs::create_dir_all(workspace.join(".jj")).unwrap();
        fs::create_dir_all(
            config
                .workspaces_directory_path()
                .unwrap()
                .join("owner/repo/not-a-workspace"),
        )
        .unwrap();
        fs::create_dir_all(
            config
                .workspaces_directory_path()
                .unwrap()
                .join("other/repo/swift-wolf/.jj"),
        )
        .unwrap();

        assert_eq!(
            list(&config, Some(Path::new("owner/repo"))).unwrap(),
            vec![workspace]
        );
    }

    #[test]
    fn infers_repository_from_source_or_workspace_directory() {
        let root = tempdir().unwrap();
        let config = config(root.path());
        let repository = create_repository(&config, "owner/repo");
        let source_directory = repository.join("src");
        fs::create_dir_all(&source_directory).unwrap();

        let (_, relative) = repository_from(&config, None, &source_directory).unwrap();
        assert_eq!(relative, Path::new("owner/repo"));

        let workspace_directory = config
            .workspaces_directory_path()
            .unwrap()
            .join("owner/repo/calm-otter/src");
        fs::create_dir_all(workspace_directory.parent().unwrap().join(".jj")).unwrap();
        fs::create_dir_all(&workspace_directory).unwrap();

        let (resolved, relative) = repository_from(&config, None, &workspace_directory).unwrap();
        assert_eq!(resolved, repository.canonicalize().unwrap());
        assert_eq!(relative, Path::new("owner/repo"));
    }

    #[test]
    fn navigation_includes_base_and_excludes_active_workspace_from_subdirectories() {
        let root = tempdir().unwrap();
        let config = config(root.path());
        let repo = create_repository(&config, "owner/repo");
        let parent = config
            .workspaces_directory_path()
            .unwrap()
            .join("owner/repo");
        let active = parent.join("calm-otter");
        let other = parent.join("swift-wolf");
        for path in [&active, &other] {
            fs::create_dir_all(path.join(".jj")).unwrap();
            fs::create_dir_all(path.join("src")).unwrap();
        }
        assert_eq!(
            current_repository(&config, &active.join("src")).unwrap(),
            Some(repo.canonicalize().unwrap())
        );
        assert_eq!(
            navigation_choices(&config, &repo, &active.join("src")).unwrap(),
            vec![
                ("base".to_owned(), repo.canonicalize().unwrap()),
                ("swift-wolf".to_owned(), other)
            ]
        );
        assert_eq!(navigation_choices(&config, &repo, &repo).unwrap().len(), 3);
    }

    #[test]
    fn navigation_outside_repository_and_without_workspaces() {
        let root = tempdir().unwrap();
        let config = config(root.path());
        let repo = create_repository(&config, "owner/repo");
        assert_eq!(current_repository(&config, root.path()).unwrap(), None);
        let git_only = config
            .repositories_directory_path()
            .unwrap()
            .join("owner/git-only");
        fs::create_dir_all(git_only.join(".git")).unwrap();
        assert_eq!(current_repository(&config, &git_only).unwrap(), None);
        assert_eq!(
            navigation_choices(&config, &repo, root.path()).unwrap(),
            vec![("base".to_owned(), repo.canonicalize().unwrap())]
        );
    }

    #[test]
    fn generated_names_have_adjective_noun_format() {
        let name = random_name();
        let (adjective, noun) = name.split_once('-').unwrap();
        assert!(ADJECTIVES.contains(&adjective));
        assert!(NOUNS.contains(&noun));
    }
}
