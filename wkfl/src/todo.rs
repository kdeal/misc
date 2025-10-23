use anyhow::{anyhow, Result};
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;
use tree_sitter::Node;
use tree_sitter_md::MarkdownParser;

use crate::config::Config;
use crate::prompts::select_prompt;
use crate::shell_actions::ShellAction;
use crate::Context;

const TODO_FILENAME: &str = "todo.md";

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub index: usize,
    pub completed: bool,
    pub description: String,
}

impl TodoItem {
    fn checkbox(&self) -> &str {
        if self.completed {
            "x"
        } else {
            " "
        }
    }

    fn to_output_format(&self) -> String {
        format!("- [{}] {}", self.checkbox(), self.description)
    }
}

impl Display for TodoItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}. [{}] {}", self.index, self.checkbox(), self.description)
    }
}

#[derive(Debug, Clone)]
pub struct TodoSection {
    pub name: String,
    pub level: usize, // 1 for h1, 2 for h2
    pub items: Vec<TodoItem>,
}

impl TodoSection {
    fn new(name: String, level: usize) -> Self {
        Self {
            name,
            level,
            items: Vec::new(),
        }
    }

    fn to_output_format(&self) -> String {
        let mut output = String::new();

        // Add heading
        let heading_prefix = "#".repeat(self.level);
        output.push_str(&format!("{} {}\n", heading_prefix, self.name));

        // Add items
        for item in &self.items {
            output.push_str(&item.to_output_format());
            output.push('\n');
        }

        output
    }
}

#[derive(Debug)]
pub struct TodoFile {
    pub sections: Vec<TodoSection>,
    pub file_path: PathBuf,
}

impl TodoFile {
    /// Get all items across all sections with global indices
    fn all_items(&self) -> Vec<(usize, &TodoSection, &TodoItem)> {
        let mut result = Vec::new();
        let mut global_index = 1;

        for section in &self.sections {
            for item in &section.items {
                result.push((global_index, section, item));
                global_index += 1;
            }
        }

        result
    }

    /// Get total count of items across all sections
    fn total_items(&self) -> usize {
        self.sections.iter().map(|s| s.items.len()).sum()
    }

    /// Validates that a user-provided 1-based index is valid for the current list
    fn validate_user_index(&self, user_index: usize) -> Result<()> {
        let total = self.total_items();
        if total == 0 {
            return Err(anyhow!("No todo items found."));
        }
        if user_index == 0 || user_index > total {
            return Err(anyhow!(
                "Invalid index: {}. Valid range is 1-{}",
                user_index,
                total
            ));
        }
        Ok(())
    }

    pub fn load(notes_directory: PathBuf) -> Result<Self> {
        let file_path = notes_directory.join(TODO_FILENAME);

        if !file_path.exists() {
            return Ok(TodoFile {
                sections: vec![TodoSection::new("Todo".to_string(), 1)],
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
                sections: vec![TodoSection::new("Todo".to_string(), 1)],
                file_path,
            });
        }

        let mut parser = MarkdownParser::new();
        let tree = parser.parse(&content).ok_or_else(|| anyhow!("Failed to parse markdown"))?;
        let root = tree.root_node();

        let mut sections = Vec::new();
        let mut current_section: Option<TodoSection> = None;

        Self::walk_tree(root, &content, &mut current_section, &mut sections)?;

        // Add the last section if it exists
        if let Some(section) = current_section {
            sections.push(section);
        }

        // If no sections were found, create a default one
        if sections.is_empty() {
            sections.push(TodoSection::new("Todo".to_string(), 1));
        }

        Ok(TodoFile { sections, file_path })
    }

