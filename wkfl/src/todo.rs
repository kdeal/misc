use anyhow::{anyhow, Result};
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::prompts::select_prompt;

const SPACES_PER_INDENT: usize = 4;

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub index: usize,
    pub completed: bool,
    pub description: String,
    pub indentation_level: usize, // 0 = root, 1 = 4 spaces, 2 = 8 spaces, etc.
}

impl TodoItem {
    fn indentation(&self) -> String {
        " ".repeat(self.indentation_level * SPACES_PER_INDENT)
    }

    fn checkbox(&self) -> &str {
        if self.completed {
            "x"
        } else {
            " "
        }
    }

    fn to_output_format(&self) -> String {
        format!(
            "{}- [{}] {}",
            self.indentation(),
            self.checkbox(),
            self.description
        )
    }
}

impl Display for TodoItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}. [{}] {}",
            self.indentation(),
            self.index,
            self.checkbox(),
            self.description
        )
    }
}

#[derive(Debug)]
pub struct TodoFile {
    pub items: Vec<TodoItem>,
    pub file_path: PathBuf,
}

impl TodoFile {
    /// Validates that a user-provided 1-based index is valid for the current list
    fn validate_user_index(&self, user_index: usize) -> Result<()> {
        if self.items.is_empty() {
            return Err(anyhow!("No todo items found."));
        }
        if user_index == 0 || user_index > self.items.len() {
            return Err(anyhow!(
                "Invalid index: {}. Valid range is 1-{}",
                user_index,
                self.items.len()
            ));
        }
        Ok(())
    }

    pub fn load(notes_directory: PathBuf) -> Result<Self> {
        let file_path = notes_directory.join("todo.md");

        if !file_path.exists() {
            return Ok(TodoFile {
                items: Vec::new(),
                file_path,
            });
        }

        let content = fs::read_to_string(&file_path)?;
        Self::parse(content, file_path)
    }

    pub fn parse(content: String, file_path: PathBuf) -> Result<Self> {
        if content.trim().is_empty() {
            // Empty file, create default structure
            return Ok(TodoFile {
                items: Vec::new(),
                file_path,
            });
        }

        let lines: Vec<&str> = content.lines().collect();

        // Check that the file starts with "# Todo List"
        if lines.is_empty() || lines[0].trim() != "# Todo List" {
            return Err(anyhow!(
                "Todo file exists but does not start with '# Todo List' heading. Please ensure the file only contains the todo list."
            ));
        }

        // Parse todo items and validate content
        let mut items = Vec::new();
        let todo_regex = regex::Regex::new(r"^(\s*)- \[([ x])\] (.+)$")?;

        for line in &lines[1..] {
            let line_trimmed = line.trim();

            // Skip empty lines
            if line_trimmed.is_empty() {
                continue;
            }

            // Check if line matches todo item pattern
            if let Some(captures) = todo_regex.captures(line) {
                let indentation_str = captures.get(1).unwrap().as_str();
                let completed = captures.get(2).unwrap().as_str() == "x";
                let description = captures.get(3).unwrap().as_str().to_string();

                let indentation_level = indentation_str.len().div_ceil(4);

                items.push(TodoItem {
                    index: items.len() + 1,
                    completed,
                    description,
                    indentation_level,
                });
            } else {
                // Found a non-empty line that's not a todo item
                return Err(anyhow!(
                    "Todo file contains invalid content: '{}'. Only todo items (- [ ] or - [x]) are allowed after the '# Todo List' heading.",
                    line.trim()
                ));
            }
        }

        Ok(TodoFile { items, file_path })
    }

