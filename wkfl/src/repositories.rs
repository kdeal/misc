use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Returns `true` when the directory contains metadata for a supported VCS
/// repository. Currently, a directory is considered a repository if it has a
/// `.git` or `.jj` subdirectory, allowing the `repos` command to surface both
/// Git and Jujutsu repositories.
fn is_dir_a_repo(directory: &Path) -> bool {
    let has_git_dir = directory.join(".git").as_path().exists();
    let has_jj_dir = directory.join(".jj").as_path().exists();

    has_git_dir || has_jj_dir
}

fn check_read_dir_entry(dir_entry_result: io::Result<fs::DirEntry>) -> Option<PathBuf> {
    let entry = dir_entry_result.ok()?;
    let entry_path = entry.path();

    if let Some(file_name) = entry_path.file_name() {
        if file_name.to_string_lossy().starts_with('.') {
            return None;
        }
    }

    let mut file_type = entry.file_type().ok()?;
    if file_type.is_symlink() {
        let symlink_path = fs::read_link(&entry_path).unwrap();
        let file_metadata = fs::symlink_metadata(symlink_path).ok()?;
        file_type = file_metadata.file_type();
    }

    if file_type.is_dir() {
        Some(entry_path)
    } else {
        None
    }
}

fn get_sub_directories(directory: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut sub_directories = vec![];
    for result in directory.read_dir()? {
        if let Some(path) = check_read_dir_entry(result) {
            sub_directories.push(path);
        }
    }
    Ok(sub_directories)
}

pub fn get_repositories_in_directory(directory: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut repositories = vec![];
    let mut dirs_to_check = vec![directory.to_owned()];
    while let Some(current_dir) = dirs_to_check.pop() {
        if !current_dir.exists() {
            continue;
        }
        if is_dir_a_repo(&current_dir) {
            repositories.push(current_dir);
        } else {
            let mut sub_directories = get_sub_directories(&current_dir)?;
            dirs_to_check.append(&mut sub_directories);
        }
    }
    Ok(repositories)
}

pub fn get_repository_names_in_directory(directory: &Path) -> anyhow::Result<Vec<String>> {
    let mut repositories = get_repositories_in_directory(directory)?
        .into_iter()
        .map(|repo_path| {
            repo_path
                .strip_prefix(directory)
                .map(|relative_path| relative_path.display().to_string())
                .map_err(anyhow::Error::from)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    repositories.sort();
    Ok(repositories)
}

#[cfg(test)]
mod tests {
    use super::{get_repository_names_in_directory, is_dir_a_repo};
    use std::fs;
    use tempfile::tempdir;

    fn assert_repo_detection(metadata_dir: Option<&str>, expected: bool) {
        let temp_dir = tempdir().expect("failed to create temp directory");
        if let Some(dir_name) = metadata_dir {
            let metadata_path = temp_dir.path().join(dir_name);
            fs::create_dir(&metadata_path).expect("failed to create metadata directory");
        }

        assert_eq!(is_dir_a_repo(temp_dir.path()), expected);
    }

    #[test]
    fn detects_git_repository_directories() {
        assert_repo_detection(Some(".git"), true);
    }

    #[test]
    fn detects_jujutsu_repository_directories() {
        assert_repo_detection(Some(".jj"), true);
    }

    #[test]
    fn returns_false_for_directories_without_vcs_metadata() {
        assert_repo_detection(None, false);
    }

    #[test]
    fn returns_sorted_relative_repository_names() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        fs::create_dir_all(temp_dir.path().join("zebra/.git")).expect("failed to create git repo");
        fs::create_dir_all(temp_dir.path().join("apps/api/.jj")).expect("failed to create jj repo");

        let repositories = get_repository_names_in_directory(temp_dir.path())
            .expect("failed to list repositories");

        assert_eq!(repositories, vec!["apps/api", "zebra"]);
    }
}
