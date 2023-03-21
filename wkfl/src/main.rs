use std::{env, error::Error};

use clap::{Parser, Subcommand};

mod actions;
mod config;
mod git;
mod utils;
mod repositories;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        name: String,
        ticket: Option<String>,
    },
    End {
        name: Option<String>,
    },
    RepoDebug,
    Repos,
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

    let config = config::get_config()?;
    match &cli.command {
        Commands::Start { name, ticket } => {
            let repo = git::get_repository()?;
            actions::start_workflow(repo, name, ticket)?
        },
        Commands::End { name } => {
            let repo = git::get_repository()?;
            actions::end_workflow(repo, name)?;
        },
        Commands::RepoDebug => {
            let repo = git::get_repository()?;
            actions::print_repo_debug_info(repo)?;
        },
        Commands::Repos => actions::list_repositories(config)?,
    };

    Ok(())
}