    fn walk_tree(
        node: Node,
        source: &str,
        current_section: &mut Option<TodoSection>,
        sections: &mut Vec<TodoSection>,
    ) -> Result<()> {
        match node.kind() {
            "atx_heading" => {
                // Extract heading level and text
                let heading_text = Self::extract_heading_text(node, source)?;
                let level = Self::get_heading_level(node, source)?;

                // Only process h1 and h2
                if level == 1 || level == 2 {
                    // Save the previous section if it exists
                    if let Some(section) = current_section.take() {
                        sections.push(section);
                    }

                    // Start a new section
                    *current_section = Some(TodoSection::new(heading_text, level));
                }
            }
            "task_list_item" => {
                // Extract TODO text and completion state
                if let Some(section) = current_section {
                    let (completed, description) = Self::extract_task_item(node, source)?;
                    section.items.push(TodoItem {
                        index: section.items.len() + 1,
                        completed,
                        description,
                    });
                }
            }
            _ => {}
        }

        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::walk_tree(child, source, current_section, sections)?;
            }
        }

        Ok(())
    }

    fn extract_heading_text(node: Node, source: &str) -> Result<String> {
        // For atx_heading, we need to skip the '#' markers and get the actual text
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "atx_h1_marker"
                    || child.kind() == "atx_h2_marker"
                    || child.kind() == "atx_h3_marker"
                    || child.kind() == "atx_h4_marker"
                    || child.kind() == "atx_h5_marker"
                    || child.kind() == "atx_h6_marker" {
                    continue;
                }

                // Get the text content
                let text = child.utf8_text(source.as_bytes())
                    .map_err(|_| anyhow!("Failed to extract heading text"))?
                    .trim()
                    .to_string();

                if !text.is_empty() {
                    return Ok(text);
                }
            }
        }

        // Fallback: get the entire node text and strip markers
        let full_text = node.utf8_text(source.as_bytes())
            .map_err(|_| anyhow!("Failed to extract heading text"))?;
        Ok(full_text.trim_start_matches('#').trim().to_string())
    }

    fn get_heading_level(node: Node, source: &str) -> Result<usize> {
        // Count the number of '#' characters at the start
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let kind = child.kind();
                if kind.starts_with("atx_h") && kind.ends_with("_marker") {
                    // Extract the level from the marker name (e.g., "atx_h1_marker" -> 1)
                    let level_str = &kind[5..6];
                    return level_str.parse::<usize>()
                        .map_err(|_| anyhow!("Failed to parse heading level"));
                }
            }
        }

        // Fallback: count '#' characters
        let text = node.utf8_text(source.as_bytes())
            .map_err(|_| anyhow!("Failed to extract heading text"))?;
        let level = text.chars().take_while(|&c| c == '#').count();
        Ok(level)
    }

    fn extract_task_item(node: Node, source: &str) -> Result<(bool, String)> {
        let mut completed = false;
        let mut description = String::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "task_list_marker_checked" => {
                        completed = true;
                    }
                    "task_list_marker_unchecked" => {
                        completed = false;
                    }
                    _ => {
                        // Collect the description text from other nodes
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty()
                                && trimmed != "[ ]"
                                && trimmed != "[x]"
                                && trimmed != "-" {
                                if !description.is_empty() {
                                    description.push(' ');
                                }
                                description.push_str(trimmed);
                            }
                        }
                    }
                }
            }
        }

        Ok((completed, description.trim().to_string()))
    }

    pub fn save(&self) -> Result<()> {
        let mut content = String::new();

        // Add all sections
        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 {
                content.push('\n');
            }
            content.push_str(&section.to_output_format());
        }

        // Atomic write: write to temporary file, then rename
        let temp_path = self.file_path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(temp_path, &self.file_path)?;

        Ok(())
    }

    pub fn add_item(
        &mut self,
        description: String,
        section_name: Option<String>,
        at_top: bool,
        after_index: Option<usize>,
    ) -> Result<()> {
        // Find or create the target section
        let section_index = if let Some(name) = section_name {
            // Find the section by name
            self.sections
                .iter()
                .position(|s| s.name == name)
                .ok_or_else(|| anyhow!("Section '{}' not found", name))?
        } else {
            // Use the first section (default)
            if self.sections.is_empty() {
                self.sections.push(TodoSection::new("Todo".to_string(), 1));
            }
            0
        };

        let section = &mut self.sections[section_index];

        // Determine the insertion index within the section
        let insertion_index = if at_top {
            0
        } else if let Some(after_idx) = after_index {
            // Need to map global index to section-local index
            let mut global_index = 1;
            let mut found = false;
            let mut local_index = 0;

            for (sec_idx, sec) in self.sections.iter().enumerate() {
                for (item_idx, _) in sec.items.iter().enumerate() {
                    if global_index == after_idx {
                        if sec_idx == section_index {
                            local_index = item_idx + 1;
                            found = true;
                            break;
                        } else {
                            return Err(anyhow!(
                                "Cannot insert after item {} as it's in a different section",
                                after_idx
                            ));
                        }
                    }
                    global_index += 1;
                }
                if found {
                    break;
                }
            }

            if !found {
                return Err(anyhow!("Invalid after_index: {}", after_idx));
            }

            local_index
        } else {
            section.items.len() // Append to end
        };

        let new_item = TodoItem {
            index: 0, // Will be re-indexed
            completed: false,
            description,
        };

        // Insert at the computed position
        section.items.insert(insertion_index, new_item);

        // Re-index all items in the section
        for (i, item) in section.items.iter_mut().enumerate() {
            item.index = i + 1;
        }

        Ok(())
    }

    pub fn remove_item(&mut self, user_index: usize) -> Result<TodoItem> {
        self.validate_user_index(user_index)?;

        // Find the section and local index
        let mut global_index = 1;
        for section in &mut self.sections {
            if global_index + section.items.len() > user_index {
                let local_index = user_index - global_index;
                let removed_item = section.items.remove(local_index);

                // Re-index all items in the section
                for (i, item) in section.items.iter_mut().enumerate() {
                    item.index = i + 1;
                }

                return Ok(removed_item);
            }
            global_index += section.items.len();
        }

        Err(anyhow!("Invalid index: {}", user_index))
    }

    pub fn set_item_completion(&mut self, user_index: usize, completed: bool) -> Result<()> {
        self.validate_user_index(user_index)?;

        // Find the section and local index
        let mut global_index = 1;
        for section in &mut self.sections {
            if global_index + section.items.len() > user_index {
                let local_index = user_index - global_index;
                section.items[local_index].completed = completed;
                return Ok(());
            }
            global_index += section.items.len();
        }

        Err(anyhow!("Invalid index: {}", user_index))
    }

    pub fn get_filtered_items(&self, show_pending: bool, show_completed: bool) -> Vec<(usize, &TodoSection, &TodoItem)> {
        let all = self.all_items();

        if !show_pending && !show_completed {
            // Show all items by default
            all
        } else {
            all.into_iter()
                .filter(|(_, _, item)| {
                    (show_pending && !item.completed) || (show_completed && item.completed)
                })
                .collect()
        }
    }

    pub fn add_section(&mut self, name: String, level: usize) -> Result<()> {
        if level != 1 && level != 2 {
            return Err(anyhow!("Section level must be 1 (h1) or 2 (h2)"));
        }

        // Check if section already exists
        if self.sections.iter().any(|s| s.name == name) {
            return Err(anyhow!("Section '{}' already exists", name));
        }

        self.sections.push(TodoSection::new(name, level));
        Ok(())
    }

    pub fn list_sections(&self) -> Vec<&TodoSection> {
        self.sections.iter().collect()
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

    let total_items = todo_file.total_items();
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

    let mut current_section: Option<&str> = None;
    for (global_index, section, item) in filtered_items {
        // Print section header if we've moved to a new section
        if current_section != Some(&section.name) {
            println!("\n{} {}:", "#".repeat(section.level), section.name);
            current_section = Some(&section.name);
        }

        // Create a display item with the global index
        let display_item = TodoItem {
            index: global_index,
            completed: item.completed,
            description: item.description.clone(),
        };
        println!("  {}", display_item);
    }

    Ok(())
}

