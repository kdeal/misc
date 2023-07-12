use std::{env, process::Command};

// Uses the same vars as getpass.getuser in python
pub fn get_current_user() -> Option<String> {
    for env_var in ["LOGNAME", "USER", "LNAME", "USERNAME"] {
        if let Ok(value) = env::var(env_var) {
            return Some(value);
        }
    }
    None
}

pub fn run_commands(commands: &Vec<String>) -> anyhow::Result<()> {
    for command in commands {
        Command::new("sh").arg("-c").arg(command).status()?;
    }
    Ok(())
}
