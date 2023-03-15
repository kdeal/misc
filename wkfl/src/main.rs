use std::{env, error::Error};

use anyhow;
use clap::{Parser, Subcommand};
use env_logger;
use git2::Repository;
use log::info;

mod git;

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
    End,
    RepoDebug,
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

fn start_workflow(repo: Repository, name: &String, ticket: &Option<String>) -> anyhow::Result<()> {
    let branch_name = match ticket {
        Some(ticket_key) => format!("kdeal/{ticket_key}_{name}"),
        None => format!("kdeal/{name}"),
    };

    if git::use_worktrees(&repo) {
        info!("Creating worktree named '{name}' on branch '{branch_name}'");
        git::create_worktree(&repo, name, &branch_name)
    } else {
        info!("Creating branch '{branch_name}' and checking it out");
        git::switch_branch(&repo, &branch_name)
    }?;

    Ok(())
}

fn print_repo_debug_info(repo: Repository) {
    info!("worktree: {}", repo.is_worktree());
    info!("bare: {}", repo.is_bare());
    info!("state: {:?}", repo.state());
    info!("path: {:?}", repo.path());
    info!("workdir: {:?}", repo.workdir());
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    match &cli.command {
        Commands::Start { name, ticket } => {
            let repo = git::get_repository()?;
            start_workflow(repo, name, ticket)?
        }
        Commands::End => {
            info!("'end' was used");
        }
        Commands::RepoDebug => {
            let repo = git::get_repository()?;
            print_repo_debug_info(repo);
        }
    };

    Ok(())
}