    pub fn save(&self) -> Result<()> {
        let mut content = String::new();

        // Add todo heading and items
        content.push_str("# Todo List\n");

        content.push_str(
            &self
                .items
                .iter()
                .map(|i| i.to_output_format())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        content.push('\n');

        // Atomic write: write to temporary file, then rename
        let temp_path = self.file_path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(temp_path, &self.file_path)?;

        Ok(())
    }

    pub fn add_item(
        &mut self,
        description: String,
        at_top: bool,
        after_index: Option<usize>,
        nest: bool,
    ) -> Result<()> {
        // First, determine the insertion index
        let insertion_index = if at_top {
            0
        } else if let Some(after_idx) = after_index {
            self.validate_user_index(after_idx)?;
            after_idx // after_idx is 1-based, so this puts it after the item
        } else {
            self.items.len() // Append to end
        };

        // Determine indentation level based on the item immediately before the insertion point
        let indentation_level = if nest {
            if insertion_index == 0 || self.items.is_empty() {
                0 // Inserting at start or empty list, use root level
            } else {
                // Get the item that will be immediately before the insertion point
                let previous_item = &self.items[insertion_index - 1];
                previous_item.indentation_level + 1
            }
        } else {
            0 // Root level by default
        };

        let new_item = TodoItem {
            index: 0, // Will be re-indexed
            completed: false,
            description,
            indentation_level,
        };

        // Insert at the computed position
        self.items.insert(insertion_index, new_item);

        // Re-index all items
        for (i, item) in self.items.iter_mut().enumerate() {
            item.index = i + 1;
        }

        Ok(())
    }

    pub fn remove_item(&mut self, user_index: usize) -> Result<TodoItem> {
        self.validate_user_index(user_index)?;

        let removed_item = self.items.remove(user_index - 1);

        // Re-index all items
        for (i, item) in self.items.iter_mut().enumerate() {
            item.index = i + 1;
        }

        Ok(removed_item)
    }

    pub fn set_item_completion(&mut self, user_index: usize, completed: bool) -> Result<()> {
        self.validate_user_index(user_index)?;
        self.items[user_index - 1].completed = completed;
        Ok(())
    }

    pub fn get_filtered_items(&self, show_pending: bool, show_completed: bool) -> Vec<&TodoItem> {
        if !show_pending && !show_completed {
            // Show all items by default
            self.items.iter().collect()
        } else {
            self.items
                .iter()
                .filter(|item| {
                    (show_pending && !item.completed) || (show_completed && item.completed)
                })
                .collect()
        }
    }
}

pub fn list_todos(config: &Config, pending: bool, completed: bool, count_only: bool) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let todo_file = TodoFile::load(notes_directory)?;

    let filtered_items = todo_file.get_filtered_items(pending, completed);

    if count_only {
        println!("{}", filtered_items.len());
        return Ok(());
    }

    if filtered_items.is_empty() {
        if pending && completed {
            println!("No todo items found.");
        } else if pending {
            println!("No pending todo items found.");
        } else if completed {
            println!("No completed todo items found.");
        } else {
            println!("No todo items found.");
        }
        return Ok(());
    }

    let total_items = todo_file.items.len();
    let filter_desc = if pending && completed {
        format!("{} items", filtered_items.len())
    } else if pending {
        format!("{} pending items", filtered_items.len())
    } else if completed {
        format!("{} completed items", filtered_items.len())
    } else {
        format!("{} items", total_items)
    };

    println!("Todo List ({}):", filter_desc);

    for item in filtered_items {
        println!("{}", item);
    }

    Ok(())
}

pub fn add_todo(
    config: &Config,
    description: String,
    at_top: bool,
    after_index: Option<usize>,
    nest: bool,
) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let mut todo_file = TodoFile::load(notes_directory)?;

    todo_file.add_item(description.clone(), at_top, after_index, nest)?;
    todo_file.save()?;

    println!("Added todo item: {}", description);
    Ok(())
}

fn select_todo_item(items: &[&TodoItem], action: &str) -> Result<usize> {
    if items.is_empty() {
        return Err(anyhow!("No todo items found."));
    }

    let options: Vec<String> = items.iter().map(|item| item.to_string()).collect();

    let prompt = format!("Select todo item to {}:", action);
    let selected = select_prompt(&prompt, &options)?;

    // Extract the user index from the selected option
    let parts: Vec<&str> = selected.trim().split('.').collect();
    if parts.len() < 2 {
        return Err(anyhow!("Failed to parse selected todo item"));
    }

    let user_index_str = parts[0].trim();
    let user_index = user_index_str
        .parse::<usize>()
        .map_err(|_| anyhow!("Failed to parse todo item index"))?;

    Ok(user_index)
}

pub fn remove_todo(config: &Config, user_index: Option<usize>) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let mut todo_file = TodoFile::load(notes_directory)?;

    let index = match user_index {
        Some(idx) => idx,
        None => select_todo_item(&todo_file.items.iter().collect::<Vec<_>>(), "remove")?,
    };

    let removed_item = todo_file.remove_item(index)?;
    todo_file.save()?;

    println!("Removed todo item: {}", removed_item.description);
    Ok(())
}

pub fn check_todo(config: &Config, user_index: Option<usize>) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let mut todo_file = TodoFile::load(notes_directory)?;

    let index = match user_index {
        Some(idx) => idx,
        None => select_todo_item(&todo_file.get_filtered_items(true, false), "check")?,
    };

    todo_file.set_item_completion(index, true)?;
    todo_file.save()?;

    let item_description = todo_file.items[index - 1].description.clone();
    println!("Marked as completed: {}", item_description);
    Ok(())
}

pub fn uncheck_todo(config: &Config, user_index: Option<usize>) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let mut todo_file = TodoFile::load(notes_directory)?;

    let index = match user_index {
        Some(idx) => idx,
        None => select_todo_item(&todo_file.get_filtered_items(false, true), "uncheck")?,
    };

    todo_file.set_item_completion(index, false)?;
    todo_file.save()?;

    let item_description = todo_file.items[index - 1].description.clone();
    println!("Marked as pending: {}", item_description);
    Ok(())
}