pub fn add_todo(
    config: &Config,
    description: String,
    section_name: Option<String>,
    at_top: bool,
    after_index: Option<usize>,
) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let mut todo_file = TodoFile::load(notes_directory)?;

    todo_file.add_item(description.clone(), section_name, at_top, after_index)?;
    todo_file.save()?;

    println!("Added todo item: {}", description);
    Ok(())
}

fn select_todo_item(items: &[(usize, &TodoSection, &TodoItem)], action: &str) -> Result<usize> {
    if items.is_empty() {
        return Err(anyhow!("No todo items found."));
    }

    let options: Vec<String> = items
        .iter()
        .map(|(global_index, section, item)| {
            let display_item = TodoItem {
                index: *global_index,
                completed: item.completed,
                description: item.description.clone(),
            };
            format!("[{}] {}", section.name, display_item)
        })
        .collect();

    let prompt = format!("Select todo item to {}:", action);
    let selected = select_prompt(&prompt, &options)?;

    // Extract the user index from the selected option
    // Format is "[Section] N. [x] Description"
    let parts: Vec<&str> = selected.split(']').collect();
    if parts.len() < 2 {
        return Err(anyhow!("Failed to parse selected todo item"));
    }

    let index_part = parts[1].trim();
    let index_str = index_part.split('.').next().unwrap_or("").trim();
    let user_index = index_str
        .parse::<usize>()
        .map_err(|_| anyhow!("Failed to parse todo item index"))?;

    Ok(user_index)
}

