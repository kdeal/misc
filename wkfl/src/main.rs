use std::{env, error::Error, path::PathBuf};

use clap::{Parser, Subcommand};

mod actions;
mod config;
mod git;
mod prompts;
mod repositories;
mod shell_actions;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
    #[arg(long)]
    shell_actions_file: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start,
    End,
    RepoDebug,
    Repos,
    Repo,
    Clone,
    Confirm {
        prompt: Option<String>,
        #[arg(short = 't', long)]
        default_true: bool,
    },
    Select {
        prompt: Option<String>,
    },
}

pub struct Context {
    config: config::Config,
    shell_actions: Vec<shell_actions::ShellAction>,
}

fn setup_logging(verbose: bool) {
    let mut log_builder = env_logger::builder();
    if verbose {
        log_builder.filter(None, log::LevelFilter::Debug);
    } else {
        // Only set default of info if not configured via env already
        if env::var("RUST_LOG").is_err() {
            log_builder.filter(None, log::LevelFilter::Info);
        }
        log_builder.format_timestamp(None);
    }
    log_builder.init();
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    let mut context = Context {
        config: config::get_config()?,
        shell_actions: vec![],
    };
    match cli.command {
        Commands::Start => actions::start_workflow(&mut context)?,
        Commands::End => actions::end_workflow()?,
        Commands::RepoDebug => actions::print_repo_debug_info()?,
        Commands::Repos => actions::list_repositories(context.config)?,
        Commands::Repo => actions::switch_repo(&mut context)?,
        Commands::Clone => actions::clone_repo(&mut context)?,
        Commands::Confirm {
            prompt: user_prompt,
            default_true: default,
        } => {
            let prompt = user_prompt.unwrap_or("Confirm?".to_string());
            actions::confirm(&prompt, default)?
        }
        Commands::Select {
            prompt: user_prompt,
        } => {
            let prompt = user_prompt.unwrap_or("?".to_string());
            actions::select(&prompt)?
        }
    };

    if let Some(shell_actions_file) = cli.shell_actions_file {
        shell_actions::write_shell_commands(&context.shell_actions, shell_actions_file)?;
    }

    Ok(())
}
