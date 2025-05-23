use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

pub enum ShellAction {
    Cd { path: PathBuf },
    EditFile { path: PathBuf },
}

pub fn write_shell_commands(commands: &Vec<ShellAction>, filepath: PathBuf) -> anyhow::Result<()> {
    let mut output_file = BufWriter::new(File::create(filepath)?);
    for command in commands {
        match command {
            ShellAction::Cd { path } => {
                output_file.write_all(b"cd,")?;
                output_file.write_all(path.to_string_lossy().as_bytes())?;
            }
            ShellAction::EditFile { path } => {
                output_file.write_all(b"edit_file,")?;
                output_file.write_all(path.to_string_lossy().as_bytes())?;
            }
        };
        output_file.write_all(b"\n")?;
    }
    output_file.flush()?;
    Ok(())
}
