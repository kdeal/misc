use std::{
    io::{self, Write},
    process::Command,
};

use crossterm::style::Stylize;

pub fn run_commands_with_output(commands: &Vec<String>) -> anyhow::Result<()> {
    for command in commands {
        println!("{}", command.as_str().yellow());
        io::stdout().flush()?;

        let status = Command::new("sh").arg("-c").arg(command).status()?;

        if !status.success() {
            send_notification("✖ Command failed", true)?;
            return Err(anyhow::anyhow!("Command failed: {}", command));
        }
    }

    send_notification("✓ All commands completed successfully", false)?;
    Ok(())
}

pub fn send_notification(message: &str, is_error: bool) -> anyhow::Result<()> {
    let colorized_message = if is_error {
        message.red()
    } else {
        message.green()
    };

    // OSC 9 for system notification: ESC ] 9 ; [message] ESC \
    let osc_notification = format!("\x1b]9;{message}\x1b\\");

    println!("{colorized_message}{osc_notification}");
    io::stdout().flush()?;
    Ok(())
}

const LOWERCASE_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "if", "in", "of", "on", "or", "the", "to",
    "up", "yet", "nor", "via",
];

fn should_capitalize(input: &str, word_index: usize, total_words: usize) -> bool {
    if input.chars().any(|c| c.is_uppercase()) {
        return false;
    }

    if word_index == 0 {
        return true;
    }
    if word_index + 1 == total_words {
        return true;
    }

    if LOWERCASE_WORDS.contains(&input.trim()) {
        return false;
    }

    true
}

pub fn to_title_case(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    // Split the input preserving whitespace
    let mut result = String::with_capacity(input.len());
    let words: Vec<&str> = input.split_inclusive(char::is_whitespace).collect();
    let total_words = words.len();

    for (i, full_word) in words.into_iter().enumerate() {
        if should_capitalize(full_word, i, total_words) {
            // Capitalize first letter, preserve rest of the case
            let mut chars = full_word.chars();
            if let Some(first_char) = chars.next() {
                result.extend(first_char.to_uppercase());
                result.push_str(chars.as_str());
            }
        } else {
            result.push_str(full_word);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::to_title_case;
    #[test]
    fn test_empty_string() {
        assert_eq!(to_title_case(""), "");
    }

    #[test]
    fn test_basic_capitalization() {
        assert_eq!(to_title_case("hello world"), "Hello World");
    }

    #[test]
    fn test_preserve_subsequent_capitalization() {
        assert_eq!(to_title_case("MacBook Pro"), "MacBook Pro");
        assert_eq!(to_title_case("iPhone and iPad"), "iPhone and iPad");
    }

    #[test]
    fn test_articles_and_prepositions() {
        assert_eq!(
            to_title_case("the quick brown fox jumps over the lazy dog"),
            "The Quick Brown Fox Jumps Over the Lazy Dog"
        );
    }

    #[test]
    fn test_preserve_whitespace() {
        assert_eq!(
            to_title_case("hello   world  \t  test"),
            "Hello   World  \t  Test"
        );
    }

    #[test]
    fn test_first_and_last_words() {
        assert_eq!(to_title_case("the end"), "The End");
        assert_eq!(
            to_title_case("journey to the center"),
            "Journey to the Center"
        );
    }

    #[test]
    fn test_technical_terms() {
        assert_eq!(to_title_case("WiFi and 5G"), "WiFi and 5G");
    }

    #[test]
    fn test_mixed_case_with_brands() {
        assert_eq!(
            to_title_case("an iOS App for macOS"),
            "An iOS App for macOS"
        );
    }
}
