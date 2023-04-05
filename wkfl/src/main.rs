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
    Confirm {
        prompt: Option<String>,
        #[arg(short = 't', long)]
        default_true: bool,
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
        Commands::Start => {
            let repo = git::get_repository()?;
            let name = prompts::basic_prompt("Name:")?;
            let ticket_str = prompts::basic_prompt("Ticket:")?;
            let ticket = if ticket_str.is_empty() {
                None
            } else {
                Some(ticket_str)
            };
            actions::start_workflow(repo, &name, &ticket, &mut context)?
        }
        Commands::End => {
            let repo = git::get_repository()?;
            actions::end_workflow(repo)?;
        }
        Commands::RepoDebug => {
            let repo = git::get_repository()?;
            actions::print_repo_debug_info(repo)?;
        }
        Commands::Repos => actions::list_repositories(context.config)?,
        Commands::Repo => actions::switch_repo(&mut context)?,
        Commands::Confirm {
            prompt: user_prompt,
            default_true: default,
        } => {
            let prompt = user_prompt.unwrap_or("Confirm?".to_string());
            actions::confirm(&prompt, default)?
        }
    };

    if let Some(shell_actions_file) = cli.shell_actions_file {
        shell_actions::write_shell_commands(&context.shell_actions, shell_actions_file)?;
    }

    Ok(())
}