pub fn remove_todo(config: &Config, user_index: Option<usize>) -> Result<()> {
    let notes_directory = config.notes_directory_path()?;
    let mut todo_file = TodoFile::load(notes_directory)?;

    let index = match user_index {
        Some(idx) => idx,
        None => {
            let all_items = todo_file.all_items();
            select_todo_item(&all_items, "remove")?
        }
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

    // Find the item description
    let all_items = todo_file.all_items();
    if let Some((_, _, item)) = all_items.iter().find(|(i, _, _)| *i == index) {
        println!("Marked as completed: {}", item.description);
    }

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

    // Find the item description
    let all_items = todo_file.all_items();
    if let Some((_, _, item)) = all_items.iter().find(|(i, _, _)| *i == index) {
        println!("Marked as pending: {}", item.description);
    }

    Ok(())
}

pub fn edit_todo(context: &mut Context) -> Result<()> {
    let notes_directory = context.config.notes_directory_path()?;
    let todo_file_path = notes_directory.join(TODO_FILENAME);

    // Ensure the todo file exists by creating an empty one if it doesn't
    if !todo_file_path.exists() {
        let empty_todo_file = TodoFile {
            sections: vec![TodoSection::new("Todo".to_string(), 1)],
            file_path: todo_file_path.clone(),
        };
        empty_todo_file.save()?;
    }

    context.shell_actions.push(ShellAction::EditFile {
        path: todo_file_path,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_single_section() {
        let content = r#"# Todo
- [ ] First task
- [x] Completed task
- [ ] Another task
"#;
        let result = TodoFile::parse(content.to_string(), PathBuf::from("/tmp/test.md"));
        assert!(result.is_ok());

        let todo_file = result.unwrap();
        assert_eq!(todo_file.sections.len(), 1);
        assert_eq!(todo_file.sections[0].name, "Todo");
        assert_eq!(todo_file.sections[0].level, 1);
        assert_eq!(todo_file.sections[0].items.len(), 3);
        assert!(!todo_file.sections[0].items[0].completed);
        assert!(todo_file.sections[0].items[1].completed);
        assert!(!todo_file.sections[0].items[2].completed);
    }

    #[test]
    fn test_parse_multiple_sections() {
        let content = r#"# Todo
- [ ] Main task

## Work
- [ ] Review PR
- [x] Fix bug

## Personal
- [ ] Buy groceries
"#;
        let result = TodoFile::parse(content.to_string(), PathBuf::from("/tmp/test.md"));
        assert!(result.is_ok());

        let todo_file = result.unwrap();
        assert_eq!(todo_file.sections.len(), 3);

        assert_eq!(todo_file.sections[0].name, "Todo");
        assert_eq!(todo_file.sections[0].level, 1);
        assert_eq!(todo_file.sections[0].items.len(), 1);

        assert_eq!(todo_file.sections[1].name, "Work");
        assert_eq!(todo_file.sections[1].level, 2);
        assert_eq!(todo_file.sections[1].items.len(), 2);

        assert_eq!(todo_file.sections[2].name, "Personal");
        assert_eq!(todo_file.sections[2].level, 2);
        assert_eq!(todo_file.sections[2].items.len(), 1);
    }

    #[test]
    fn test_parse_empty_file() {
        let content = "";
        let result = TodoFile::parse(content.to_string(), PathBuf::from("/tmp/test.md"));
        assert!(result.is_ok());

        let todo_file = result.unwrap();
        assert_eq!(todo_file.sections.len(), 1);
        assert_eq!(todo_file.sections[0].name, "Todo");
        assert_eq!(todo_file.sections[0].items.len(), 0);
    }

    #[test]
    fn test_save_and_parse_roundtrip() {
        let mut todo_file = TodoFile {
            sections: vec![
                TodoSection {
                    name: "Todo".to_string(),
                    level: 1,
                    items: vec![
                        TodoItem {
                            index: 1,
                            completed: false,
                            description: "Task 1".to_string(),
                        },
                    ],
                },
                TodoSection {
                    name: "Work".to_string(),
                    level: 2,
                    items: vec![
                        TodoItem {
                            index: 1,
                            completed: true,
                            description: "Task 2".to_string(),
                        },
                    ],
                },
            ],
            file_path: PathBuf::from("/tmp/test_roundtrip.md"),
        };

        // Save
        let result = todo_file.save();
        assert!(result.is_ok());

        // Read back
        let content = std::fs::read_to_string(&todo_file.file_path).unwrap();
        let parsed = TodoFile::parse(content, todo_file.file_path.clone()).unwrap();

        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].name, "Todo");
        assert_eq!(parsed.sections[0].items.len(), 1);
        assert_eq!(parsed.sections[1].name, "Work");
        assert_eq!(parsed.sections[1].items.len(), 1);
    }

    #[test]
    fn test_add_item_to_default_section() {
        let mut todo_file = TodoFile {
            sections: vec![TodoSection::new("Todo".to_string(), 1)],
            file_path: PathBuf::from("/tmp/test.md"),
        };

        let result = todo_file.add_item("New task".to_string(), None, false, None);
        assert!(result.is_ok());
        assert_eq!(todo_file.sections[0].items.len(), 1);
        assert_eq!(todo_file.sections[0].items[0].description, "New task");
    }

    #[test]
    fn test_add_item_to_specific_section() {
        let mut todo_file = TodoFile {
            sections: vec![
                TodoSection::new("Todo".to_string(), 1),
                TodoSection::new("Work".to_string(), 2),
            ],
            file_path: PathBuf::from("/tmp/test.md"),
        };

        let result = todo_file.add_item("Work task".to_string(), Some("Work".to_string()), false, None);
        assert!(result.is_ok());
        assert_eq!(todo_file.sections[1].items.len(), 1);
        assert_eq!(todo_file.sections[1].items[0].description, "Work task");
    }

    #[test]
    fn test_global_indexing() {
        let todo_file = TodoFile {
            sections: vec![
                TodoSection {
                    name: "Todo".to_string(),
                    level: 1,
                    items: vec![
                        TodoItem { index: 1, completed: false, description: "Task 1".to_string() },
                        TodoItem { index: 2, completed: false, description: "Task 2".to_string() },
                    ],
                },
                TodoSection {
                    name: "Work".to_string(),
                    level: 2,
                    items: vec![
                        TodoItem { index: 1, completed: false, description: "Task 3".to_string() },
                    ],
                },
            ],
            file_path: PathBuf::from("/tmp/test.md"),
        };

        let all_items = todo_file.all_items();
        assert_eq!(all_items.len(), 3);
        assert_eq!(all_items[0].0, 1); // Global index
        assert_eq!(all_items[1].0, 2);
        assert_eq!(all_items[2].0, 3);
    }
}

