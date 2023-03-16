use std::env;

// Uses the same vars as getpass.getuser in python
pub fn get_current_user() -> Option<String> {
    for env_var in ["LOGNAME", "USER", "LNAME", "USERNAME"] {
        if let Ok(value) = env::var(env_var) {
            return Some(value);
        }
    }
    None
}
